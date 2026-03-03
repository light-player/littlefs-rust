//! Mount implementation.
//!
//! Traverses metadata tail chain from [0,1], finds superblock, accumulates gstate.
//! Per lfs_mount_ (lfs.c:4482).

use core::cell::RefCell;

use crate::block::BlockDevice;
use crate::config::Config;
use crate::error::Error;
use crate::superblock::{Superblock, DISK_VERSION};

use super::alloc::Lookahead;
use super::bdcache::{self, BdContext, ProgCache, ReadCache};
use super::gstate::{self, GState};
use super::metadata;

const BLOCK_NULL: u32 = 0xffff_ffff;

/// Mount state to store in LittleFs.
pub(crate) struct MountState {
    pub root: [u32; 2],
    pub block_size: u32,
    pub block_count: u32,
    pub name_max: u32,
    pub file_max: u32,
    pub attr_max: u32,
    pub inline_max: u32,
    pub disk_version: u32,
    pub lookahead: Lookahead,
    /// Accumulated global state (orphans, moves). Per lfs.gstate.
    pub gstate: GState,
    /// Last committed gstate on disk. Per lfs.gdisk.
    pub gdisk: GState,
    /// Pending gstate delta. Per lfs.gdelta.
    pub gdelta: GState,
    pub(crate) rcache: RefCell<ReadCache>,
    pub(crate) pcache: RefCell<ProgCache>,
}

fn pair_is_null(pair: [u32; 2]) -> bool {
    pair[0] == BLOCK_NULL && pair[1] == BLOCK_NULL
}

pub fn mount<B: BlockDevice>(bd: &B, config: &Config) -> Result<MountState, Error> {
    use crate::trace;
    trace!("mount: sync block device");
    bd.sync()?;

    let rcache = RefCell::new(bdcache::new_read_cache(config)?);
    let pcache = RefCell::new(bdcache::new_prog_cache(config)?);
    let ctx = BdContext::new(bd, config, &rcache, &pcache);

    let mut tail = [0u32, 1u32];
    let mut gstate = GState::zero();
    let mut root = [0u32, 1u32];
    let mut superblock: Option<Superblock> = None;
    let limit = config.block_count.max(1) as usize;
    let mut seed = 0u32;

    for _ in 0..limit {
        if pair_is_null(tail) {
            break;
        }

        trace!("mount: fetch_metadata_pair tail={:?}", tail);
        let dir = metadata::fetch_metadata_pair_ext(&ctx, tail, Some(&mut seed), None)?;

        if let Some(sb) = metadata::get_superblock_from_dir(&dir) {
            root = dir.pair;
            superblock = Some(sb);
        }

        gstate::dir_getgstate(&dir, &mut gstate)?;
        tail = dir.tail;
    }

    let sb = superblock.ok_or_else(|| {
        trace!("mount Corrupt: no superblock found in tail chain");
        Error::Corrupt
    })?;

    if sb.block_size != config.block_size {
        trace!(
            "mount Corrupt: block_size mismatch sb={} config={}",
            sb.block_size,
            config.block_size
        );
        return Err(Error::Corrupt);
    }
    if config.block_count != 0 && sb.block_count != config.block_count {
        trace!(
            "mount Corrupt: block_count mismatch sb={} config={}",
            sb.block_count,
            config.block_count
        );
        return Err(Error::Corrupt);
    }

    let major = (0xffff & (sb.version >> 16)) as u16;
    let minor = (0xffff & sb.version) as u16;
    let disk_major = (0xffff & (DISK_VERSION >> 16)) as u16;
    let disk_minor = (0xffff & DISK_VERSION) as u16;
    if major != disk_major || minor > disk_minor {
        trace!(
            "mount Corrupt: version mismatch major={} minor={} disk={}/{}",
            major,
            minor,
            disk_major,
            disk_minor
        );
        return Err(Error::Corrupt);
    }

    let name_max = sb.name_max;
    let file_max = sb.file_max;
    let attr_max = sb.attr_max;
    let disk_version = sb.version;

    let mut lookahead = Lookahead::new(config);
    lookahead.start = seed.wrapping_rem(sb.block_count);
    lookahead.alloc_drop(sb.block_count);

    let inline_max = match config.inline_max {
        0 => config.cache_size.min(attr_max),
        -1 => 0,
        n if n > 0 => n as u32,
        _ => config.cache_size.min(attr_max),
    };

    gstate::ensure_valid(&mut gstate);
    let gdisk = gstate;

    trace!("mount done root={:?}", root);
    Ok(MountState {
        root,
        block_size: sb.block_size,
        block_count: sb.block_count,
        name_max,
        file_max,
        attr_max,
        inline_max,
        disk_version,
        lookahead,
        gstate,
        gdisk,
        gdelta: GState::zero(),
        rcache,
        pcache,
    })
}
