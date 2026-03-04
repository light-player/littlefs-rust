//! FS grow. Per lfs.c lfs_fs_grow_, lfs_shrink_checkblock.

/// Per lfs.c lfs_shrink_checkblock (lines 5244-5251)
///
/// C:
/// ```c
/// static int lfs_shrink_checkblock(void *data, lfs_block_t block) {
///     lfs_size_t threshold = *((lfs_size_t*)data);
///     if (block >= threshold) {
///         return LFS_ERR_NOTEMPTY;
///     }
///     return 0;
/// }
/// #endif
/// ```
pub fn lfs_shrink_checkblock(
    _data: *mut core::ffi::c_void,
    _block: crate::types::lfs_block_t,
) -> i32 {
    todo!("lfs_shrink_checkblock")
}

/// Per lfs.c lfs_fs_grow_ (lines 5253-5303)
///
/// C:
/// ```c
/// static int lfs_fs_grow_(lfs_t *lfs, lfs_size_t block_count) {
///     int err;
///
///     if (block_count == lfs->block_count) {
///         return 0;
///     }
///
///     
/// #ifndef LFS_SHRINKNONRELOCATING
///     // shrinking is not supported
///     LFS_ASSERT(block_count >= lfs->block_count);
/// #endif
/// #ifdef LFS_SHRINKNONRELOCATING
///     if (block_count < lfs->block_count) {
///         err = lfs_fs_traverse_(lfs, lfs_shrink_checkblock, &block_count, true);
///         if (err) {
///             return err;
///         }
///     }
/// #endif
///
///     lfs->block_count = block_count;
///
///     // fetch the root
///     lfs_mdir_t root;
///     err = lfs_dir_fetch(lfs, &root, lfs->root);
///     if (err) {
///         return err;
///     }
///
///     // update the superblock
///     lfs_superblock_t superblock;
///     lfs_stag_t tag = lfs_dir_get(lfs, &root, LFS_MKTAG(0x7ff, 0x3ff, 0),
///             LFS_MKTAG(LFS_TYPE_INLINESTRUCT, 0, sizeof(superblock)),
///             &superblock);
///     if (tag < 0) {
///         return tag;
///     }
///     lfs_superblock_fromle32(&superblock);
///
///     superblock.block_count = lfs->block_count;
///
///     lfs_superblock_tole32(&superblock);
///     err = lfs_dir_commit(lfs, &root, LFS_MKATTRS(
///             {tag, &superblock}));
///     if (err) {
///         return err;
///     }
///     return 0;
/// }
/// #endif
/// ```
pub fn lfs_fs_grow_(_lfs: *mut super::lfs::Lfs, _block_count: crate::types::lfs_size_t) -> i32 {
    todo!("lfs_fs_grow_")
}
