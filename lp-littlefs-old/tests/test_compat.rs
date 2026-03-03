//! Compatibility tests (C↔Rust, version forward/backward).
//!
//! Corresponds to upstream test_compat.toml
//! Source: https://github.com/littlefs-project/littlefs/blob/master/tests/test_compat.toml
//!
//! These tests require formatting with C littlefs or fixture images.
//! Most are ignored until we have interop fixtures.

mod common;

// --- test_compat_forward_mount ---
// Upstream: format with newer C version, mount with Rust
#[test]
#[ignore = "C↔Rust format interop; need C binary or fixture"]
fn test_compat_forward_mount() {}

// --- test_compat_forward_read_dirs ---
#[test]
#[ignore = "C↔Rust interop"]
fn test_compat_forward_read_dirs() {}

// --- test_compat_forward_read_files ---
#[test]
#[ignore = "C↔Rust interop"]
fn test_compat_forward_read_files() {}

// --- test_compat_forward_read_files_in_dirs ---
#[test]
#[ignore = "C↔Rust interop"]
fn test_compat_forward_read_files_in_dirs() {}

// --- test_compat_forward_write_dirs ---
#[test]
#[ignore = "C↔Rust interop"]
fn test_compat_forward_write_dirs() {}

// --- test_compat_forward_write_files ---
#[test]
#[ignore = "C↔Rust interop"]
fn test_compat_forward_write_files() {}

// --- test_compat_forward_write_files_in_dirs ---
#[test]
#[ignore = "C↔Rust interop"]
fn test_compat_forward_write_files_in_dirs() {}

// --- test_compat_backward_mount ---
#[test]
#[ignore = "C↔Rust interop"]
fn test_compat_backward_mount() {}

// --- test_compat_backward_read_dirs ---
#[test]
#[ignore = "C↔Rust interop"]
fn test_compat_backward_read_dirs() {}

// --- test_compat_backward_read_files ---
#[test]
#[ignore = "C↔Rust interop"]
fn test_compat_backward_read_files() {}

// --- test_compat_backward_read_files_in_dirs ---
#[test]
#[ignore = "C↔Rust interop"]
fn test_compat_backward_read_files_in_dirs() {}

// --- test_compat_backward_write_dirs ---
#[test]
#[ignore = "C↔Rust interop"]
fn test_compat_backward_write_dirs() {}

// --- test_compat_backward_write_files ---
#[test]
#[ignore = "C↔Rust interop"]
fn test_compat_backward_write_files() {}

// --- test_compat_backward_write_files_in_dirs ---
#[test]
#[ignore = "C↔Rust interop"]
fn test_compat_backward_write_files_in_dirs() {}

// --- test_compat_major_incompat ---
#[test]
#[ignore = "version incompat handling"]
fn test_compat_major_incompat() {}

// --- test_compat_minor_incompat ---
#[test]
#[ignore = "version incompat handling"]
fn test_compat_minor_incompat() {}

// --- test_compat_minor_bump ---
#[test]
#[ignore = "version bump handling"]
fn test_compat_minor_bump() {}
