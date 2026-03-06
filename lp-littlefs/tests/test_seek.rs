//! Upstream: tests/test_seek.toml

mod common;

use rstest::rstest;

/// Upstream: [cases.test_seek_read]
/// defines = [{COUNT=132, SKIP=4}, {COUNT=132, SKIP=128}, {COUNT=200, SKIP=10},
///            {COUNT=200, SKIP=100}, {COUNT=4, SKIP=1}, {COUNT=4, SKIP=2}]
///
/// Write COUNT copies of "kittycatcat", seek to SKIP, verify read at various positions.
#[rstest]
#[case(132, 4)]
#[case(132, 128)]
#[case(200, 10)]
#[case(200, 100)]
#[case(4, 1)]
#[case(4, 2)]
#[ignore = "stub"]
fn test_seek_read(#[case] _count: u32, #[case] _skip: u32) {
    todo!()
}

/// Upstream: [cases.test_seek_write]
///
/// Seek and write; verify content at expected positions.
#[test]
#[ignore = "stub"]
fn test_seek_write() {
    todo!()
}

/// Upstream: [cases.test_seek_boundary]
///
/// Seek at block/chunk boundaries.
#[test]
#[ignore = "stub"]
fn test_seek_boundary() {
    todo!()
}

/// Upstream: [cases.test_seek_overflow]
///
/// Seek beyond file size behavior.
#[test]
#[ignore = "stub"]
fn test_seek_overflow() {
    todo!()
}

/// Upstream: [cases.test_seek_simple]
///
/// Basic seek + tell + read.
#[test]
#[ignore = "stub"]
fn test_seek_simple() {
    todo!()
}

/// Upstream: [cases.test_seek_dual]
///
/// Two open files, seek in each.
#[test]
#[ignore = "stub"]
fn test_seek_dual() {
    todo!()
}

/// Upstream: [cases.test_seek_rewind]
///
/// Rewind and re-read.
#[test]
#[ignore = "stub"]
fn test_seek_rewind() {
    todo!()
}

/// Upstream: [cases.test_seek_append]
///
/// Append mode seek behavior.
#[test]
#[ignore = "stub"]
fn test_seek_append() {
    todo!()
}

/// Upstream: [cases.test_seek_resize]
///
/// Seek after truncate/resize.
#[test]
#[ignore = "stub"]
fn test_seek_resize() {
    todo!()
}

/// Upstream: [cases.test_seek_sync]
///
/// Seek, sync, verify.
#[test]
#[ignore = "stub"]
fn test_seek_sync() {
    todo!()
}
