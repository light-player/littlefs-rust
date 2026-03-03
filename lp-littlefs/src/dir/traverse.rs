//! Directory traverse. Per lfs.c lfs_dir_traverse, lfs_dir_traverse_filter, lfs_dir_getread.

/// Per lfs.c lfs_dir_traverse (lines 912+)
///
/// C: Iterates over directory and attrs, uses explicit stack for recursion.
/// Calls cb for each matching tag. Handles LFS_FROM_MOVE recursion.
pub fn lfs_dir_traverse(_lfs: *const core::ffi::c_void) -> i32 {
    todo!("lfs_dir_traverse")
}
