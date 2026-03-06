//! Upstream: tests/test_shrink.toml
//!
//! Shrink/grow block count tests. Guarded by LFS_SHRINKNONRELOCATING
//! (lfs_fs_grow shrink path not yet implemented).

mod common;

/// Upstream: [cases.test_shrink_simple]
///
/// defines.BLOCK_COUNT = [10, 15, 20]
/// defines.AFTER_BLOCK_COUNT = [5, 10, 15, 19]
/// if = 'AFTER_BLOCK_COUNT <= BLOCK_COUNT'
///
/// Format on BLOCK_COUNT blocks, shrink via lfs_fs_grow(AFTER_BLOCK_COUNT).
/// If sizes differ, mount with original config fails (LFS_ERR_INVAL),
/// mount with reduced config succeeds.
#[test]
#[ignore = "requires LFS_SHRINKNONRELOCATING"]
fn test_shrink_simple() {
    todo!()
}

/// Upstream: [cases.test_shrink_full]
///
/// defines.BLOCK_COUNT = [10, 15, 20]
/// defines.AFTER_BLOCK_COUNT = [5, 7, 10, 12, 15, 17, 20]
/// defines.FILES_COUNT = [7, 8, 9, 10]
/// if = 'AFTER_BLOCK_COUNT <= BLOCK_COUNT && FILES_COUNT + 2 < BLOCK_COUNT'
///
/// Create FILES_COUNT+1 files of BLOCK_SIZE-0x40 bytes. Shrink via
/// lfs_fs_grow(AFTER_BLOCK_COUNT). On success: verify all files and
/// remount with reduced config. On LFS_ERR_NOTEMPTY: shrink expected
/// to fail (too many files for smaller device).
#[test]
#[ignore = "requires LFS_SHRINKNONRELOCATING"]
fn test_shrink_full() {
    todo!()
}
