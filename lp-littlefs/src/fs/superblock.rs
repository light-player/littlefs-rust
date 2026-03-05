//! Superblock and consistency. Per lfs.c lfs_fs_prepsuperblock, lfs_fs_deorphan, etc.

use crate::types::lfs_block_t;

/// Per lfs.c lfs_fs_prepsuperblock (lines 4888-4892)
///
/// C:
/// ```c
/// static void lfs_fs_prepsuperblock(lfs_t *lfs, bool needssuperblock) {
///     lfs->gstate.tag = (lfs->gstate.tag & ~LFS_MKTAG(0, 0, 0x200))
///             | (uint32_t)needssuperblock << 9;
/// }
/// ```
pub fn lfs_fs_prepsuperblock(lfs: *mut super::lfs::Lfs, needssuperblock: bool) {
    use crate::tag::lfs_mktag;
    unsafe {
        let lfs = &mut *lfs;
        lfs.gstate.tag =
            (lfs.gstate.tag & !lfs_mktag(0, 0, 0x200)) | ((needssuperblock as u32) << 9);
    }
}

/// Translation docs: Prepend orphan count delta to gstate before a commit that may create orphans.
/// Assertions ensure we don't overflow the 9-bit orphan count.
///
/// C: lfs.c:4894-4904
pub fn lfs_fs_preporphans(lfs: *mut super::lfs::Lfs, orphans: i8) -> i32 {
    use crate::lfs_gstate::lfs_gstate_hasorphans;
    use crate::tag::{lfs_mktag, lfs_tag_size};

    unsafe {
        let lfs = &mut *lfs;
        let tag_size = lfs_tag_size(lfs.gstate.tag);
        crate::lfs_assert!(tag_size > 0x000 || orphans >= 0);
        crate::lfs_assert!(tag_size < 0x1ff || orphans <= 0);
        lfs.gstate.tag = lfs.gstate.tag.wrapping_add(orphans as u32);
        lfs.gstate.tag = (lfs.gstate.tag & !lfs_mktag(0x800, 0, 0))
            | ((lfs_gstate_hasorphans(&lfs.gstate) as u32) << 31);
        0
    }
}

/// Translation docs: Record a pending move (or clear it when id=0x3ff) in gstate.
///
/// C: lfs.c:4906-4914
pub fn lfs_fs_prepmove(lfs: *mut super::lfs::Lfs, id: u16, pair: *const [lfs_block_t; 2]) {
    use crate::lfs_type::lfs_type::LFS_TYPE_DELETE;
    use crate::tag::lfs_mktag;

    unsafe {
        let lfs = &mut *lfs;
        lfs.gstate.tag = (lfs.gstate.tag & !lfs_mktag(0x7ff, 0x3ff, 0))
            | if id != 0x3ff {
                lfs_mktag(LFS_TYPE_DELETE, id as u32, 0)
            } else {
                0
            };
        if id != 0x3ff && !pair.is_null() {
            lfs.gstate.pair[0] = (*pair)[0];
            lfs.gstate.pair[1] = (*pair)[1];
        } else {
            lfs.gstate.pair[0] = 0;
            lfs.gstate.pair[1] = 0;
        }
    }
}

/// Translation docs: Rewrite superblock when needssuperblock is set (older minor version on disk).
///
/// C: lfs.c:4916-4953
pub fn lfs_fs_desuperblock(lfs: *mut super::lfs::Lfs) -> i32 {
    use crate::dir::commit::lfs_dir_commit;
    use crate::dir::fetch::lfs_dir_fetch;
    use crate::lfs_gstate::lfs_gstate_needssuperblock;
    use crate::lfs_superblock::{lfs_superblock_tole32, LfsSuperblock};
    use crate::lfs_type::lfs_type::LFS_TYPE_INLINESTRUCT;
    use crate::tag::lfs_mktag;
    use crate::types::LFS_DISK_VERSION;

    unsafe {
        if !lfs_gstate_needssuperblock(&(*lfs).gstate) {
            return 0;
        }

        let mut root = core::mem::zeroed();
        let err = lfs_dir_fetch(lfs, &mut root, &(*lfs).root);
        if err != 0 {
            return err;
        }

        // write a new superblock
        let mut superblock = LfsSuperblock {
            version: LFS_DISK_VERSION,
            block_size: (*lfs).cfg.as_ref().expect("cfg").block_size,
            block_count: (*lfs).block_count,
            name_max: (*lfs).name_max,
            file_max: (*lfs).file_max,
            attr_max: (*lfs).attr_max,
        };
        lfs_superblock_tole32(&mut superblock);

        let attrs = [crate::tag::lfs_mattr {
            tag: lfs_mktag(
                LFS_TYPE_INLINESTRUCT,
                0,
                core::mem::size_of::<LfsSuperblock>() as u32,
            ),
            buffer: &superblock as *const _ as *const _,
        }];
        let err = lfs_dir_commit(lfs, &mut root, attrs.as_ptr() as *const _, 1);
        if err != 0 {
            return err;
        }

        lfs_fs_prepsuperblock(lfs, false);
        0
    }
}

/// Per lfs.c lfs_fs_demove (lines 4955-4989)
///
/// C:
/// ```c
/// static int lfs_fs_demove(lfs_t *lfs) {
///     if (!lfs_gstate_hasmove(&lfs->gdisk)) {
///         return 0;
///     }
///
///     // Fix bad moves
///     LFS_DEBUG("Fixing move {0x%"PRIx32", 0x%"PRIx32"} 0x%"PRIx16,
///             lfs->gdisk.pair[0],
///             lfs->gdisk.pair[1],
///             lfs_tag_id(lfs->gdisk.tag));
///
///     // no other gstate is supported at this time, so if we found something else
///     // something most likely went wrong in gstate calculation
///     LFS_ASSERT(lfs_tag_type3(lfs->gdisk.tag) == LFS_TYPE_DELETE);
///
///     // fetch and delete the moved entry
///     lfs_mdir_t movedir;
///     int err = lfs_dir_fetch(lfs, &movedir, lfs->gdisk.pair);
///     if (err) {
///         return err;
///     }
///
///     // prep gstate and delete move id
///     uint16_t moveid = lfs_tag_id(lfs->gdisk.tag);
///     lfs_fs_prepmove(lfs, 0x3ff, NULL);
///     err = lfs_dir_commit(lfs, &movedir, LFS_MKATTRS(
///             {LFS_MKTAG(LFS_TYPE_DELETE, moveid, 0), NULL}));
///     if (err) {
///         return err;
///     }
///
///     return 0;
/// }
/// #endif
/// ```
pub fn lfs_fs_demove(lfs: *mut super::lfs::Lfs) -> i32 {
    use crate::dir::commit::lfs_dir_commit;
    use crate::dir::fetch::lfs_dir_fetch;
    use crate::lfs_gstate::lfs_gstate_hasmove;
    use crate::lfs_type::lfs_type::LFS_TYPE_DELETE;
    use crate::tag::{lfs_mktag, lfs_tag_id, lfs_tag_type3};

    unsafe {
        if !lfs_gstate_hasmove(&(*lfs).gdisk) {
            return 0;
        }

        crate::lfs_assert!(u32::from(lfs_tag_type3((*lfs).gdisk.tag)) == LFS_TYPE_DELETE);

        let mut movedir = core::mem::zeroed();
        let err = lfs_dir_fetch(lfs, &mut movedir, &(*lfs).gdisk.pair);
        if err != 0 {
            return err;
        }

        let moveid = lfs_tag_id((*lfs).gdisk.tag);
        lfs_fs_prepmove(lfs, 0x3ff, core::ptr::null());

        let attrs = [crate::tag::lfs_mattr {
            tag: lfs_mktag(LFS_TYPE_DELETE, moveid as u32, 0),
            buffer: core::ptr::null(),
        }];
        lfs_dir_commit(lfs, &mut movedir, attrs.as_ptr() as *const _, 1)
    }
}

/// Per lfs.c lfs_fs_deorphan (lines 4991-5120)
///
/// C:
/// ```c
/// static int lfs_fs_deorphan(lfs_t *lfs, bool powerloss) {
///     if (!lfs_gstate_hasorphans(&lfs->gstate)) {
///         return 0;
///     }
///
///     // Check for orphans in two separate passes:
///     // - 1 for half-orphans (relocations)
///     // - 2 for full-orphans (removes/renames)
///     //
///     // Two separate passes are needed as half-orphans can contain outdated
///     // references to full-orphans, effectively hiding them from the deorphan
///     // search.
///     //
///     int pass = 0;
///     while (pass < 2) {
///         // Fix any orphans
///         lfs_mdir_t pdir = {.split = true, .tail = {0, 1}};
///         lfs_mdir_t dir;
///         bool moreorphans = false;
///
///         // iterate over all directory directory entries
///         while (!lfs_pair_isnull(pdir.tail)) {
///             int err = lfs_dir_fetch(lfs, &dir, pdir.tail);
///             if (err) {
///                 return err;
///             }
///
///             // check head blocks for orphans
///             if (!pdir.split) {
///                 // check if we have a parent
///                 lfs_mdir_t parent;
///                 lfs_stag_t tag = lfs_fs_parent(lfs, pdir.tail, &parent);
///                 if (tag < 0 && tag != LFS_ERR_NOENT) {
///                     return tag;
///                 }
///
///                 if (pass == 0 && tag != LFS_ERR_NOENT) {
///                     lfs_block_t pair[2];
///                     lfs_stag_t state = lfs_dir_get(lfs, &parent,
///                             LFS_MKTAG(0x7ff, 0x3ff, 0), tag, pair);
///                     if (state < 0) {
///                         return state;
///                     }
///                     lfs_pair_fromle32(pair);
///
///                     if (!lfs_pair_issync(pair, pdir.tail)) {
///                         // we have desynced
///                         LFS_DEBUG("Fixing half-orphan "
///                                 "{0x%"PRIx32", 0x%"PRIx32"} "
///                                 "-> {0x%"PRIx32", 0x%"PRIx32"}",
///                                 pdir.tail[0], pdir.tail[1], pair[0], pair[1]);
///
///                         // fix pending move in this pair? this looks like an
///                         // optimization but is in fact _required_ since
///                         // relocating may outdate the move.
///                         uint16_t moveid = 0x3ff;
///                         if (lfs_gstate_hasmovehere(&lfs->gstate, pdir.pair)) {
///                             moveid = lfs_tag_id(lfs->gstate.tag);
///                             LFS_DEBUG("Fixing move while fixing orphans "
///                                     "{0x%"PRIx32", 0x%"PRIx32"} 0x%"PRIx16"\n",
///                                     pdir.pair[0], pdir.pair[1], moveid);
///                             lfs_fs_prepmove(lfs, 0x3ff, NULL);
///                         }
///
///                         lfs_pair_tole32(pair);
///                         state = lfs_dir_orphaningcommit(lfs, &pdir, LFS_MKATTRS(
///                                 {LFS_MKTAG_IF(moveid != 0x3ff,
///                                     LFS_TYPE_DELETE, moveid, 0), NULL},
///                                 {LFS_MKTAG(LFS_TYPE_SOFTTAIL, 0x3ff, 8),
///                                     pair}));
///                         lfs_pair_fromle32(pair);
///                         if (state < 0) {
///                             return state;
///                         }
///
///                         // did our commit create more orphans?
///                         if (state == LFS_OK_ORPHANED) {
///                             moreorphans = true;
///                         }
///
///                         // refetch tail
///                         continue;
///                     }
///                 }
///
///                 // note we only check for full orphans if we may have had a
///                 // power-loss, otherwise orphans are created intentionally
///                 // during operations such as lfs_mkdir
///                 if (pass == 1 && tag == LFS_ERR_NOENT && powerloss) {
///                     // we are an orphan
///                     LFS_DEBUG("Fixing orphan {0x%"PRIx32", 0x%"PRIx32"}",
///                             pdir.tail[0], pdir.tail[1]);
///
///                     // steal state
///                     err = lfs_dir_getgstate(lfs, &dir, &lfs->gdelta);
///                     if (err) {
///                         return err;
///                     }
///
///                     // steal tail
///                     lfs_pair_tole32(dir.tail);
///                     int state = lfs_dir_orphaningcommit(lfs, &pdir, LFS_MKATTRS(
///                             {LFS_MKTAG(LFS_TYPE_TAIL + dir.split, 0x3ff, 8),
///                                 dir.tail}));
///                     lfs_pair_fromle32(dir.tail);
///                     if (state < 0) {
///                         return state;
///                     }
///
///                     // did our commit create more orphans?
///                     if (state == LFS_OK_ORPHANED) {
///                         moreorphans = true;
///                     }
///
///                     // refetch tail
///                     continue;
///                 }
///             }
///
///             pdir = dir;
///         }
///
///         pass = moreorphans ? 0 : pass+1;
///     }
///
///     // mark orphans as fixed
///     return lfs_fs_preporphans(lfs, -lfs_gstate_getorphans(&lfs->gstate));
/// }
/// #endif
/// ```
/// Translation docs: Fix half-orphans (relocations) and full-orphans (removes/renames).
/// Two passes: pass 0 for half-orphans, pass 1 for full-orphans.
///
/// C: lfs.c:4991-5120
pub fn lfs_fs_deorphan(lfs: *mut super::lfs::Lfs, powerloss: bool) -> i32 {
    use crate::dir::commit::{lfs_dir_commit, lfs_dir_orphaningcommit};
    use crate::dir::fetch::lfs_dir_fetch;
    use crate::dir::traverse::lfs_dir_get;
    use crate::dir::LfsMdir;
    use crate::error::{LFS_ERR_NOENT, LFS_OK_ORPHANED};
    use crate::fs::parent::lfs_fs_parent;
    use crate::lfs_gstate::{lfs_gstate_getorphans, lfs_gstate_hasmovehere};
    use crate::lfs_type::lfs_type::{LFS_TYPE_SOFTTAIL, LFS_TYPE_TAIL};
    use crate::tag::{lfs_mktag, lfs_mktag_if, lfs_tag_id};
    use crate::types::LFS_BLOCK_NULL;
    use crate::util::{lfs_pair_fromle32, lfs_pair_issync, lfs_pair_tole32};

    unsafe {
        if !crate::lfs_gstate::lfs_gstate_hasorphans(&(*lfs).gstate) {
            return 0;
        }

        let mut pass: i32 = 0;
        while pass < 2 {
            let mut pdir = LfsMdir {
                pair: [0, 0],
                rev: 0,
                off: 0,
                etag: 0,
                count: 0,
                erased: false,
                split: true,
                tail: [0, 1],
            };
            let mut dir = core::mem::zeroed::<LfsMdir>();
            let mut moreorphans = false;
            #[cfg(feature = "loop_limits")]
            let mut iter: u32 = 0;
            #[cfg(feature = "loop_limits")]
            const MAX_DEORPHAN_ITER: u32 = 512;

            while !crate::util::lfs_pair_isnull(&pdir.tail) {
                #[cfg(feature = "loop_limits")]
                {
                    if iter >= MAX_DEORPHAN_ITER {
                        return crate::error::LFS_ERR_CORRUPT;
                    }
                    iter += 1;
                }
                let err = lfs_dir_fetch(lfs, &mut dir, &pdir.tail);
                if err != 0 {
                    return err;
                }

                if !pdir.split {
                    let mut parent = core::mem::zeroed();
                    let tag = lfs_fs_parent(lfs, &pdir.tail, &mut parent);
                    if tag < 0 && tag != LFS_ERR_NOENT {
                        return tag;
                    }

                    if pass == 0 && tag != LFS_ERR_NOENT {
                        let mut pair: [crate::types::lfs_block_t; 2] = [0, 0];
                        let state = lfs_dir_get(
                            lfs,
                            &parent,
                            lfs_mktag(0x7ff, 0x3ff, 0),
                            tag as u32,
                            pair.as_mut_ptr() as *mut core::ffi::c_void,
                        );
                        if state < 0 {
                            return state;
                        }
                        lfs_pair_fromle32(&mut pair);

                        if !lfs_pair_issync(&pair, &pdir.tail) {
                            let mut moveid: u16 = 0x3ff;
                            if lfs_gstate_hasmovehere(&(*lfs).gstate, &pdir.pair) {
                                moveid = lfs_tag_id((*lfs).gstate.tag);
                                lfs_fs_prepmove(lfs, 0x3ff, core::ptr::null());
                            }

                            lfs_pair_tole32(&mut pair);
                            let attrs = [
                                crate::tag::lfs_mattr {
                                    tag: lfs_mktag_if(
                                        moveid != 0x3ff,
                                        crate::lfs_type::lfs_type::LFS_TYPE_DELETE,
                                        moveid as u32,
                                        0,
                                    ),
                                    buffer: core::ptr::null(),
                                },
                                crate::tag::lfs_mattr {
                                    tag: lfs_mktag(LFS_TYPE_SOFTTAIL, 0x3ff, 8),
                                    buffer: pair.as_ptr() as *const core::ffi::c_void,
                                },
                            ];
                            let state = lfs_dir_orphaningcommit(
                                lfs,
                                &mut pdir,
                                attrs.as_ptr() as *const _,
                                2,
                            );
                            lfs_pair_fromle32(&mut pair);
                            if state < 0 {
                                return state;
                            }
                            if state == LFS_OK_ORPHANED {
                                moreorphans = true;
                            }
                            continue;
                        }
                    }

                    if pass == 1 && tag == LFS_ERR_NOENT && powerloss {
                        let err =
                            crate::dir::fetch::lfs_dir_getgstate(lfs, &dir, &mut (*lfs).gdelta);
                        if err != 0 {
                            return err;
                        }

                        let mut dir_tail = dir.tail;
                        lfs_pair_tole32(&mut dir_tail);
                        let attrs = [crate::tag::lfs_mattr {
                            tag: lfs_mktag(LFS_TYPE_TAIL + if dir.split { 1 } else { 0 }, 0x3ff, 8),
                            buffer: dir_tail.as_ptr() as *const core::ffi::c_void,
                        }];
                        let state =
                            lfs_dir_orphaningcommit(lfs, &mut pdir, attrs.as_ptr() as *const _, 1);
                        lfs_pair_fromle32(&mut dir_tail);
                        if state < 0 {
                            return state;
                        }
                        if state == LFS_OK_ORPHANED {
                            moreorphans = true;
                        }
                        continue;
                    }
                }

                pdir = dir;
            }

            pass = if moreorphans { 0 } else { pass + 1 };
        }

        let orphans = lfs_gstate_getorphans(&(*lfs).gstate);
        lfs_fs_preporphans(lfs, -(orphans as i8))
    }
}

/// Translation docs: Ensure filesystem consistency before mutations. Calls desuperblock,
/// demove, and deorphan in sequence.
///
/// C: lfs.c:5122-5140
pub fn lfs_fs_forceconsistency(lfs: *mut super::lfs::Lfs) -> i32 {
    let err = lfs_fs_desuperblock(lfs);
    if err != 0 {
        return err;
    }
    let err = lfs_fs_demove(lfs);
    if err != 0 {
        return err;
    }
    lfs_fs_deorphan(lfs, true)
}
