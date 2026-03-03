//! Block allocator. Per lfs.c lfs_alloc, lfs_alloc_scan, lfs_alloc_lookahead, etc.

use crate::types::lfs_block_t;

/// Per lfs.c lfs_alloc (lines 666-716)
///
/// C: scans lookahead buffer for free blocks, calls lfs_alloc_scan when exhausted.
/// Returns LFS_ERR_NOSPC if ckpoint <= 0.
pub fn lfs_alloc(_lfs: *const core::ffi::c_void, _block: *mut lfs_block_t) -> i32 {
    todo!("lfs_alloc")
}
