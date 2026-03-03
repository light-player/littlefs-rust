//! Format. Per lfs.c lfs_format_.

/// Per lfs.c lfs_format_ (lines 4391-4455)
///
/// C: lfs_init, memset lookahead, lfs_dir_alloc root, write superblock via
/// lfs_dir_commit, force compaction, lfs_dir_fetch sanity check.
pub fn lfs_format_(_lfs: *mut super::lfs::Lfs, _cfg: *const crate::lfs_config::LfsConfig) -> i32 {
    todo!("lfs_format_")
}
