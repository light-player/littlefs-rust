//! Consistency. Per lfs.c lfs_fs_mkconsistent_, lfs_fs_gc_.

use crate::dir::fetch::lfs_dir_fetch;
use crate::dir::LfsMdir;

/// Translation docs: Deorphan, complete moves, persist gstate. If pending gstate
/// (delta != 0), fetches root and commits empty to write MOVESTATE.
///
/// Per lfs.c lfs_fs_mkconsistent_ (lines 5143-5170)
///
/// C:
/// ```c
/// static int lfs_fs_mkconsistent_(lfs_t *lfs) {
///     // lfs_fs_forceconsistency does most of the work here
///     int err = lfs_fs_forceconsistency(lfs);
///     if (err) {
///         return err;
///     }
///
///     // do we have any pending gstate?
///     lfs_gstate_t delta = {0};
///     lfs_gstate_xor(&delta, &lfs->gdisk);
///     lfs_gstate_xor(&delta, &lfs->gstate);
///     if (!lfs_gstate_iszero(&delta)) {
///         // lfs_dir_commit will implicitly write out any pending gstate
///         lfs_mdir_t root;
///         err = lfs_dir_fetch(lfs, &root, lfs->root);
///         if (err) {
///             return err;
///         }
///
///         err = lfs_dir_commit(lfs, &root, NULL, 0);
///         if (err) {
///             return err;
///         }
///     }
///
///     return 0;
/// }
/// #endif
/// ```
pub fn lfs_fs_mkconsistent_(lfs: *mut super::lfs::Lfs) -> i32 {
    use crate::dir::commit::lfs_dir_commit;
    use crate::lfs_gstate::{lfs_gstate_iszero, lfs_gstate_xor};

    let err = super::superblock::lfs_fs_forceconsistency(lfs);
    if err != 0 {
        return err;
    }

    unsafe {
        let mut delta = crate::lfs_gstate::LfsGstate {
            tag: 0,
            pair: [0, 0],
        };
        lfs_gstate_xor(&mut delta, &(*lfs).gdisk);
        lfs_gstate_xor(&mut delta, &(*lfs).gstate);

        if !lfs_gstate_iszero(&delta) {
            let mut root = core::mem::zeroed::<LfsMdir>();
            let err = lfs_dir_fetch(lfs, &mut root, &(*lfs).root);
            if err != 0 {
                return err;
            }

            let err = lfs_dir_commit(lfs, &mut root, core::ptr::null(), 0);
            if err != 0 {
                return err;
            }
        }
    }

    0
}

/// Per lfs.c lfs_fs_gc_ (lines 5191-5240)
///
/// C:
/// ```c
/// static int lfs_fs_gc_(lfs_t *lfs) {
///     // force consistency, even if we're not necessarily going to write,
///     // because this function is supposed to take care of janitorial work
///     // isn't it?
///     int err = lfs_fs_forceconsistency(lfs);
///     if (err) {
///         return err;
///     }
///
///     // try to compact metadata pairs, note we can't really accomplish
///     // anything if compact_thresh doesn't at least leave a prog_size
///     // available
///     if (lfs->cfg->compact_thresh
///             < lfs->cfg->block_size - lfs->cfg->prog_size) {
///         // iterate over all mdirs
///         lfs_mdir_t mdir = {.tail = {0, 1}};
///         while (!lfs_pair_isnull(mdir.tail)) {
///             err = lfs_dir_fetch(lfs, &mdir, mdir.tail);
///             if (err) {
///                 return err;
///             }
///
///             // not erased? exceeds our compaction threshold?
///             if (!mdir.erased || ((lfs->cfg->compact_thresh == 0)
///                     ? mdir.off > lfs->cfg->block_size - lfs->cfg->block_size/8
///                     : mdir.off > lfs->cfg->compact_thresh)) {
///                 // the easiest way to trigger a compaction is to mark
///                 // the mdir as unerased and add an empty commit
///                 mdir.erased = false;
///                 err = lfs_dir_commit(lfs, &mdir, NULL, 0);
///                 if (err) {
///                     return err;
///                 }
///             }
///         }
///     }
///
///     // try to populate the lookahead buffer, unless it's already full
///     if (lfs->lookahead.size < lfs_min(
///             8 * lfs->cfg->lookahead_size,
///             lfs->block_count)) {
///         err = lfs_alloc_scan(lfs);
///         if (err) {
///             return err;
///         }
///     }
///
///     return 0;
/// }
/// #endif
/// ```
pub fn lfs_fs_gc_(lfs: *mut super::lfs::Lfs) -> i32 {
    use crate::block_alloc::alloc::lfs_alloc_scan;
    use crate::dir::commit::lfs_dir_commit;
    use crate::util::{lfs_min, lfs_pair_isnull};

    crate::lfs_trace!("lfs_fs_gc: start");
    let err = super::superblock::lfs_fs_forceconsistency(lfs);
    crate::lfs_trace!("lfs_fs_gc: after forceconsistency err={}", err);
    if err != 0 {
        return err;
    }

    unsafe {
        let lfs_ref = &*lfs;
        let cfg = lfs_ref.cfg.as_ref().expect("cfg");
        let block_size = cfg.block_size;
        let prog_size = cfg.prog_size;
        let compact_thresh = cfg.compact_thresh;

        if compact_thresh < block_size.saturating_sub(prog_size) {
            crate::lfs_trace!("lfs_fs_gc: compact loop start");
            let mut mdir = LfsMdir {
                pair: [0, 0],
                rev: 0,
                off: 0,
                etag: 0,
                count: 0,
                erased: false,
                split: false,
                tail: [0, 1],
            };
            #[cfg(feature = "loop_limits")]
            const MAX_GC_COMPACT_ITER: u32 = 2048;
            #[cfg(feature = "loop_limits")]
            let mut iter: u32 = 0;

            while !lfs_pair_isnull(&mdir.tail) {
                #[cfg(feature = "loop_limits")]
                {
                    if iter >= MAX_GC_COMPACT_ITER {
                        panic!("loop_limits: MAX_GC_COMPACT_ITER ({}) exceeded", MAX_GC_COMPACT_ITER);
                    }
                    iter += 1;
                }
                let err = lfs_dir_fetch(lfs, &mut mdir, &mdir.tail);
                if err != 0 {
                    return err;
                }

                let should_compact = !mdir.erased
                    || if compact_thresh == 0 {
                        mdir.off > block_size - block_size / 8
                    } else {
                        mdir.off > compact_thresh
                    };

                if should_compact {
                    let mdir_ref = &mut mdir;
                    mdir_ref.erased = false;
                    let err = lfs_dir_commit(lfs, mdir_ref, core::ptr::null(), 0);
                    if err != 0 {
                        return err;
                    }
                }
            }
        }

        let lfs_ref = &*lfs;
        let lookahead_size = cfg.lookahead_size;
        let block_count = lfs_ref.block_count;
        if lfs_ref.lookahead.size < lfs_min(8 * lookahead_size, block_count) {
            crate::lfs_trace!("lfs_fs_gc: alloc_scan start");
            let err = lfs_alloc_scan(lfs);
            crate::lfs_trace!("lfs_fs_gc: alloc_scan done err={}", err);
            if err != 0 {
                return err;
            }
        }
    }

    0
}
