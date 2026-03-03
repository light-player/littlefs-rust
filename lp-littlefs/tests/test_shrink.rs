//! Filesystem shrink tests. Per upstream test_shrink.toml.
//!
//! Both tests require lfs_fs_grow/shrink (block_count change) — not implemented.

mod common;

// --- test_shrink_simple ---
#[test]
#[ignore = "fs_grow/fs_shrink (block_count change) not implemented"]
fn test_shrink_simple() {
    // Would: lfs_fs_grow(&lfs, AFTER_BLOCK_COUNT), remount with new block_count
}

// --- test_shrink_full ---
#[test]
#[ignore = "fs_grow/fs_shrink (block_count change) not implemented"]
fn test_shrink_full() {
    // Would: create FILES_COUNT files, lfs_fs_grow to AFTER_BLOCK_COUNT,
    // verify files still readable, remount with new block_count
}
