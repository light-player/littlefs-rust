//! Directory iteration.
//!
//! Per lfs_dir_read_ (lfs.c:1771).

use crate::block::BlockDevice;
use crate::error::Error;
use crate::info::{FileType, Info};
use crate::superblock::tag;
use crate::trace;

use super::bdcache::BdContext;
use super::gstate;
use super::metadata;

/// True if id has a DELETE tag in dir or any block in its tail chain.
fn entry_deleted_in_tail_chain<B: BlockDevice>(
    ctx: &BdContext<'_, B>,
    dir: &metadata::MdDir,
    id: u16,
    gdisk: Option<&gstate::GState>,
) -> Result<bool, Error> {
    let delete_gtag = (tag::TYPE_DELETE << 20) | ((id as u32) << 10);
    if metadata::get_tag_backwards(dir, 0x7ff0_fc00, delete_gtag, gdisk)?.is_some() {
        return Ok(true);
    }
    if dir.split && !dir.tail_is_null() {
        let tail_dir = metadata::fetch_metadata_pair(ctx, dir.tail)?;
        if entry_deleted_in_tail_chain(ctx, &tail_dir, id, gdisk)? {
            return Ok(true);
        }
    }
    Ok(false)
}

/// True if id has a DELETE tag in any block from head up to (but not including) current.
fn entry_deleted_in_predecessors<B: BlockDevice>(
    ctx: &BdContext<'_, B>,
    head: [u32; 2],
    current_pair: [u32; 2],
    id: u16,
    gdisk: Option<&gstate::GState>,
) -> Result<bool, Error> {
    if gstate::pair_issync(head, current_pair) {
        return Ok(false);
    }
    let mut tail = head;
    loop {
        let dir = metadata::fetch_metadata_pair(ctx, tail)?;
        let delete_gtag = (tag::TYPE_DELETE << 20) | ((id as u32) << 10);
        if metadata::get_tag_backwards(&dir, 0x7ff0_fc00, delete_gtag, gdisk)?.is_some() {
            return Ok(true);
        }
        if dir.tail_is_null() || gstate::pair_issync(dir.tail, current_pair) {
            return Ok(false);
        }
        tail = dir.tail;
    }
}

/// Read next directory entry. Returns 1 on success, 0 at end of directory.
/// Per lfs_dir_read_ (lfs.c:1771).
pub fn dir_read<B: BlockDevice>(
    ctx: &BdContext<'_, B>,
    dir: &mut super::Dir,
    info: &mut Info,
    name_max: u32,
    gdisk: Option<&super::gstate::GState>,
    is_root_dir: bool,
) -> Result<u32, Error> {
    if dir.pos == 0 {
        *info = Info::new(FileType::Dir, 0);
        info.set_name(b".");
        dir.pos = 1;
        return Ok(1);
    }
    if dir.pos == 1 {
        *info = Info::new(FileType::Dir, 0);
        info.set_name(b"..");
        dir.pos = 2;
        return Ok(1);
    }

    loop {
        if dir.id >= dir.mdir.count {
            if !dir.mdir.split {
                return Ok(0);
            }
            dir.mdir = metadata::fetch_metadata_pair(ctx, dir.mdir.tail)?;
            dir.id = 0;
        }

        let deleted_in_tail =
            dir.mdir.split && entry_deleted_in_tail_chain(ctx, &dir.mdir, dir.id, gdisk)?;
        let deleted_in_pred =
            entry_deleted_in_predecessors(ctx, dir.head, dir.mdir.pair, dir.id, gdisk)?;
        if deleted_in_tail || deleted_in_pred {
            trace!("dir_read id={} Noent skip (deleted in chain)", dir.id);
            dir.id += 1;
            continue;
        }

        match metadata::get_entry_info(&dir.mdir, dir.id, name_max, gdisk, is_root_dir) {
            Ok(entry_info) => {
                trace!("dir_read id={} name={:?}", dir.id, entry_info.name().ok());
                *info = entry_info;
                dir.id += 1;
                dir.pos += 1;
                return Ok(1);
            }
            Err(Error::Noent) => {
                trace!("dir_read id={} Noent skip", dir.id);
                dir.id += 1;
                continue;
            }
            Err(e) => return Err(e),
        }
    }
}
