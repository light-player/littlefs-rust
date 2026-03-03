//! Mount/unmount. Per lfs.c lfs_mount_, lfs_unmount_.

/// Per lfs.c lfs_mount_ (lines 4482-4645)
///
/// C: lfs_init, scan dir blocks for superblock via lfs_dir_fetchmatch,
/// tortoise cycle detect, update root/gstate, lfs_dir_getgstate, seed lookahead.
pub fn lfs_mount_(_lfs: *mut super::lfs::Lfs, _cfg: *const crate::lfs_config::LfsConfig) -> i32 {
    todo!("lfs_mount_")
}

/// Per lfs.c lfs_unmount_ (lines 4647-4649)
///
/// C:
/// ```c
/// static int lfs_unmount_(lfs_t *lfs) {
///     return lfs_deinit(lfs);
/// }
/// ```
pub fn lfs_unmount_(_lfs: *mut super::lfs::Lfs) -> i32 {
    todo!("lfs_unmount_")
}
