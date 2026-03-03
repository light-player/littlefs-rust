//! Evil/corruption recovery tests.
//!
//! Corresponds to upstream test_evil.toml
//! Source: https://github.com/littlefs-project/littlefs/blob/master/tests/test_evil.toml
//!
//! Tests recovery from conditions that shouldn't happen during normal operation:
//! invalid pointers, mdir loops, block-level corruption.

mod common;

// --- test_evil_invalid_tail_pointer ---
// Upstream: change tail to invalid block; mount => Corrupt
#[test]
#[ignore = "block-level corruption simulation not implemented"]
fn test_evil_invalid_tail_pointer() {}

// --- test_evil_invalid_dir_pointer ---
// Upstream: change dir struct pointer to invalid; dir_open, stat, file_open => Corrupt
#[test]
#[ignore = "block-level corruption simulation not implemented"]
fn test_evil_invalid_dir_pointer() {}

// --- test_evil_invalid_move_pointer ---
#[test]
#[ignore = "block-level corruption simulation not implemented"]
fn test_evil_invalid_move_pointer() {}

// --- test_evil_powerloss ---
#[test]
#[ignore = "powerloss runner not implemented"]
fn test_evil_powerloss() {}

// --- test_evil_mdir_loop ---
// Upstream: create mdir loop via corrupted pointers; mount should detect
#[test]
#[ignore = "block-level corruption simulation not implemented"]
fn test_evil_mdir_loop() {}

// --- test_evil_multiple_revs ---
#[test]
#[ignore = "block-level corruption simulation not implemented"]
fn test_evil_multiple_revs() {}

// --- test_evil_split_both_dirs ---
#[test]
#[ignore = "block-level corruption simulation not implemented"]
fn test_evil_split_both_dirs() {}

// --- test_evil_double_compact ---
#[test]
#[ignore = "block-level corruption simulation not implemented"]
fn test_evil_double_compact() {}
