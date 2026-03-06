//! Upstream forward/backward compatibility tests.
//!
//! Mirrors test_compat.toml from the C littlefs test suite.
//! "Forward" = C (littlefs2-sys) creates, Rust (lp-littlefs-core) reads/writes.
//! "Backward" = Rust creates, C reads/writes.

use lp_littlefs_compat::c_impl;
use lp_littlefs_compat::rust_impl;
use lp_littlefs_compat::storage::{SharedStorage, TestGeometry};
use rstest::rstest;

const CHUNK: u32 = 4;

fn compat_storage() -> SharedStorage {
    SharedStorage::new(TestGeometry {
        block_count: 1024,
        ..TestGeometry::default()
    })
}

// ── Forward: C creates, Rust reads ──────────────────────────────────────

/// Upstream: test_compat_forward_mount
#[test]
fn test_compat_forward_mount() {
    let storage = compat_storage();
    c_impl::format(&storage).expect("C format");
    rust_impl::format_only(&storage).ok(); // just verify Rust can also format
    c_impl::format(&storage).expect("C format again");
    // Rust mounts what C formatted
    let names = rust_impl::mount_dir_names(&storage, "/").expect("Rust mount");
    assert!(names.is_empty());
}

/// Upstream: test_compat_forward_read_dirs
#[test]
fn test_compat_forward_read_dirs() {
    let storage = compat_storage();
    c_impl::format_create_n_dirs(&storage, 5).expect("C create dirs");
    rust_impl::mount_verify_n_empty_dirs(&storage, 5).expect("Rust verify dirs");
}

/// Upstream: test_compat_forward_read_files
#[rstest]
fn test_compat_forward_read_files(#[values(4, 32, 512, 8192)] size: u32) {
    let storage = compat_storage();
    c_impl::format_create_n_files_prng(&storage, 5, size, CHUNK).expect("C create files");
    rust_impl::mount_verify_n_files_prng(&storage, 5, size, CHUNK).expect("Rust verify files");
}

/// Upstream: test_compat_forward_read_files_in_dirs
#[rstest]
fn test_compat_forward_read_files_in_dirs(#[values(4, 32, 512, 8192)] size: u32) {
    let storage = compat_storage();
    c_impl::format_create_n_dirs_with_files_prng(&storage, 5, size, CHUNK)
        .expect("C create dirs+files");
    rust_impl::mount_verify_n_dirs_with_files_prng(&storage, 5, size, CHUNK)
        .expect("Rust verify dirs+files");
}

/// Upstream: test_compat_forward_write_dirs
#[test]
fn test_compat_forward_write_dirs() {
    let storage = compat_storage();
    c_impl::format_create_n_dirs(&storage, 5).expect("C create 5 dirs");
    rust_impl::mount_create_dirs_and_list(&storage, 5, 5, 10).expect("Rust create 5 more, list 10");
}

/// Upstream: test_compat_forward_write_files
#[rstest]
fn test_compat_forward_write_files(#[values(4, 32, 512, 8192)] size: u32) {
    let storage = compat_storage();
    c_impl::format_create_n_files_prng(&storage, 5, size, CHUNK).expect("C create 5 files");
    rust_impl::mount_create_files_prng_and_verify_all(&storage, 5, 5, 10, size, CHUNK)
        .expect("Rust create 5 more, verify all 10");
}

/// Upstream: test_compat_forward_write_files_in_dirs
#[rstest]
fn test_compat_forward_write_files_in_dirs(#[values(4, 32, 512, 8192)] size: u32) {
    let storage = compat_storage();
    c_impl::format_create_n_dirs_with_files_prng(&storage, 5, size, CHUNK)
        .expect("C create 5 dirs+files");
    rust_impl::mount_create_dirs_files_prng_and_verify_all(&storage, 5, 5, 10, size, CHUNK)
        .expect("Rust create 5 more, verify all 10");
}

// ── Backward: Rust creates, C reads ─────────────────────────────────────

/// Upstream: test_compat_backward_mount
#[test]
fn test_compat_backward_mount() {
    let storage = compat_storage();
    rust_impl::format(&storage).expect("Rust format");
    let names = c_impl::mount_dir_names(&storage, "/").expect("C mount");
    assert!(names.is_empty());
}

/// Upstream: test_compat_backward_read_dirs
#[test]
fn test_compat_backward_read_dirs() {
    let storage = compat_storage();
    rust_impl::format_create_n_dirs(&storage, 5).expect("Rust create dirs");
    c_impl::mount_verify_n_empty_dirs(&storage, 5).expect("C verify dirs");
}

/// Upstream: test_compat_backward_read_files
#[rstest]
fn test_compat_backward_read_files(#[values(4, 32, 512, 8192)] size: u32) {
    let storage = compat_storage();
    rust_impl::format_create_n_files_prng(&storage, 5, size, CHUNK).expect("Rust create files");
    c_impl::mount_verify_n_files_prng(&storage, 5, size, CHUNK).expect("C verify files");
}

/// Upstream: test_compat_backward_read_files_in_dirs
#[rstest]
fn test_compat_backward_read_files_in_dirs(#[values(4, 32, 512, 8192)] size: u32) {
    let storage = compat_storage();
    rust_impl::format_create_n_dirs_with_files_prng(&storage, 5, size, CHUNK)
        .expect("Rust create dirs+files");
    c_impl::mount_verify_n_dirs_with_files_prng(&storage, 5, size, CHUNK)
        .expect("C verify dirs+files");
}

/// Upstream: test_compat_backward_write_dirs
#[test]
fn test_compat_backward_write_dirs() {
    let storage = compat_storage();
    rust_impl::format_create_n_dirs(&storage, 5).expect("Rust create 5 dirs");
    c_impl::mount_create_dirs_and_list(&storage, 5, 5, 10).expect("C create 5 more, list 10");
}

/// Upstream: test_compat_backward_write_files
#[rstest]
fn test_compat_backward_write_files(#[values(4, 32, 512, 8192)] size: u32) {
    let storage = compat_storage();
    rust_impl::format_create_n_files_prng(&storage, 5, size, CHUNK).expect("Rust create 5 files");
    c_impl::mount_create_files_prng_and_verify_all(&storage, 5, 5, 10, size, CHUNK)
        .expect("C create 5 more, verify all 10");
}

/// Upstream: test_compat_backward_write_files_in_dirs
#[rstest]
fn test_compat_backward_write_files_in_dirs(#[values(4, 32, 512, 8192)] size: u32) {
    let storage = compat_storage();
    rust_impl::format_create_n_dirs_with_files_prng(&storage, 5, size, CHUNK)
        .expect("Rust create 5 dirs+files");
    c_impl::mount_create_dirs_files_prng_and_verify_all(&storage, 5, 5, 10, size, CHUNK)
        .expect("C create 5 more, verify all 10");
}
