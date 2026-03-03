//! Bad block tests.
//!
//! Corresponds to upstream test_badblocks.toml
//! Source: https://github.com/littlefs-project/littlefs/blob/master/tests/test_badblocks.toml
//!
//! Requires bad-block BD simulation (blocks that return Corrupt or no-op).

mod common;

// --- test_badblocks_single ---
// Upstream: one bad block, format/mount/ops work around it
#[test]
#[ignore = "bad-block BD simulation not implemented"]
fn test_badblocks_single() {}

// --- test_badblocks_double ---
#[test]
#[ignore = "bad-block BD simulation not implemented"]
fn test_badblocks_double() {}

// --- test_badblocks_boundary ---
#[test]
#[ignore = "bad-block BD simulation not implemented"]
fn test_badblocks_boundary() {}

// --- test_badblocks_corrupt ---
// Upstream: bad block returns corrupt, propagation
#[test]
#[ignore = "bad-block BD simulation not implemented"]
fn test_badblocks_corrupt() {}
