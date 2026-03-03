//! Directory commit. Per lfs.c lfs_dir_commit, lfs_dir_commitattr, lfs_dir_alloc, etc.

/// Per lfs.c lfs_dir_commit (lines 2601-2619)
///
/// C:
/// ```c
/// static int lfs_dir_commit(lfs_t *lfs, lfs_mdir_t *dir,
///         const struct lfs_mattr *attrs, int attrcount) {
///     int orphans = lfs_dir_orphaningcommit(lfs, dir, attrs, attrcount);
///     if (orphans < 0) return orphans;
///     if (orphans) {
///         int err = lfs_fs_deorphan(lfs, false);
///         if (err) return err;
///     }
///     return 0;
/// }
/// ```
pub fn lfs_dir_commit(_lfs: *const core::ffi::c_void, _dir: *mut super::lfs_mdir::LfsMdir) -> i32 {
    todo!("lfs_dir_commit")
}
