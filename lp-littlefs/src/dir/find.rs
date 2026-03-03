//! Directory find. Per lfs.c lfs_dir_find, lfs_dir_find_match.

/// Per lfs.c lfs_dir_find (lines 1483+)
///
/// C: Walks path components, lfs_dir_fetchmatch for each, lfs_dir_find_match
/// callback. Returns tag or negative error.
pub fn lfs_dir_find(_lfs: *const core::ffi::c_void) -> i32 {
    todo!("lfs_dir_find")
}
