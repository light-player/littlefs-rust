//! FS parent. Per lfs.c lfs_fs_pred, lfs_fs_parent.

/// Per lfs.c lfs_fs_pred (lines 4796-4833)
///
/// C:
/// ```c
/// static int lfs_fs_pred(lfs_t *lfs,
///         const lfs_block_t pair[2], lfs_mdir_t *pdir) {
///     // iterate over all directory directory entries
///     pdir->tail[0] = 0;
///     pdir->tail[1] = 1;
///     struct lfs_tortoise_t tortoise = {
///         .pair = {LFS_BLOCK_NULL, LFS_BLOCK_NULL},
///         .i = 1,
///         .period = 1,
///     };
///     int err = LFS_ERR_OK;
///     while (!lfs_pair_isnull(pdir->tail)) {
///         err = lfs_tortoise_detectcycles(pdir, &tortoise);
///         if (err < 0) {
///             return LFS_ERR_CORRUPT;
///         }
///
///         if (lfs_pair_cmp(pdir->tail, pair) == 0) {
///             return 0;
///         }
///
///         int err = lfs_dir_fetch(lfs, pdir, pdir->tail);
///         if (err) {
///             return err;
///         }
///     }
///
///     return LFS_ERR_NOENT;
/// }
/// #endif
/// ```
pub fn lfs_fs_pred(
    lfs: *mut crate::fs::Lfs,
    pair: &[crate::types::lfs_block_t; 2],
    pdir: *mut crate::dir::LfsMdir,
) -> i32 {
    use crate::dir::fetch::lfs_dir_fetch;
    use crate::fs::mount::{lfs_tortoise_detectcycles, LfsTortoise};
    use crate::types::LFS_BLOCK_NULL;
    use crate::util::{lfs_pair_cmp, lfs_pair_isnull};

    unsafe {
        (*pdir).tail = [0, 1];
        let mut tortoise = LfsTortoise {
            pair: [LFS_BLOCK_NULL, LFS_BLOCK_NULL],
            i: 1,
            period: 1,
        };
        let mut have_fetched = false;
        #[cfg(feature = "loop_limits")]
        const MAX_PARENT_ITER: u32 = 2048;
        #[cfg(feature = "loop_limits")]
        let mut iter: u32 = 0;

        while !lfs_pair_isnull(&(*pdir).tail) {
            #[cfg(feature = "loop_limits")]
            {
                if iter >= MAX_PARENT_ITER {
                    panic!("loop_limits: MAX_PARENT_ITER ({}) exceeded in lfs_fs_parent", MAX_PARENT_ITER);
                }
                iter += 1;
            }
            let err = lfs_tortoise_detectcycles(pdir, &mut tortoise);
            if err < 0 {
                return crate::error::LFS_ERR_CORRUPT;
            }

            if lfs_pair_cmp(&(*pdir).tail, pair) == 0 {
                if !have_fetched {
                    // Matched before any fetch: tail [0,1] == pair (root).
                    // The root has no predecessor.
                    let err = lfs_dir_fetch(lfs, pdir, &(*pdir).tail);
                    if err != 0 {
                        return err;
                    }
                    if lfs_pair_isnull(&(*pdir).tail) {
                        return crate::error::LFS_ERR_NOENT;
                    }
                }
                return 0;
            }

            let err = lfs_dir_fetch(lfs, pdir, &(*pdir).tail);
            if err != 0 {
                return err;
            }
            have_fetched = true;
        }

        crate::error::LFS_ERR_NOENT
    }
}

/// C: lfs.c:4835-4853
#[repr(C)]
pub struct LfsFsParentMatch {
    pub lfs: *mut crate::fs::Lfs,
    pub pair: [crate::types::lfs_block_t; 2],
}

// Per lfs.c enum: LFS_CMP_EQ=0, LFS_CMP_LT=1, LFS_CMP_GT=2
const LFS_CMP_EQ: i32 = 0;
const LFS_CMP_LT: i32 = 1;

/// Per lfs.c lfs_fs_parent_match (lines 4835-4853)
///
/// C:
/// ```c
/// static int lfs_fs_parent_match(void *data,
///         lfs_tag_t tag, const void *buffer) {
///     struct lfs_fs_parent_match *find = data;
///     lfs_t *lfs = find->lfs;
///     const struct lfs_diskoff *disk = buffer;
///     (void)tag;
///     lfs_block_t child[2];
///     int err = lfs_bd_read(lfs, ...);
///     lfs_pair_fromle32(child);
///     return (lfs_pair_cmp(child, find->pair) == 0) ? LFS_CMP_EQ : LFS_CMP_LT;
/// }
/// ```
pub unsafe extern "C" fn lfs_fs_parent_match(
    data: *mut core::ffi::c_void,
    _tag: crate::types::lfs_tag_t,
    buffer: *const core::ffi::c_void,
) -> i32 {
    use crate::bd::bd::lfs_bd_read;
    use crate::tag::lfs_diskoff;
    use crate::util::{lfs_pair_cmp, lfs_pair_fromle32};

    if data.is_null() || buffer.is_null() {
        return LFS_CMP_LT;
    }
    let find = &*(data as *const LfsFsParentMatch);
    let disk = &*(buffer as *const lfs_diskoff);

    let mut child: [crate::types::lfs_block_t; 2] = [0, 0];
    let err = lfs_bd_read(
        find.lfs,
        core::ptr::null(),
        &mut (*find.lfs).rcache,
        (*find.lfs).cfg.as_ref().expect("cfg").block_size,
        disk.block,
        disk.off,
        child.as_mut_ptr() as *mut u8,
        8,
    );
    if err != 0 {
        return err;
    }
    lfs_pair_fromle32(&mut child);
    if lfs_pair_cmp(&child, &find.pair) == 0 {
        LFS_CMP_EQ
    } else {
        LFS_CMP_LT
    }
}

/// Per lfs.c lfs_fs_parent (lines 4856-4892)
///
/// C:
/// ```c
/// static lfs_stag_t lfs_fs_parent(lfs_t *lfs, const lfs_block_t pair[2],
///         lfs_mdir_t *parent) {
///     // use fetchmatch with callback to find pairs
///     parent->tail[0] = 0;
///     parent->tail[1] = 1;
///     struct lfs_tortoise_t tortoise = {
///         .pair = {LFS_BLOCK_NULL, LFS_BLOCK_NULL},
///         .i = 1,
///         .period = 1,
///     };
///     int err = LFS_ERR_OK;
///     while (!lfs_pair_isnull(parent->tail)) {
///         err = lfs_tortoise_detectcycles(parent, &tortoise);
///         if (err < 0) {
///             return err;
///         }
///
///         lfs_stag_t tag = lfs_dir_fetchmatch(lfs, parent, parent->tail,
///                 LFS_MKTAG(0x7ff, 0, 0x3ff),
///                 LFS_MKTAG(LFS_TYPE_DIRSTRUCT, 0, 8),
///                 NULL,
///                 lfs_fs_parent_match, &(struct lfs_fs_parent_match){
///                     lfs, {pair[0], pair[1]}});
///         if (tag && tag != LFS_ERR_NOENT) {
///             return tag;
///         }
///     }
///
///     return LFS_ERR_NOENT;
/// }
/// #endif
/// ```
pub fn lfs_fs_parent(
    lfs: *mut crate::fs::Lfs,
    pair: *const [crate::types::lfs_block_t; 2],
    parent: *mut crate::dir::LfsMdir,
) -> crate::types::lfs_stag_t {
    use crate::dir::fetch::lfs_dir_fetchmatch;
    use crate::fs::mount::{lfs_tortoise_detectcycles, LfsTortoise};
    use crate::lfs_type::lfs_type::LFS_TYPE_DIRSTRUCT;
    use crate::tag::lfs_mktag;
    use crate::types::{lfs_block_t, LFS_BLOCK_NULL};
    use crate::util::lfs_pair_isnull;

    unsafe {
        (*parent).tail = [0, 1];
        let mut tortoise = LfsTortoise {
            pair: [LFS_BLOCK_NULL, LFS_BLOCK_NULL],
            i: 1,
            period: 1,
        };
        #[cfg(feature = "loop_limits")]
        const MAX_PARENT_ITER: u32 = 2048;
        #[cfg(feature = "loop_limits")]
        let mut iter: u32 = 0;

        while !lfs_pair_isnull(&(*parent).tail) {
            #[cfg(feature = "loop_limits")]
            {
                if iter >= MAX_PARENT_ITER {
                    panic!("loop_limits: MAX_PARENT_ITER ({}) exceeded in lfs_fs_parent (parent)", MAX_PARENT_ITER);
                }
                iter += 1;
            }
            let err = lfs_tortoise_detectcycles(parent, &mut tortoise);
            if err < 0 {
                return err;
            }

            let find_match = LfsFsParentMatch {
                lfs,
                pair: [(*pair)[0], (*pair)[1]],
            };
            let tag = lfs_dir_fetchmatch(
                lfs as *mut _ as *const core::ffi::c_void,
                parent,
                &(*parent).tail as *const _,
                lfs_mktag(0x7ff, 0, 0x3ff),
                lfs_mktag(LFS_TYPE_DIRSTRUCT, 0, 8),
                core::ptr::null_mut(),
                Some(lfs_fs_parent_match),
                &find_match as *const _ as *mut core::ffi::c_void,
            );

            if tag != 0 && tag != crate::error::LFS_ERR_NOENT {
                return tag;
            }
        }

        crate::error::LFS_ERR_NOENT
    }
}
