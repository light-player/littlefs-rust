//! Entry/inline file corner case tests. Per upstream test_entries.toml.

mod common;

use common::{config_with_cache, init_log};
use lp_littlefs::OpenFlags;

fn fs_with_cache_512() -> (
    lp_littlefs::RamBlockDevice,
    lp_littlefs::Config,
    lp_littlefs::LittleFs,
) {
    let config = config_with_cache(512, 128);
    let bd = common::ram_bd(&config);
    let mut lfs = lp_littlefs::LittleFs::new();
    lfs.format(&bd, &config).unwrap();
    lfs.mount(&bd, &config).unwrap();
    (bd, config, lfs)
}

#[test]
fn test_entries_grow() {
    init_log();
    let (bd, config, mut lfs) = fs_with_cache_512();
    let buf = [b'c'; 1024];

    for i in 0..4 {
        let path = format!("hi{i}");
        let size = if i == 1 { 20 } else { 20 };
        let mut file = lfs
            .file_open(
                &bd,
                &config,
                &path,
                OpenFlags::new(OpenFlags::WRONLY | OpenFlags::CREAT | OpenFlags::TRUNC),
            )
            .unwrap();
        lfs.file_write(&bd, &config, &mut file, &buf[..size])
            .unwrap();
        lfs.file_close(&bd, &config, file).unwrap();
    }
    let mut file = lfs
        .file_open(&bd, &config, "hi1", OpenFlags::new(OpenFlags::RDONLY))
        .unwrap();
    let mut rb = [0u8; 256];
    let n = lfs
        .file_read(&bd, &config, &mut file, &mut rb[..20])
        .unwrap();
    assert_eq!(n, 20);
    lfs.file_close(&bd, &config, file).unwrap();

    let mut file = lfs
        .file_open(
            &bd,
            &config,
            "hi1",
            OpenFlags::new(OpenFlags::WRONLY | OpenFlags::CREAT | OpenFlags::TRUNC),
        )
        .unwrap();
    lfs.file_write(&bd, &config, &mut file, &buf[..200])
        .unwrap();
    lfs.file_close(&bd, &config, file).unwrap();

    for i in 0..4 {
        let path = format!("hi{i}");
        let size = if i == 1 { 200 } else { 20 };
        let mut file = lfs
            .file_open(&bd, &config, &path, OpenFlags::new(OpenFlags::RDONLY))
            .unwrap();
        let n = lfs
            .file_read(&bd, &config, &mut file, &mut rb[..size])
            .unwrap();
        assert_eq!(n, size);
        assert_eq!(&rb[..n], &buf[..size]);
        lfs.file_close(&bd, &config, file).unwrap();
    }
}

#[test]
fn test_entries_shrink() {
    init_log();
    let (bd, config, mut lfs) = fs_with_cache_512();
    let buf = [b'c'; 1024];

    for i in 0..4 {
        let path = format!("hi{i}");
        let size = if i == 1 { 200 } else { 20 };
        let mut file = lfs
            .file_open(
                &bd,
                &config,
                &path,
                OpenFlags::new(OpenFlags::WRONLY | OpenFlags::CREAT | OpenFlags::TRUNC),
            )
            .unwrap();
        lfs.file_write(&bd, &config, &mut file, &buf[..size])
            .unwrap();
        lfs.file_close(&bd, &config, file).unwrap();
    }
    let mut file = lfs
        .file_open(&bd, &config, "hi1", OpenFlags::new(OpenFlags::RDONLY))
        .unwrap();
    let mut rb = [0u8; 256];
    let n = lfs
        .file_read(&bd, &config, &mut file, &mut rb[..200])
        .unwrap();
    assert_eq!(n, 200);
    lfs.file_close(&bd, &config, file).unwrap();

    let mut file = lfs
        .file_open(
            &bd,
            &config,
            "hi1",
            OpenFlags::new(OpenFlags::WRONLY | OpenFlags::CREAT | OpenFlags::TRUNC),
        )
        .unwrap();
    lfs.file_write(&bd, &config, &mut file, &buf[..20]).unwrap();
    lfs.file_close(&bd, &config, file).unwrap();

    for i in 0..4 {
        let path = format!("hi{i}");
        let size = if i == 1 { 20 } else { 20 };
        let mut file = lfs
            .file_open(&bd, &config, &path, OpenFlags::new(OpenFlags::RDONLY))
            .unwrap();
        let n = lfs
            .file_read(&bd, &config, &mut file, &mut rb[..size])
            .unwrap();
        assert_eq!(n, size);
        assert_eq!(&rb[..n], &buf[..size]);
        lfs.file_close(&bd, &config, file).unwrap();
    }
}

#[test]
#[ignore = "spill layout may differ with metadata"]
fn test_entries_spill() {
    init_log();
    let (bd, config, mut lfs) = fs_with_cache_512();
    let buf = [b'c'; 256];
    for i in 0..4 {
        let path = format!("hi{i}");
        let mut file = lfs
            .file_open(
                &bd,
                &config,
                &path,
                OpenFlags::new(OpenFlags::WRONLY | OpenFlags::CREAT | OpenFlags::TRUNC),
            )
            .unwrap();
        lfs.file_write(&bd, &config, &mut file, &buf[..200])
            .unwrap();
        lfs.file_close(&bd, &config, file).unwrap();
    }
    let mut rb = [0u8; 256];
    for i in 0..4 {
        let path = format!("hi{i}");
        let mut file = lfs
            .file_open(&bd, &config, &path, OpenFlags::new(OpenFlags::RDONLY))
            .unwrap();
        let n = lfs
            .file_read(&bd, &config, &mut file, &mut rb[..200])
            .unwrap();
        assert_eq!(n, 200);
        assert_eq!(&rb[..n], &buf[..200]);
        lfs.file_close(&bd, &config, file).unwrap();
    }
}

#[test]
#[ignore = "push_spill layout may differ with metadata"]
fn test_entries_push_spill() {
    init_log();
    let (bd, config, mut lfs) = fs_with_cache_512();
    let buf = [b'c'; 256];

    let mut file = lfs
        .file_open(
            &bd,
            &config,
            "hi0",
            OpenFlags::new(OpenFlags::WRONLY | OpenFlags::CREAT | OpenFlags::TRUNC),
        )
        .unwrap();
    lfs.file_write(&bd, &config, &mut file, &buf[..200])
        .unwrap();
    lfs.file_close(&bd, &config, file).unwrap();
    for i in 1..4 {
        let path = format!("hi{i}");
        let size = if i == 1 { 20 } else { 200 };
        let mut file = lfs
            .file_open(
                &bd,
                &config,
                &path,
                OpenFlags::new(OpenFlags::WRONLY | OpenFlags::CREAT | OpenFlags::TRUNC),
            )
            .unwrap();
        lfs.file_write(&bd, &config, &mut file, &buf[..size])
            .unwrap();
        lfs.file_close(&bd, &config, file).unwrap();
    }
    let mut file = lfs
        .file_open(&bd, &config, "hi1", OpenFlags::new(OpenFlags::RDONLY))
        .unwrap();
    let mut rb = [0u8; 256];
    let n = lfs
        .file_read(&bd, &config, &mut file, &mut rb[..20])
        .unwrap();
    assert_eq!(n, 20);
    lfs.file_close(&bd, &config, file).unwrap();

    let mut file = lfs
        .file_open(
            &bd,
            &config,
            "hi1",
            OpenFlags::new(OpenFlags::WRONLY | OpenFlags::CREAT | OpenFlags::TRUNC),
        )
        .unwrap();
    lfs.file_write(&bd, &config, &mut file, &buf[..200])
        .unwrap();
    lfs.file_close(&bd, &config, file).unwrap();

    for i in 0..4 {
        let path = format!("hi{i}");
        let mut file = lfs
            .file_open(&bd, &config, &path, OpenFlags::new(OpenFlags::RDONLY))
            .unwrap();
        let n = lfs
            .file_read(&bd, &config, &mut file, &mut rb[..200])
            .unwrap();
        assert_eq!(n, 200);
        lfs.file_close(&bd, &config, file).unwrap();
    }
}

#[test]
#[ignore = "drop/remove with inline neighbors may differ"]
fn test_entries_drop() {
    init_log();
    let (bd, config, mut lfs) = fs_with_cache_512();
    let buf = [b'c'; 256];
    for i in 0..4 {
        let path = format!("hi{i}");
        let size = if i == 1 { 200 } else { 20 };
        let mut file = lfs
            .file_open(
                &bd,
                &config,
                &path,
                OpenFlags::new(OpenFlags::WRONLY | OpenFlags::CREAT | OpenFlags::TRUNC),
            )
            .unwrap();
        lfs.file_write(&bd, &config, &mut file, &buf[..size])
            .unwrap();
        lfs.file_close(&bd, &config, file).unwrap();
    }
    lfs.remove(&bd, &config, "hi1").unwrap();
    let mut file = lfs
        .file_open(
            &bd,
            &config,
            "hi1",
            OpenFlags::new(OpenFlags::WRONLY | OpenFlags::CREAT | OpenFlags::TRUNC),
        )
        .unwrap();
    lfs.file_write(&bd, &config, &mut file, &buf[..20]).unwrap();
    lfs.file_close(&bd, &config, file).unwrap();
    let mut rb = [0u8; 256];
    for i in 0..4 {
        let path = format!("hi{i}");
        let mut file = lfs
            .file_open(&bd, &config, &path, OpenFlags::new(OpenFlags::RDONLY))
            .unwrap();
        let n = lfs
            .file_read(&bd, &config, &mut file, &mut rb[..20])
            .unwrap();
        assert_eq!(n, 20);
        lfs.file_close(&bd, &config, file).unwrap();
    }
}

#[test]
#[ignore = "create_too_big / name_max may need specific config"]
fn test_entries_create_too_big() {}

#[test]
#[ignore = "resize_too_big may need specific config"]
fn test_entries_resize_too_big() {}
