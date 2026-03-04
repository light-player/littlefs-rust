//! Format. Per lfs.c lfs_format_.

/// Per lfs.c lfs_format_ (lines 4391-4462)
///
/// C:
/// ```c
/// static int lfs_format_(lfs_t *lfs, const struct lfs_config *cfg) {
///     int err = 0;
///     {
///         err = lfs_init(lfs, cfg);
///         if (err) {
///             return err;
///         }
///
///         LFS_ASSERT(cfg->block_count != 0);
///
///         // create free lookahead
///         memset(lfs->lookahead.buffer, 0, lfs->cfg->lookahead_size);
///         lfs->lookahead.start = 0;
///         lfs->lookahead.size = lfs_min(8*lfs->cfg->lookahead_size,
///                 lfs->block_count);
///         lfs->lookahead.next = 0;
///         lfs_alloc_ckpoint(lfs);
///
///         // create root dir
///         lfs_mdir_t root;
///         err = lfs_dir_alloc(lfs, &root);
///         if (err) {
///             goto cleanup;
///         }
///
///         // write one superblock
///         lfs_superblock_t superblock = {
///             .version     = lfs_fs_disk_version(lfs),
///             .block_size  = lfs->cfg->block_size,
///             .block_count = lfs->block_count,
///             .name_max    = lfs->name_max,
///             .file_max    = lfs->file_max,
///             .attr_max    = lfs->attr_max,
///         };
///
///         lfs_superblock_tole32(&superblock);
///         err = lfs_dir_commit(lfs, &root, LFS_MKATTRS(
///                 {LFS_MKTAG(LFS_TYPE_CREATE, 0, 0), NULL},
///                 {LFS_MKTAG(LFS_TYPE_SUPERBLOCK, 0, 8), "littlefs"},
///                 {LFS_MKTAG(LFS_TYPE_INLINESTRUCT, 0, sizeof(superblock)),
///                     &superblock}));
///         if (err) {
///             goto cleanup;
///         }
///
///         // force compaction to prevent accidentally mounting any
///         // older version of littlefs that may live on disk
///         root.erased = false;
///         err = lfs_dir_commit(lfs, &root, NULL, 0);
///         if (err) {
///             goto cleanup;
///         }
///
///         // sanity check that fetch works
///         err = lfs_dir_fetch(lfs, &root, (const lfs_block_t[2]){0, 1});
///         if (err) {
///             goto cleanup;
///         }
///     }
///
/// cleanup:
///     lfs_deinit(lfs);
///     return err;
///
/// }
/// #endif
///
/// struct lfs_tortoise_t {
///     lfs_block_t pair[2];
///     lfs_size_t i;
///     lfs_size_t period;
/// };
/// ```
pub fn lfs_format_(_lfs: *mut super::lfs::Lfs, _cfg: *const crate::lfs_config::LfsConfig) -> i32 {
    todo!("lfs_format_")
}
