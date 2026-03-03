//! File operations. Per lfs.c lfs_file_opencfg_, lfs_file_close_, lfs_file_sync_, etc.

/// Per lfs.c lfs_file_opencfg_ (lines 3065+)
///
/// C: lfs_fs_forceconsistency if write, lfs_dir_find, lfs_mlist_append,
/// create or load ctz from lfs_dir_get, handle truncate, inline/outline.
pub fn lfs_file_opencfg_(_lfs: *const core::ffi::c_void) -> i32 {
    todo!("lfs_file_opencfg_")
}
