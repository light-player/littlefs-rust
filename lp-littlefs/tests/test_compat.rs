//! Upstream: tests/test_compat.toml
//!
//! Version edge-case tests. The 14 forward/backward compat tests live in
//! lp-littlefs-compat where they test actual C ↔ Rust interop.
//! These 3 remaining tests exercise superblock version field handling.

mod common;

/// Upstream: [cases.test_compat_major_incompat]
///
/// Bump major version in superblock, verify mount rejects with LFS_ERR_INVAL.
#[test]
#[ignore = "stub: requires internal superblock APIs (test-parity2 phase 7)"]
fn test_compat_major_incompat() {
    todo!()
}

/// Upstream: [cases.test_compat_minor_incompat]
///
/// Bump minor version in superblock beyond what we support, verify mount rejects.
#[test]
#[ignore = "stub: requires internal superblock APIs (test-parity2 phase 7)"]
fn test_compat_minor_incompat() {
    todo!()
}

/// Upstream: [cases.test_compat_minor_bump]
///
/// Downgrade minor version in superblock, mount works, write triggers minor bump.
#[test]
#[ignore = "stub: requires internal superblock APIs (test-parity2 phase 7)"]
fn test_compat_minor_bump() {
    todo!()
}
