//! Force consistency: demove, deorphan, desuperblock.
//!
//! Per lfs.c lfs_fs_forceconsistency, lfs_fs_demove, lfs_fs_deorphan, lfs_fs_desuperblock.

use crate::block::BlockDevice;
use crate::error::Error;
use crate::superblock::{Superblock, DISK_VERSION};

use super::alloc::Lookahead;
use super::bdcache::BdContext;
use super::commit;
use super::gstate::{self, GState};
use super::metadata;
use super::parent;

const BLOCK_NULL: u32 = 0xffff_ffff;

fn pair_is_null(pair: [u32; 2]) -> bool {
    pair[0] == BLOCK_NULL && pair[1] == BLOCK_NULL
}

fn pair_issync(a: [u32; 2], b: [u32; 2]) -> bool {
    (a[0] == b[0] && a[1] == b[1]) || (a[0] == b[1] && a[1] == b[0])
}

/// Run demove, deorphan, desuperblock. Per lfs_fs_forceconsistency.
pub fn force_consistency<B: BlockDevice>(
    ctx: &BdContext<'_, B>,
    root: &mut [u32; 2],
    gstate: &mut GState,
    gdisk: &mut GState,
    gdelta: &mut GState,
    lookahead: &mut Lookahead,
    name_max: u32,
    block_count: u32,
    file_max: u32,
    attr_max: u32,
    disk_version: u32,
) -> Result<(), Error> {
    demove(
        ctx,
        root,
        gstate,
        gdisk,
        gdelta,
        lookahead,
        name_max,
        disk_version,
    )?;
    deorphan(
        ctx,
        root,
        gstate,
        gdisk,
        gdelta,
        lookahead,
        name_max,
        true,
        disk_version,
    )?;
    desuperblock(
        ctx,
        root,
        gstate,
        gdisk,
        gdelta,
        lookahead,
        name_max,
        block_count,
        file_max,
        attr_max,
        disk_version,
    )?;
    Ok(())
}

/// Complete pending move: delete move id from gdisk.pair. Per lfs_fs_demove.
fn demove<B: BlockDevice>(
    ctx: &BdContext<'_, B>,
    root: &mut [u32; 2],
    gstate: &mut GState,
    gdisk: &mut GState,
    gdelta: &mut GState,
    lookahead: &mut Lookahead,
    name_max: u32,
    disk_version: u32,
) -> Result<(), Error> {
    if !gdisk.hasmove() {
        return Ok(());
    }

    let move_id = (gdisk.tag >> 10) & 0x3ff;
    let pair = gdisk.pair;

    gstate::prepmove(gstate, 0x3ff, [0, 0]);

    let mut movedir = metadata::fetch_metadata_pair(ctx, pair)?;
    commit::dir_orphaningcommit(
        ctx,
        &mut movedir,
        &[commit::CommitAttr::delete(move_id as u16)],
        root,
        lookahead,
        name_max,
        gstate,
        gdisk,
        gdelta,
        false,
        disk_version,
    )?;

    *gdisk = *gstate;
    Ok(())
}

/// Rewrite superblock to root when needssuperblock. Per lfs_fs_desuperblock.
fn desuperblock<B: BlockDevice>(
    ctx: &BdContext<'_, B>,
    root: &mut [u32; 2],
    gstate: &mut GState,
    gdisk: &mut GState,
    gdelta: &mut GState,
    lookahead: &mut Lookahead,
    name_max: u32,
    block_count: u32,
    file_max: u32,
    attr_max: u32,
    disk_version: u32,
) -> Result<(), Error> {
    if !gstate.needssuperblock() {
        return Ok(());
    }

    let superblock = Superblock {
        version: DISK_VERSION,
        block_size: ctx.config.block_size,
        block_count,
        name_max,
        file_max,
        attr_max,
    };
    let sb_bytes: [u8; Superblock::SIZE] = [
        superblock.version.to_le_bytes(),
        superblock.block_size.to_le_bytes(),
        superblock.block_count.to_le_bytes(),
        superblock.name_max.to_le_bytes(),
        superblock.file_max.to_le_bytes(),
        superblock.attr_max.to_le_bytes(),
    ]
    .into_iter()
    .flatten()
    .collect::<alloc::vec::Vec<_>>()
    .try_into()
    .unwrap();

    let mut root_dir = metadata::fetch_metadata_pair(ctx, *root)?;
    commit::dir_orphaningcommit(
        ctx,
        &mut root_dir,
        &[commit::CommitAttr::inline_struct(0, &sb_bytes)],
        root,
        lookahead,
        name_max,
        gstate,
        gdisk,
        gdelta,
        false,
        disk_version,
    )?;

    gstate::prepsuperblock(gstate, false);
    Ok(())
}

/// Repair half-orphans (relocations) and full-orphans (removes/renames). Per lfs_fs_deorphan.
fn deorphan<B: BlockDevice>(
    ctx: &BdContext<'_, B>,
    root: &mut [u32; 2],
    gstate: &mut GState,
    gdisk: &mut GState,
    gdelta: &mut GState,
    lookahead: &mut Lookahead,
    name_max: u32,
    powerloss: bool,
    disk_version: u32,
) -> Result<(), Error> {
    if !gstate.hasorphans() {
        return Ok(());
    }

    let mut pass: u8 = 0;
    while pass < 2 {
        let mut pdir = {
            let mut d = metadata::MdDir::alloc_empty([0, 0], ctx.config.block_size as usize);
            d.split = true;
            d.tail = [0, 1];
            d
        };
        let mut moreorphans = false;

        while !pair_is_null(pdir.tail) {
            let dir = metadata::fetch_metadata_pair(ctx, pdir.tail)?;

            if !pdir.split {
                let parent_opt = parent::fs_parent(ctx, *root, pdir.tail, name_max)?;

                if pass == 0 {
                    if let Some((ref parent_dir, tag_id)) = parent_opt {
                        let bytes = metadata::get_struct(parent_dir, tag_id)?;
                        let expected_pair = [
                            u32::from_le_bytes(bytes[0..4].try_into().unwrap()),
                            u32::from_le_bytes(bytes[4..8].try_into().unwrap()),
                        ];
                        if !pair_issync(expected_pair, pdir.tail) {
                            let mut move_id = 0x3ffu16;
                            if gstate.hasmovehere(pdir.pair) {
                                move_id = (gstate.tag >> 10) as u16 & 0x3ff;
                                gstate::prepmove(gstate, 0x3ff, [0, 0]);
                            }

                            let attrs: alloc::vec::Vec<commit::CommitAttr> = [
                                Some(commit::CommitAttr::soft_tail(expected_pair)),
                                (move_id != 0x3ff).then(|| commit::CommitAttr::delete(move_id)),
                            ]
                            .into_iter()
                            .flatten()
                            .collect();

                            let orphaned = commit::dir_orphaningcommit(
                                ctx,
                                &mut pdir,
                                &attrs,
                                root,
                                lookahead,
                                name_max,
                                gstate,
                                gdisk,
                                gdelta,
                                false,
                                disk_version,
                            )?;
                            if orphaned {
                                moreorphans = true;
                            }
                            continue;
                        }
                    }
                }

                if pass == 1 && parent_opt.is_none() && powerloss {
                    gstate::dir_getgstate(&dir, gdelta)?;

                    let tail_attr = if dir.split {
                        commit::CommitAttr::hard_tail(dir.tail)
                    } else {
                        commit::CommitAttr::soft_tail(dir.tail)
                    };

                    let orphaned = commit::dir_orphaningcommit(
                        ctx,
                        &mut pdir,
                        &[tail_attr],
                        root,
                        lookahead,
                        name_max,
                        gstate,
                        gdisk,
                        gdelta,
                        false,
                        disk_version,
                    )?;
                    if orphaned {
                        moreorphans = true;
                    }
                    continue;
                }
            }

            pdir = dir;
        }

        pass = if moreorphans { 0 } else { pass + 1 };
    }

    gstate::preporphans(gstate, -(gstate.getorphans() as i8))?;
    Ok(())
}
