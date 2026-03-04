//! FS traverse. Per lfs.c lfs_fs_traverse_.
//
/// Per lfs.c lfs_fs_traverse_ (lines 4693-4794)
///
/// C:
/// ```c
/// int lfs_fs_traverse_(lfs_t *lfs,
///         int (*cb)(void *data, lfs_block_t block), void *data,
///         bool includeorphans) {
///     // iterate over metadata pairs
///     lfs_mdir_t dir = {.tail = {0, 1}};
///
/// #ifdef LFS_MIGRATE
///     // also consider v1 blocks during migration
///     if (lfs->lfs1) {
///         int err = lfs1_traverse(lfs, cb, data);
///         if (err) {
///             return err;
///         }
///
///         dir.tail[0] = lfs->root[0];
///         dir.tail[1] = lfs->root[1];
///     }
/// #endif
///
///     struct lfs_tortoise_t tortoise = {
///         .pair = {LFS_BLOCK_NULL, LFS_BLOCK_NULL},
///         .i = 1,
///         .period = 1,
///     };
///     int err = LFS_ERR_OK;
///     while (!lfs_pair_isnull(dir.tail)) {
///         err = lfs_tortoise_detectcycles(&dir, &tortoise);
///         if (err < 0) {
///             return LFS_ERR_CORRUPT;
///         }
///
///         for (int i = 0; i < 2; i++) {
///             int err = cb(data, dir.tail[i]);
///             if (err) {
///                 return err;
///             }
///         }
///
///         // iterate through ids in directory
///         int err = lfs_dir_fetch(lfs, &dir, dir.tail);
///         if (err) {
///             return err;
///         }
///
///         for (uint16_t id = 0; id < dir.count; id++) {
///             struct lfs_ctz ctz;
///             lfs_stag_t tag = lfs_dir_get(lfs, &dir, LFS_MKTAG(0x700, 0x3ff, 0),
///                     LFS_MKTAG(LFS_TYPE_STRUCT, id, sizeof(ctz)), &ctz);
///             if (tag < 0) {
///                 if (tag == LFS_ERR_NOENT) {
///                     continue;
///                 }
///                 return tag;
///             }
///             lfs_ctz_fromle32(&ctz);
///
///             if (lfs_tag_type3(tag) == LFS_TYPE_CTZSTRUCT) {
///                 err = lfs_ctz_traverse(lfs, NULL, &lfs->rcache,
///                         ctz.head, ctz.size, cb, data);
///                 if (err) {
///                     return err;
///                 }
///             } else if (includeorphans &&
///                     lfs_tag_type3(tag) == LFS_TYPE_DIRSTRUCT) {
///                 for (int i = 0; i < 2; i++) {
///                     err = cb(data, (&ctz.head)[i]);
///                     if (err) {
///                         return err;
///                     }
///                 }
///             }
///         }
///     }
///
/// #ifndef LFS_READONLY
///     // iterate over any open files
///     for (lfs_file_t *f = (lfs_file_t*)lfs->mlist; f; f = f->next) {
///         if (f->type != LFS_TYPE_REG) {
///             continue;
///         }
///
///         if ((f->flags & LFS_F_DIRTY) && !(f->flags & LFS_F_INLINE)) {
///             int err = lfs_ctz_traverse(lfs, &f->cache, &lfs->rcache,
///                     f->ctz.head, f->ctz.size, cb, data);
///             if (err) {
///                 return err;
///             }
///         }
///
///         if ((f->flags & LFS_F_WRITING) && !(f->flags & LFS_F_INLINE)) {
///             int err = lfs_ctz_traverse(lfs, &f->cache, &lfs->rcache,
///                     f->block, f->pos, cb, data);
///             if (err) {
///                 return err;
///             }
///         }
///     }
/// #endif
///
///     return 0;
/// }
///
/// ```

pub fn lfs_fs_traverse_(
    _lfs: *mut super::lfs::Lfs,
    _cb: Option<unsafe extern "C" fn(*mut core::ffi::c_void, crate::types::lfs_block_t) -> i32>,
    _data: *mut core::ffi::c_void,
    _powerloss: bool,
) -> i32 {
    todo!("lfs_fs_traverse_")
}
