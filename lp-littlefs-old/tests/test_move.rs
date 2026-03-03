//! Move/rename tests. Per upstream test_move.toml.
//!
//! Cross-dir rename not implemented; corruption/powerloss tests need infra.

mod common;

use common::{dir_entry_names, fresh_fs, init_log};
use lp_littlefs::{FileType, OpenFlags};

// --- test_move_file ---
// Cross-dir rename a/hello -> c/hello
#[test]
#[ignore = "cross-dir rename not implemented (FROM_MOVE)"]
fn test_move_file() {
    init_log();
    let (bd, config, mut lfs) = fresh_fs();

    lfs.mkdir(&bd, &config, "a").unwrap();
    lfs.mkdir(&bd, &config, "b").unwrap();
    lfs.mkdir(&bd, &config, "c").unwrap();
    lfs.mkdir(&bd, &config, "d").unwrap();

    let mut file = lfs
        .file_open(
            &bd,
            &config,
            "a/hello",
            OpenFlags::new(OpenFlags::CREAT | OpenFlags::WRONLY),
        )
        .unwrap();
    lfs.file_write(&bd, &config, &mut file, b"hola\n").unwrap();
    lfs.file_write(&bd, &config, &mut file, b"bonjour\n")
        .unwrap();
    lfs.file_write(&bd, &config, &mut file, b"ohayo\n").unwrap();
    lfs.file_close(&bd, &config, file).unwrap();
    lfs.unmount(&bd, &config).unwrap();

    lfs.mount(&bd, &config).unwrap();
    lfs.rename(&bd, &config, "a/hello", "c/hello").unwrap();
    lfs.unmount(&bd, &config).unwrap();

    lfs.mount(&bd, &config).unwrap();
    let a_names = dir_entry_names(&mut lfs, &bd, &config, "a").unwrap();
    assert_eq!(a_names.len(), 0);
    let c_names = dir_entry_names(&mut lfs, &bd, &config, "c").unwrap();
    assert_eq!(c_names.len(), 1);
    assert_eq!(c_names[0], "hello");

    let info = lfs.stat(&bd, &config, "c/hello").unwrap();
    assert_eq!(info.size, 5 + 8 + 6);

    assert!(lfs.stat(&bd, &config, "a/hello").is_err());
    assert!(lfs.stat(&bd, &config, "b/hello").is_err());
    let mut file = lfs
        .file_open(&bd, &config, "c/hello", OpenFlags::new(OpenFlags::RDONLY))
        .unwrap();
    let mut buf = [0u8; 32];
    let n = lfs.file_read(&bd, &config, &mut file, &mut buf).unwrap();
    assert_eq!(n, 5);
    assert_eq!(&buf[..5], b"hola\n");
    lfs.file_read(&bd, &config, &mut file, &mut buf).unwrap();
    lfs.file_read(&bd, &config, &mut file, &mut buf).unwrap();
    lfs.file_close(&bd, &config, file).unwrap();
    assert!(lfs
        .file_open(&bd, &config, "d/hello", OpenFlags::new(OpenFlags::RDONLY))
        .is_err());
    lfs.unmount(&bd, &config).unwrap();
}

// --- test_move_nop ---
// Rename to self is legal
#[test]
fn test_move_nop() {
    init_log();
    let (bd, config, mut lfs) = fresh_fs();

    lfs.mkdir(&bd, &config, "hi").unwrap();
    lfs.rename(&bd, &config, "hi", "hi").unwrap();
    lfs.mkdir(&bd, &config, "hi/hi").unwrap();
    lfs.rename(&bd, &config, "hi/hi", "hi/hi").unwrap();
    lfs.mkdir(&bd, &config, "hi/hi/hi").unwrap();
    lfs.rename(&bd, &config, "hi/hi/hi", "hi/hi/hi").unwrap();

    let info = lfs.stat(&bd, &config, "hi/hi/hi").unwrap();
    assert_eq!(info.name().unwrap(), "hi");
    assert!(matches!(info.typ, FileType::Dir));
    lfs.unmount(&bd, &config).unwrap();
}

// --- test_move_file_corrupt_source ---
#[test]
#[ignore = "block-level corruption simulation not implemented"]
fn test_move_file_corrupt_source() {
    // Needs uncached BD + direct block corruption
}

// --- test_move_file_corrupt_source_dest ---
#[test]
#[ignore = "block-level corruption simulation not implemented"]
fn test_move_file_corrupt_source_dest() {
    // Needs corruption + PROG_SIZE <= 0x3fe
}

// --- test_move_file_after_corrupt ---
#[test]
#[ignore = "block-level corruption simulation not implemented"]
fn test_move_file_after_corrupt() {
    // Needs corruption + continue move
}

// --- test_move_reentrant_file ---
#[test]
#[ignore = "powerloss runner not implemented"]
fn test_move_reentrant_file() {
    // Needs automated powerloss at rename points
}

// --- test_move_dir ---
// Cross-dir rename a/hi -> c/hi
#[test]
#[ignore = "cross-dir rename not implemented (FROM_MOVE)"]
fn test_move_dir() {
    init_log();
    let (bd, config, mut lfs) = fresh_fs();

    lfs.mkdir(&bd, &config, "a").unwrap();
    lfs.mkdir(&bd, &config, "b").unwrap();
    lfs.mkdir(&bd, &config, "c").unwrap();
    lfs.mkdir(&bd, &config, "d").unwrap();
    lfs.mkdir(&bd, &config, "a/hi").unwrap();
    lfs.mkdir(&bd, &config, "a/hi/hola").unwrap();
    lfs.mkdir(&bd, &config, "a/hi/bonjour").unwrap();
    lfs.mkdir(&bd, &config, "a/hi/ohayo").unwrap();
    lfs.unmount(&bd, &config).unwrap();

    lfs.mount(&bd, &config).unwrap();
    lfs.rename(&bd, &config, "a/hi", "c/hi").unwrap();
    lfs.unmount(&bd, &config).unwrap();

    lfs.mount(&bd, &config).unwrap();
    let names = dir_entry_names(&mut lfs, &bd, &config, "c/hi").unwrap();
    assert!(names.contains(&"bonjour".to_string()));
    assert!(names.contains(&"hola".to_string()));
    assert!(names.contains(&"ohayo".to_string()));
    lfs.unmount(&bd, &config).unwrap();
}

// --- test_move_dir_corrupt_source ---
#[test]
#[ignore = "block-level corruption simulation not implemented"]
fn test_move_dir_corrupt_source() {}

// --- test_move_dir_corrupt_source_dest ---
#[test]
#[ignore = "block-level corruption simulation not implemented"]
fn test_move_dir_corrupt_source_dest() {}

// --- test_move_dir_after_corrupt ---
#[test]
#[ignore = "block-level corruption simulation not implemented"]
fn test_move_dir_after_corrupt() {}

// --- test_reentrant_dir ---
#[test]
#[ignore = "powerloss runner not implemented"]
fn test_reentrant_dir() {}

// --- test_move_state_stealing ---
// Chain a->b->c->d then remove b,c
#[test]
#[ignore = "cross-dir rename not implemented (FROM_MOVE)"]
fn test_move_state_stealing() {
    init_log();
    let (bd, config, mut lfs) = fresh_fs();

    lfs.mkdir(&bd, &config, "a").unwrap();
    lfs.mkdir(&bd, &config, "b").unwrap();
    lfs.mkdir(&bd, &config, "c").unwrap();
    lfs.mkdir(&bd, &config, "d").unwrap();
    let mut file = lfs
        .file_open(
            &bd,
            &config,
            "a/hello",
            OpenFlags::new(OpenFlags::CREAT | OpenFlags::WRONLY),
        )
        .unwrap();
    lfs.file_write(&bd, &config, &mut file, b"hola\n").unwrap();
    lfs.file_write(&bd, &config, &mut file, b"bonjour\n")
        .unwrap();
    lfs.file_write(&bd, &config, &mut file, b"ohayo\n").unwrap();
    lfs.file_close(&bd, &config, file).unwrap();
    lfs.unmount(&bd, &config).unwrap();

    lfs.mount(&bd, &config).unwrap();
    lfs.rename(&bd, &config, "a/hello", "b/hello").unwrap();
    lfs.unmount(&bd, &config).unwrap();
    lfs.mount(&bd, &config).unwrap();
    lfs.rename(&bd, &config, "b/hello", "c/hello").unwrap();
    lfs.unmount(&bd, &config).unwrap();
    lfs.mount(&bd, &config).unwrap();
    lfs.rename(&bd, &config, "c/hello", "d/hello").unwrap();
    lfs.unmount(&bd, &config).unwrap();

    lfs.mount(&bd, &config).unwrap();
    lfs.remove(&bd, &config, "b").unwrap();
    lfs.remove(&bd, &config, "c").unwrap();
    lfs.unmount(&bd, &config).unwrap();

    lfs.mount(&bd, &config).unwrap();
    let mut file = lfs
        .file_open(&bd, &config, "d/hello", OpenFlags::new(OpenFlags::RDONLY))
        .unwrap();
    let mut buf = [0u8; 32];
    let n = lfs.file_read(&bd, &config, &mut file, &mut buf).unwrap();
    assert_eq!(n, 5);
    assert_eq!(&buf[..5], b"hola\n");
    lfs.file_close(&bd, &config, file).unwrap();
    lfs.unmount(&bd, &config).unwrap();
}

// --- test_move_create_delete_same ---
// Same-dir rename while files open
#[test]
#[ignore = "rename with open files may have implementation-specific behavior"]
fn test_move_create_delete_same() {
    init_log();
    let (bd, config, mut lfs) = fresh_fs();

    let f1 = lfs
        .file_open(
            &bd,
            &config,
            "1.move_me",
            OpenFlags::new(OpenFlags::WRONLY | OpenFlags::CREAT),
        )
        .unwrap();
    lfs.file_close(&bd, &config, f1).unwrap();

    let mut f0 = lfs
        .file_open(
            &bd,
            &config,
            "0.before",
            OpenFlags::new(OpenFlags::WRONLY | OpenFlags::CREAT),
        )
        .unwrap();
    lfs.file_write(&bd, &config, &mut f0, b"test.1").unwrap();
    lfs.file_close(&bd, &config, f0).unwrap();

    let mut f2 = lfs
        .file_open(
            &bd,
            &config,
            "2.in_between",
            OpenFlags::new(OpenFlags::WRONLY | OpenFlags::CREAT),
        )
        .unwrap();
    lfs.file_write(&bd, &config, &mut f2, b"test.2").unwrap();
    lfs.file_close(&bd, &config, f2).unwrap();

    let mut f4 = lfs
        .file_open(
            &bd,
            &config,
            "4.after",
            OpenFlags::new(OpenFlags::WRONLY | OpenFlags::CREAT),
        )
        .unwrap();
    lfs.file_write(&bd, &config, &mut f4, b"test.3").unwrap();
    lfs.file_close(&bd, &config, f4).unwrap();

    let mut fa = lfs
        .file_open(
            &bd,
            &config,
            "0.before",
            OpenFlags::new(OpenFlags::WRONLY | OpenFlags::TRUNC),
        )
        .unwrap();
    let mut fb = lfs
        .file_open(
            &bd,
            &config,
            "2.in_between",
            OpenFlags::new(OpenFlags::WRONLY | OpenFlags::TRUNC),
        )
        .unwrap();
    let mut fc = lfs
        .file_open(
            &bd,
            &config,
            "4.after",
            OpenFlags::new(OpenFlags::WRONLY | OpenFlags::TRUNC),
        )
        .unwrap();
    lfs.file_write(&bd, &config, &mut fa, b"test.4").unwrap();
    lfs.file_write(&bd, &config, &mut fb, b"test.5").unwrap();
    lfs.file_write(&bd, &config, &mut fc, b"test.6").unwrap();

    lfs.rename(&bd, &config, "1.move_me", "3.move_me").unwrap();

    lfs.file_close(&bd, &config, fa).unwrap();
    lfs.file_close(&bd, &config, fb).unwrap();
    lfs.file_close(&bd, &config, fc).unwrap();

    let names = dir_entry_names(&mut lfs, &bd, &config, "/").unwrap();
    assert!(names.contains(&"0.before".to_string()));
    assert!(names.contains(&"2.in_between".to_string()));
    assert!(names.contains(&"3.move_me".to_string()));
    assert!(names.contains(&"4.after".to_string()));

    let mut file = lfs
        .file_open(&bd, &config, "0.before", OpenFlags::new(OpenFlags::RDONLY))
        .unwrap();
    let mut buf = [0u8; 16];
    let n = lfs.file_read(&bd, &config, &mut file, &mut buf).unwrap();
    assert_eq!(n, 7);
    assert_eq!(&buf[..7], b"test.4");
    lfs.file_close(&bd, &config, file).unwrap();

    lfs.unmount(&bd, &config).unwrap();
}

// --- test_move_create_delete_delete_same ---
#[test]
fn test_move_create_delete_delete_same() {
    init_log();
    let (bd, config, mut lfs) = fresh_fs();

    let f1 = lfs
        .file_open(
            &bd,
            &config,
            "1.move_me",
            OpenFlags::new(OpenFlags::WRONLY | OpenFlags::CREAT),
        )
        .unwrap();
    lfs.file_close(&bd, &config, f1).unwrap();

    let mut f3 = lfs
        .file_open(
            &bd,
            &config,
            "3.move_me",
            OpenFlags::new(OpenFlags::WRONLY | OpenFlags::CREAT),
        )
        .unwrap();
    lfs.file_write(&bd, &config, &mut f3, b"remove me").unwrap();
    lfs.file_close(&bd, &config, f3).unwrap();

    let mut f0 = lfs
        .file_open(
            &bd,
            &config,
            "0.before",
            OpenFlags::new(OpenFlags::WRONLY | OpenFlags::CREAT),
        )
        .unwrap();
    lfs.file_write(&bd, &config, &mut f0, b"test.1").unwrap();
    lfs.file_close(&bd, &config, f0).unwrap();

    let mut f2 = lfs
        .file_open(
            &bd,
            &config,
            "2.in_between",
            OpenFlags::new(OpenFlags::WRONLY | OpenFlags::CREAT),
        )
        .unwrap();
    lfs.file_write(&bd, &config, &mut f2, b"test.2").unwrap();
    lfs.file_close(&bd, &config, f2).unwrap();

    let mut f4 = lfs
        .file_open(
            &bd,
            &config,
            "4.after",
            OpenFlags::new(OpenFlags::WRONLY | OpenFlags::CREAT),
        )
        .unwrap();
    lfs.file_write(&bd, &config, &mut f4, b"test.3").unwrap();
    lfs.file_close(&bd, &config, f4).unwrap();

    let mut fa = lfs
        .file_open(
            &bd,
            &config,
            "0.before",
            OpenFlags::new(OpenFlags::WRONLY | OpenFlags::TRUNC),
        )
        .unwrap();
    let mut fb = lfs
        .file_open(
            &bd,
            &config,
            "2.in_between",
            OpenFlags::new(OpenFlags::WRONLY | OpenFlags::TRUNC),
        )
        .unwrap();
    let mut fc = lfs
        .file_open(
            &bd,
            &config,
            "4.after",
            OpenFlags::new(OpenFlags::WRONLY | OpenFlags::TRUNC),
        )
        .unwrap();
    lfs.file_write(&bd, &config, &mut fa, b"test.4").unwrap();
    lfs.file_write(&bd, &config, &mut fb, b"test.5").unwrap();
    lfs.file_write(&bd, &config, &mut fc, b"test.6").unwrap();

    lfs.rename(&bd, &config, "1.move_me", "3.move_me").unwrap();

    lfs.file_close(&bd, &config, fa).unwrap();
    lfs.file_close(&bd, &config, fb).unwrap();
    lfs.file_close(&bd, &config, fc).unwrap();

    let info = lfs.stat(&bd, &config, "3.move_me").unwrap();
    assert_eq!(info.size, 0);

    lfs.unmount(&bd, &config).unwrap();
}

// --- test_move_create_delete_different ---
// Cross-dir rename with open files
#[test]
#[ignore = "cross-dir rename not implemented (FROM_MOVE)"]
fn test_move_create_delete_different() {
    init_log();
    let (bd, config, mut lfs) = fresh_fs();

    lfs.mkdir(&bd, &config, "dir.1").unwrap();
    lfs.mkdir(&bd, &config, "dir.2").unwrap();
    let f = lfs
        .file_open(
            &bd,
            &config,
            "dir.1/1.move_me",
            OpenFlags::new(OpenFlags::WRONLY | OpenFlags::CREAT),
        )
        .unwrap();
    lfs.file_close(&bd, &config, f).unwrap();
    let mut f = lfs
        .file_open(
            &bd,
            &config,
            "dir.2/1.move_me",
            OpenFlags::new(OpenFlags::WRONLY | OpenFlags::CREAT),
        )
        .unwrap();
    lfs.file_write(&bd, &config, &mut f, b"remove me").unwrap();
    lfs.file_close(&bd, &config, f).unwrap();

    lfs.rename(&bd, &config, "dir.1/1.move_me", "dir.2/1.move_me")
        .unwrap();

    let names = dir_entry_names(&mut lfs, &bd, &config, "dir.2").unwrap();
    assert!(names.contains(&"1.move_me".to_string()));
    lfs.unmount(&bd, &config).unwrap();
}

// --- test_move_fix_relocation ---
#[test]
#[ignore = "cross-dir rename and lfs_emubd_setwear not implemented"]
fn test_move_fix_relocation() {}

// --- test_move_fix_relocation_predecessor ---
#[test]
#[ignore = "cross-dir rename and lfs_emubd_setwear not implemented"]
fn test_move_fix_relocation_predecessor() {}
