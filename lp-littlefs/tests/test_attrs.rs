//! Custom attributes tests. Per upstream test_attrs.toml.
//!
//! All tests ignore: getattr/setattr/removeattr not implemented.

mod common;

use common::fresh_fs;
use lp_littlefs::{Error, FileOpenConfig, OpenFlags, ATTR_MAX};

// --- test_attrs_get_set ---
#[test]
#[ignore = "getattr/setattr/removeattr not implemented"]
fn test_attrs_get_set() {
    let (bd, config, mut lfs) = fresh_fs();

    lfs.mkdir(&bd, &config, "hello").unwrap();
    let mut file = lfs
        .file_open(
            &bd,
            &config,
            "hello/hello",
            OpenFlags::new(OpenFlags::WRONLY | OpenFlags::CREAT),
        )
        .unwrap();
    let n = lfs.file_write(&bd, &config, &mut file, b"hello").unwrap();
    assert_eq!(n, 5);
    lfs.file_close(&bd, &config, file).unwrap();
    lfs.unmount(&bd, &config).unwrap();

    lfs.mount(&bd, &config).unwrap();
    let mut buffer = [0u8; 1024];

    lfs.setattr(&bd, &config, "hello", b'A', b"aaaa", 4)
        .unwrap();
    lfs.setattr(&bd, &config, "hello", b'B', b"bbbbbb", 6)
        .unwrap();
    lfs.setattr(&bd, &config, "hello", b'C', b"ccccc", 5)
        .unwrap();

    let n = lfs
        .getattr(&bd, &config, "hello", b'A', &mut buffer[..4])
        .unwrap();
    assert_eq!(n, 4);
    let n = lfs
        .getattr(&bd, &config, "hello", b'B', &mut buffer[4..10])
        .unwrap();
    assert_eq!(n, 6);
    let n = lfs
        .getattr(&bd, &config, "hello", b'C', &mut buffer[10..15])
        .unwrap();
    assert_eq!(n, 5);
    assert_eq!(&buffer[0..4], b"aaaa");
    assert_eq!(&buffer[4..10], b"bbbbbb");
    assert_eq!(&buffer[10..15], b"ccccc");

    lfs.setattr(&bd, &config, "hello", b'B', b"", 0).unwrap();
    let n = lfs
        .getattr(&bd, &config, "hello", b'B', &mut buffer[4..10])
        .unwrap();
    assert_eq!(n, 0);
    assert_eq!(&buffer[4..10], b"\0\0\0\0\0\0");

    lfs.removeattr(&bd, &config, "hello", b'B').unwrap();
    let err = lfs
        .getattr(&bd, &config, "hello", b'B', &mut buffer[4..10])
        .unwrap_err();
    assert_eq!(err, Error::Noattr);

    lfs.setattr(&bd, &config, "hello", b'B', b"dddddd", 6)
        .unwrap();
    lfs.setattr(&bd, &config, "hello", b'B', b"eee", 3).unwrap();

    let err = lfs
        .setattr(
            &bd,
            &config,
            "hello",
            b'A',
            &buffer[..ATTR_MAX + 1],
            ATTR_MAX + 1,
        )
        .unwrap_err();
    assert_eq!(err, Error::Nospc);

    lfs.setattr(&bd, &config, "hello", b'B', b"fffffffff", 9)
        .unwrap();
    lfs.unmount(&bd, &config).unwrap();

    lfs.mount(&bd, &config).unwrap();
    let n = lfs
        .getattr(&bd, &config, "hello", b'B', &mut buffer[4..13])
        .unwrap();
    assert_eq!(n, 9);
    assert_eq!(&buffer[4..13], b"fffffffff");

    let mut file = lfs
        .file_open(
            &bd,
            &config,
            "hello/hello",
            OpenFlags::new(OpenFlags::RDONLY),
        )
        .unwrap();
    let n = lfs.file_read(&bd, &config, &mut file, &mut buffer).unwrap();
    assert_eq!(n, 5);
    assert_eq!(&buffer[..5], b"hello");
    lfs.file_close(&bd, &config, file).unwrap();
    lfs.unmount(&bd, &config).unwrap();
}

// --- test_attrs_get_set_root ---
#[test]
#[ignore = "getattr/setattr/removeattr not implemented"]
fn test_attrs_get_set_root() {
    let (bd, config, mut lfs) = fresh_fs();

    lfs.mkdir(&bd, &config, "hello").unwrap();
    let mut file = lfs
        .file_open(
            &bd,
            &config,
            "hello/hello",
            OpenFlags::new(OpenFlags::WRONLY | OpenFlags::CREAT),
        )
        .unwrap();
    lfs.file_write(&bd, &config, &mut file, b"hello").unwrap();
    lfs.file_close(&bd, &config, file).unwrap();
    lfs.unmount(&bd, &config).unwrap();

    lfs.mount(&bd, &config).unwrap();
    let mut buffer = [0u8; 1024];

    lfs.setattr(&bd, &config, "/", b'A', b"aaaa", 4).unwrap();
    lfs.setattr(&bd, &config, "/", b'B', b"bbbbbb", 6).unwrap();
    lfs.setattr(&bd, &config, "/", b'C', b"ccccc", 5).unwrap();

    let n = lfs
        .getattr(&bd, &config, "/", b'A', &mut buffer[..4])
        .unwrap();
    assert_eq!(n, 4);
    let n = lfs
        .getattr(&bd, &config, "/", b'B', &mut buffer[4..10])
        .unwrap();
    assert_eq!(n, 6);
    let n = lfs
        .getattr(&bd, &config, "/", b'C', &mut buffer[10..15])
        .unwrap();
    assert_eq!(n, 5);
    assert_eq!(&buffer[0..4], b"aaaa");
    assert_eq!(&buffer[4..10], b"bbbbbb");
    assert_eq!(&buffer[10..15], b"ccccc");

    lfs.setattr(&bd, &config, "/", b'B', b"", 0).unwrap();
    lfs.removeattr(&bd, &config, "/", b'B').unwrap();
    lfs.setattr(&bd, &config, "/", b'B', b"fffffffff", 9)
        .unwrap();
    lfs.unmount(&bd, &config).unwrap();

    lfs.mount(&bd, &config).unwrap();
    let mut buffer = [0u8; 1024];
    let n = lfs
        .getattr(&bd, &config, "/", b'A', &mut buffer[..4])
        .unwrap();
    assert_eq!(n, 4);
    let n = lfs
        .getattr(&bd, &config, "/", b'B', &mut buffer[4..13])
        .unwrap();
    assert_eq!(n, 9);
    assert_eq!(&buffer[4..13], b"fffffffff");

    let mut file = lfs
        .file_open(
            &bd,
            &config,
            "hello/hello",
            OpenFlags::new(OpenFlags::RDONLY),
        )
        .unwrap();
    let n = lfs.file_read(&bd, &config, &mut file, &mut buffer).unwrap();
    assert_eq!(n, 5);
    assert_eq!(&buffer[..5], b"hello");
    lfs.file_close(&bd, &config, file).unwrap();
    lfs.unmount(&bd, &config).unwrap();
}

// --- test_attrs_get_set_file ---
// Uses lfs_file_opencfg with attrs; attrs not implemented.
#[test]
#[ignore = "getattr/setattr/removeattr and file_opencfg attrs not implemented"]
fn test_attrs_get_set_file() {
    let (bd, config, mut lfs) = fresh_fs();

    lfs.mkdir(&bd, &config, "hello").unwrap();
    let mut file = lfs
        .file_open(
            &bd,
            &config,
            "hello/hello",
            OpenFlags::new(OpenFlags::WRONLY | OpenFlags::CREAT),
        )
        .unwrap();
    lfs.file_write(&bd, &config, &mut file, b"hello").unwrap();
    lfs.file_close(&bd, &config, file).unwrap();
    lfs.unmount(&bd, &config).unwrap();

    lfs.mount(&bd, &config).unwrap();
    let mut buffer = [0u8; 1024];
    buffer[0..4].copy_from_slice(b"aaaa");
    buffer[4..10].copy_from_slice(b"bbbbbb");
    buffer[10..15].copy_from_slice(b"ccccc");

    let cfg = FileOpenConfig { attr_count: 3 };
    let file = lfs
        .file_opencfg(
            &bd,
            &config,
            "hello/hello",
            OpenFlags::new(OpenFlags::WRONLY),
            &cfg,
        )
        .unwrap();
    lfs.file_close(&bd, &config, file).unwrap();

    buffer.fill(0);
    let cfg = FileOpenConfig { attr_count: 3 };
    let file = lfs
        .file_opencfg(
            &bd,
            &config,
            "hello/hello",
            OpenFlags::new(OpenFlags::RDONLY),
            &cfg,
        )
        .unwrap();
    lfs.file_close(&bd, &config, file).unwrap();
    assert_eq!(&buffer[0..4], b"aaaa");
    assert_eq!(&buffer[4..10], b"bbbbbb");
    assert_eq!(&buffer[10..15], b"ccccc");

    lfs.unmount(&bd, &config).unwrap();

    lfs.mount(&bd, &config).unwrap();
    let mut file = lfs
        .file_open(
            &bd,
            &config,
            "hello/hello",
            OpenFlags::new(OpenFlags::RDONLY),
        )
        .unwrap();
    let n = lfs.file_read(&bd, &config, &mut file, &mut buffer).unwrap();
    assert_eq!(n, 5);
    assert_eq!(&buffer[..5], b"hello");
    lfs.file_close(&bd, &config, file).unwrap();
    lfs.unmount(&bd, &config).unwrap();
}

// --- test_attrs_deferred_file ---
// Uses lfs_file_opencfg with deferred attrs (synced on file_sync).
#[test]
#[ignore = "getattr/setattr/removeattr and file_opencfg attrs not implemented"]
fn test_attrs_deferred_file() {
    let (bd, config, mut lfs) = fresh_fs();

    lfs.mkdir(&bd, &config, "hello").unwrap();
    let mut file = lfs
        .file_open(
            &bd,
            &config,
            "hello/hello",
            OpenFlags::new(OpenFlags::WRONLY | OpenFlags::CREAT),
        )
        .unwrap();
    lfs.file_write(&bd, &config, &mut file, b"hello").unwrap();
    lfs.file_close(&bd, &config, file).unwrap();
    lfs.unmount(&bd, &config).unwrap();

    lfs.mount(&bd, &config).unwrap();
    lfs.setattr(&bd, &config, "hello/hello", b'B', b"fffffffff", 9)
        .unwrap();
    lfs.setattr(&bd, &config, "hello/hello", b'C', b"ccccc", 5)
        .unwrap();

    let mut buffer = [0u8; 1024];
    let n = lfs
        .getattr(&bd, &config, "hello/hello", b'B', &mut buffer[..9])
        .unwrap();
    assert_eq!(n, 9);
    assert_eq!(&buffer[..9], b"fffffffff");

    let cfg = FileOpenConfig {
        attr_count: 3, // B=gggg, C=empty, D=hhhh
    };
    let mut file = lfs
        .file_opencfg(
            &bd,
            &config,
            "hello/hello",
            OpenFlags::new(OpenFlags::WRONLY),
            &cfg,
        )
        .unwrap();

    lfs.file_sync(&bd, &config, &mut file).unwrap();

    let n = lfs
        .getattr(&bd, &config, "hello/hello", b'B', &mut buffer[..9])
        .unwrap();
    assert_eq!(n, 4);
    assert_eq!(&buffer[..9], b"gggg\0\0\0\0\0");

    lfs.file_close(&bd, &config, file).unwrap();
    lfs.unmount(&bd, &config).unwrap();
}
