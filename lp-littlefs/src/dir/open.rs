//! Directory open/read. Per lfs.c lfs_dir_open_, lfs_dir_close_, lfs_dir_read_, etc.

/// Per lfs.c lfs_dir_open_ (lines 2721+)
///
/// C: lfs_dir_find, check LFS_TYPE_DIR, get pair from root or lfs_dir_get,
/// lfs_dir_fetch, set dir->head from dir->m.pair.
pub fn lfs_dir_open_(
    _lfs: *const core::ffi::c_void,
    _dir: *mut super::lfs_dir::LfsDir,
    _path: *const i8,
) -> i32 {
    todo!("lfs_dir_open_")
}
