//! Initialization. Per lfs.c lfs_init, lfs_deinit.

/// Per lfs.c lfs_init (lines 4198-4371)
///
/// C: Validates cfg, sets rcache/pcache/lookahead buffers (malloc or provided),
/// lfs_cache_zero both caches, sets name_max/file_max/attr_max/inline_max,
/// initializes root, mlist, gstate.
pub fn lfs_init(_lfs: *mut super::lfs::Lfs, _cfg: *const crate::lfs_config::LfsConfig) -> i32 {
    todo!("lfs_init")
}
