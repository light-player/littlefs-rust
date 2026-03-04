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

        while !lfs_pair_isnull(&(*pdir).tail) {
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
///
///     lfs_block_t child[2];
///     int err = lfs_bd_read(lfs,
///             &lfs->pcache, &lfs->rcache, lfs->cfg->block_size,
///             disk->block, disk->off, &child, sizeof(child));
///     if (err) {
///         return err;
///     }
///
///     lfs_pair_fromle32(child);
///     return (lfs_pair_cmp(child, find->pair) == 0) ? LFS_CMP_EQ : LFS_CMP_LT;
/// }
/// #endif
/// ```
pub fn lfs_fs_parent_match(
    _data: *mut core::ffi::c_void,
    _block: crate::types::lfs_block_t,
) -> i32 {
    todo!("lfs_fs_parent_match")
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
    _lfs: *mut crate::fs::Lfs,
    _pair: *const [crate::types::lfs_block_t; 2],
    _parent: *mut crate::dir::LfsMdir,
) -> crate::types::lfs_stag_t {
    todo!("lfs_fs_parent")
}
