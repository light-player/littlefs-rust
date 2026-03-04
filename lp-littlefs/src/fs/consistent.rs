//! Consistency. Per lfs.c lfs_fs_mkconsistent_, lfs_fs_gc_.

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
pub fn lfs_fs_mkconsistent_(_lfs: *mut super::lfs::Lfs) -> i32 {
    todo!("lfs_fs_mkconsistent_")
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
pub fn lfs_fs_gc_(_lfs: *mut super::lfs::Lfs) -> i32 {
    todo!("lfs_fs_gc_")
}
