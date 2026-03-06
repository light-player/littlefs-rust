//! Upstream: tests/test_truncate.toml

mod common;

use rstest::rstest;

/// Upstream: [cases.test_truncate_simple]
/// defines.MEDIUMSIZE = [31, 32, 33, 511, 512, 513, 2047, 2048, 2049]
/// defines.LARGESIZE = [32, 33, 512, 513, 2048, 2049, 8192, 8193]
/// if = 'MEDIUMSIZE < LARGESIZE'
///
/// Write LARGESIZE "hair", truncate to MEDIUMSIZE, remount, verify.
#[rstest]
#[ignore = "stub"]
fn test_truncate_simple(
    #[values(31, 32, 33, 511, 512, 513, 2047, 2048, 2049)] medium: u32,
    #[values(32, 33, 512, 513, 2048, 2049, 8192, 8193)] large: u32,
) {
    if medium >= large {
        return;
    }
    todo!()
}

/// Upstream: [cases.test_truncate_read]
/// defines.MEDIUMSIZE = [...], defines.LARGESIZE = [...]
/// if = 'MEDIUMSIZE < LARGESIZE'
///
/// Truncate then read; verify read respects new size.
#[rstest]
#[ignore = "stub"]
fn test_truncate_read(
    #[values(31, 32, 33, 511, 512, 513, 2047, 2048, 2049)] medium: u32,
    #[values(32, 33, 512, 513, 2048, 2049, 8192, 8193)] large: u32,
) {
    if medium >= large {
        return;
    }
    todo!()
}

/// Upstream: [cases.test_truncate_write_read]
/// defines.MEDIUMSIZE = [...], defines.LARGESIZE = [...]
/// if = 'MEDIUMSIZE < LARGESIZE'
///
/// Truncate, write more, read; verify.
#[rstest]
#[ignore = "stub"]
fn test_truncate_write_read(
    #[values(31, 32, 33, 511, 512, 513, 2047, 2048, 2049)] medium: u32,
    #[values(32, 33, 512, 513, 2048, 2049, 8192, 8193)] large: u32,
) {
    if medium >= large {
        return;
    }
    todo!()
}

/// Upstream: [cases.test_truncate_write]
///
/// Truncate via open with O_TRUNC, write, verify.
#[test]
#[ignore = "stub"]
fn test_truncate_write() {
    todo!()
}

/// Upstream: [cases.test_truncate_reentrant_write]
///
/// Power-loss during truncate; reentrant write path.
#[test]
#[ignore = "stub"]
fn test_truncate_reentrant_write() {
    todo!()
}

/// Upstream: [cases.test_truncate_aggressive]
///
/// Aggressive truncate/grow cycling.
#[test]
#[ignore = "stub"]
fn test_truncate_aggressive() {
    todo!()
}

/// Upstream: [cases.test_truncate_nop]
///
/// Truncate to same size is no-op.
#[test]
#[ignore = "stub"]
fn test_truncate_nop() {
    todo!()
}
