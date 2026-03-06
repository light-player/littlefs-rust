//! C ↔ Rust operation-level compat tests.
//! Ported from lp-littlefs-core-c-align/tests/align_tests.rs.

use lp_littlefs_compat::c_impl;
use lp_littlefs_compat::rust_impl;
use lp_littlefs_compat::storage::{SharedStorage, TestGeometry};

fn default_storage() -> SharedStorage {
    SharedStorage::with_defaults()
}

fn storage_with_prog_size(prog_size: u32, block_size: u32) -> SharedStorage {
    SharedStorage::new(TestGeometry {
        prog_size,
        block_size,
        ..TestGeometry::default()
    })
}

// ── Format / mount ──────────────────────────────────────────────────────

#[test]
fn c_format_rust_mount_root() {
    let storage = default_storage();
    c_impl::format(&storage).expect("C format");
    let names = rust_impl::mount_dir_names(&storage, "/").expect("Rust mount");
    assert!(names.is_empty(), "root should be empty, got: {names:?}");
}

#[test]
fn rust_format_c_mount_root() {
    let storage = default_storage();
    rust_impl::format(&storage).expect("Rust format");
    let names = c_impl::mount_dir_names(&storage, "/").expect("C mount");
    assert!(names.is_empty(), "root should be empty, got: {names:?}");
}

// ── Mkdir + file ────────────────────────────────────────────────────────

#[test]
fn c_mkdir_file_rust_sees_both() {
    let storage = default_storage();
    c_impl::format_mkdir_file_unmount(&storage, "potato", "burito").expect("C write");
    let names = rust_impl::mount_dir_names(&storage, "/").expect("Rust read");
    assert!(
        names.contains(&"potato".to_string()),
        "missing potato: {names:?}"
    );
    assert!(
        names.contains(&"burito".to_string()),
        "missing burito: {names:?}"
    );
}

#[test]
fn rust_mkdir_file_c_sees_both() {
    let storage = default_storage();
    rust_impl::format_mkdir_file_unmount(&storage, "potato", "burito").expect("Rust write");
    let names = c_impl::mount_dir_names(&storage, "/").expect("C read");
    assert!(
        names.contains(&"potato".to_string()),
        "missing potato: {names:?}"
    );
    assert!(
        names.contains(&"burito".to_string()),
        "missing burito: {names:?}"
    );
}

#[test]
fn c_mkdir_file_rust_sees_both_reverse_order() {
    let storage = default_storage();
    c_impl::format_file_mkdir_unmount(&storage, "burito", "potato").expect("C write");
    let names = rust_impl::mount_dir_names(&storage, "/").expect("Rust read");
    assert!(
        names.contains(&"potato".to_string()),
        "missing potato: {names:?}"
    );
    assert!(
        names.contains(&"burito".to_string()),
        "missing burito: {names:?}"
    );
}

// ── Insert-before ordering ──────────────────────────────────────────────

#[test]
fn c_insert_before_rust_sees_all() {
    let storage = default_storage();
    c_impl::format_create_three_unmount(&storage).expect("C write");
    let names = rust_impl::mount_dir_names(&storage, "/").expect("Rust read");
    assert!(names.contains(&"aaa".to_string()), "missing aaa: {names:?}");
    assert!(names.contains(&"zzz".to_string()), "missing zzz: {names:?}");
    assert!(names.contains(&"mmm".to_string()), "missing mmm: {names:?}");
}

#[test]
fn rust_insert_before_c_sees_all() {
    let storage = default_storage();
    rust_impl::format_create_three_unmount(&storage).expect("Rust write");
    let names = c_impl::mount_dir_names(&storage, "/").expect("C read");
    assert!(names.contains(&"aaa".to_string()), "missing aaa: {names:?}");
    assert!(names.contains(&"zzz".to_string()), "missing zzz: {names:?}");
    assert!(names.contains(&"mmm".to_string()), "missing mmm: {names:?}");
}

// ── Remount persist ─────────────────────────────────────────────────────

#[test]
fn c_mkdir_remount_exist() {
    let storage = default_storage();
    c_impl::format_mkdir_unmount(&storage, "potato").expect("C write");
    c_impl::mount_mkdir_expect_exist(&storage, "potato").expect("expected EXIST");
}

// ── Rename ──────────────────────────────────────────────────────────────

#[test]
fn c_rename_rust_sees_new_name() {
    let storage = default_storage();
    c_impl::format_create_rename_unmount(&storage, "oldfile", "newfile").expect("C write");
    let names = rust_impl::mount_dir_names(&storage, "/").expect("Rust read");
    assert!(
        !names.contains(&"oldfile".to_string()),
        "old name visible: {names:?}"
    );
    assert!(
        names.contains(&"newfile".to_string()),
        "new name missing: {names:?}"
    );
}

#[test]
fn rust_rename_c_sees_new_name() {
    let storage = default_storage();
    rust_impl::format_create_rename_unmount(&storage, "oldfile", "newfile").expect("Rust write");
    let names = c_impl::mount_dir_names(&storage, "/").expect("C read");
    assert!(
        !names.contains(&"oldfile".to_string()),
        "old name visible: {names:?}"
    );
    assert!(
        names.contains(&"newfile".to_string()),
        "new name missing: {names:?}"
    );
}

// ── Remove ──────────────────────────────────────────────────────────────

#[test]
fn c_remove_rust_sees_gone() {
    let storage = default_storage();
    c_impl::format_create_remove_unmount(&storage, "goner").expect("C write");
    let names = rust_impl::mount_dir_names(&storage, "/").expect("Rust read");
    assert!(
        !names.contains(&"goner".to_string()),
        "removed file visible: {names:?}"
    );
}

#[test]
fn rust_remove_c_sees_gone() {
    let storage = default_storage();
    rust_impl::format_create_remove_unmount(&storage, "goner").expect("Rust write");
    let names = c_impl::mount_dir_names(&storage, "/").expect("C read");
    assert!(
        !names.contains(&"goner".to_string()),
        "removed file visible: {names:?}"
    );
}

// ── File content ────────────────────────────────────────────────────────

#[test]
fn c_write_rust_reads_content_inline() {
    let storage = default_storage();
    let content = b"hello littlefs";
    c_impl::format_create_write_unmount(&storage, "f", content).expect("C write");
    let read = rust_impl::mount_read_file(&storage, "f").expect("Rust read");
    assert_eq!(read, content, "inline content mismatch");
}

#[test]
fn rust_write_c_reads_content_inline() {
    let storage = default_storage();
    let content = b"hello littlefs";
    rust_impl::format_create_write_unmount(&storage, "f", content).expect("Rust write");
    let read = c_impl::mount_read_file(&storage, "f").expect("C read");
    assert_eq!(read, content, "inline content mismatch");
}

#[test]
fn c_write_rust_reads_content_ctz() {
    let storage = default_storage();
    let content: Vec<u8> = (0..600).map(|i| (i % 256) as u8).collect();
    c_impl::format_create_write_unmount(&storage, "big", &content).expect("C write");
    let read = rust_impl::mount_read_file(&storage, "big").expect("Rust read");
    assert_eq!(read, content, "CTZ content mismatch");
}

#[test]
fn rust_write_c_reads_content_ctz() {
    let storage = default_storage();
    let content: Vec<u8> = (0..600).map(|i| (i % 256) as u8).collect();
    rust_impl::format_create_write_unmount(&storage, "big", &content).expect("Rust write");
    let read = c_impl::mount_read_file(&storage, "big").expect("C read");
    assert_eq!(read, content, "CTZ content mismatch");
}

// ── Nested dirs ─────────────────────────────────────────────────────────

#[test]
fn c_nested_dir_rust_sees() {
    let storage = default_storage();
    c_impl::format_nested_dir_file_unmount(&storage, "a", "b", "f").expect("C write");

    let root = rust_impl::mount_dir_names(&storage, "/").expect("Rust /");
    assert!(root.contains(&"a".to_string()), "missing a: {root:?}");

    let a = rust_impl::mount_dir_names(&storage, "a").expect("Rust a");
    assert!(a.contains(&"b".to_string()), "missing b: {a:?}");

    let ab = rust_impl::mount_dir_names(&storage, "a/b").expect("Rust a/b");
    assert!(ab.contains(&"f".to_string()), "missing f: {ab:?}");
}

#[test]
fn rust_nested_dir_c_sees() {
    let storage = default_storage();
    rust_impl::format_nested_dir_file_unmount(&storage, "a", "b", "f").expect("Rust write");

    let root = c_impl::mount_dir_names(&storage, "/").expect("C /");
    assert!(root.contains(&"a".to_string()), "missing a: {root:?}");

    let a = c_impl::mount_dir_names(&storage, "a").expect("C a");
    assert!(a.contains(&"b".to_string()), "missing b: {a:?}");

    let ab = c_impl::mount_dir_names(&storage, "a/b").expect("C a/b");
    assert!(ab.contains(&"f".to_string()), "missing f: {ab:?}");
}

// ── Rmdir ───────────────────────────────────────────────────────────────

#[test]
fn c_rmdir_rust_sees_gone() {
    let storage = default_storage();
    c_impl::format_mkdir_file_rmdir_unmount(&storage, "emptydir", "f").expect("C write");
    let names = rust_impl::mount_dir_names(&storage, "/").expect("Rust read");
    assert!(
        !names.contains(&"emptydir".to_string()),
        "rmdir'd dir visible: {names:?}"
    );
}

#[test]
fn rust_rmdir_c_sees_gone() {
    let storage = default_storage();
    rust_impl::format_mkdir_file_rmdir_unmount(&storage, "emptydir", "f").expect("Rust write");
    let names = c_impl::mount_dir_names(&storage, "/").expect("C read");
    assert!(
        !names.contains(&"emptydir".to_string()),
        "rmdir'd dir visible: {names:?}"
    );
}

// ── CRC layout: different prog_size ─────────────────────────────────────

#[test]
fn format_layout_prog_size_16() {
    let storage = storage_with_prog_size(16, 512);
    rust_impl::format(&storage).expect("Rust format");
    let names = c_impl::mount_dir_names(&storage, "/").expect("C mount");
    assert!(names.is_empty(), "expected empty root: {names:?}");
}

#[test]
fn format_layout_prog_size_64() {
    let storage = storage_with_prog_size(64, 512);
    c_impl::format(&storage).expect("C format");
    let names = rust_impl::mount_dir_names(&storage, "/").expect("Rust mount");
    assert!(names.is_empty(), "expected empty root: {names:?}");
}
