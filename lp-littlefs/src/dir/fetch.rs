//! Directory fetch. Per lfs.c lfs_dir_fetch, lfs_dir_getgstate, lfs_dir_getinfo.

/// Per lfs.c lfs_dir_fetch (lines 1387-1394)
///
/// C:
/// ```c
/// static int lfs_dir_fetch(lfs_t *lfs,
///         lfs_mdir_t *dir, const lfs_block_t pair[2]) {
///     return (int)lfs_dir_fetchmatch(lfs, dir, pair,
///             (lfs_tag_t)-1, (lfs_tag_t)-1, NULL, NULL, NULL);
/// }
/// ```
pub fn lfs_dir_fetch(
    _lfs: *const core::ffi::c_void,
    _dir: *mut super::lfs_mdir::LfsMdir,
    _pair: &[crate::types::lfs_block_t; 2],
) -> i32 {
    todo!("lfs_dir_fetch")
}
