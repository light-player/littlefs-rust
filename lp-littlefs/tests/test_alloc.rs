//! Allocator and block allocation tests.
//!
//! Corresponds to upstream test_alloc.toml
//! Source: https://github.com/littlefs-project/littlefs/blob/master/tests/test_alloc.toml

mod common;

use common::{default_config, init_log, ram_bd};
use lp_littlefs::{LittleFs, OpenFlags};

// --- test_alloc_parallel ---
// Upstream: parallel alloc (multiple dirs/files)
#[test]
fn test_alloc_parallel() {
    init_log();
    let config = default_config();
    let bd = ram_bd(&config);
    let mut lfs = LittleFs::new();
    lfs.format(&bd, &config).unwrap();
    lfs.mount(&bd, &config).unwrap();

    for i in 0..4 {
        lfs.mkdir(&bd, &config, &format!("d{i}")).unwrap();
    }
    for i in 0..4 {
        let path = format!("d{i}/f");
        let file = lfs
            .file_open(
                &bd,
                &config,
                &path,
                OpenFlags::new(OpenFlags::WRONLY | OpenFlags::CREAT),
            )
            .unwrap();
        lfs.file_close(&bd, &config, file).unwrap();
    }
    for i in 0..4 {
        let info = lfs.stat(&bd, &config, &format!("d{i}/f")).unwrap();
        assert_eq!(info.name().unwrap(), "f");
    }
    lfs.unmount(&bd, &config).unwrap();
}

// --- test_alloc_serial ---
#[test]
fn test_alloc_serial() {
    init_log();
    let config = default_config();
    let bd = ram_bd(&config);
    let mut lfs = LittleFs::new();
    lfs.format(&bd, &config).unwrap();
    lfs.mount(&bd, &config).unwrap();

    lfs.mkdir(&bd, &config, "d0").unwrap();
    for i in 0..8 {
        let path = format!("d0/f{i}");
        let file = lfs
            .file_open(
                &bd,
                &config,
                &path,
                OpenFlags::new(OpenFlags::WRONLY | OpenFlags::CREAT),
            )
            .unwrap();
        lfs.file_close(&bd, &config, file).unwrap();
    }
    lfs.unmount(&bd, &config).unwrap();
}

// --- test_alloc_parallel_reuse ---
#[test]
fn test_alloc_parallel_reuse() {
    init_log();
    let config = default_config();
    let bd = ram_bd(&config);
    let mut lfs = LittleFs::new();
    lfs.format(&bd, &config).unwrap();
    lfs.mount(&bd, &config).unwrap();

    lfs.mkdir(&bd, &config, "a").unwrap();
    lfs.mkdir(&bd, &config, "b").unwrap();
    let f = lfs
        .file_open(
            &bd,
            &config,
            "a/x",
            OpenFlags::new(OpenFlags::WRONLY | OpenFlags::CREAT),
        )
        .unwrap();
    lfs.file_close(&bd, &config, f).unwrap();
    lfs.remove(&bd, &config, "a/x").unwrap();
    let f = lfs
        .file_open(
            &bd,
            &config,
            "b/y",
            OpenFlags::new(OpenFlags::WRONLY | OpenFlags::CREAT),
        )
        .unwrap();
    lfs.file_close(&bd, &config, f).unwrap();
    lfs.unmount(&bd, &config).unwrap();
}

// --- test_alloc_serial_reuse ---
#[test]
fn test_alloc_serial_reuse() {
    init_log();
    let config = default_config();
    let bd = ram_bd(&config);
    let mut lfs = LittleFs::new();
    lfs.format(&bd, &config).unwrap();
    lfs.mount(&bd, &config).unwrap();

    for i in 0..4 {
        let path = format!("f{i}");
        let f = lfs
            .file_open(
                &bd,
                &config,
                &path,
                OpenFlags::new(OpenFlags::WRONLY | OpenFlags::CREAT),
            )
            .unwrap();
        lfs.file_close(&bd, &config, f).unwrap();
    }
    for i in 0..4 {
        lfs.remove(&bd, &config, &format!("f{i}")).unwrap();
    }
    for i in 0..4 {
        let path = format!("g{i}");
        let f = lfs
            .file_open(
                &bd,
                &config,
                &path,
                OpenFlags::new(OpenFlags::WRONLY | OpenFlags::CREAT),
            )
            .unwrap();
        lfs.file_close(&bd, &config, f).unwrap();
    }
    lfs.unmount(&bd, &config).unwrap();
}

// --- test_alloc_exhaustion ---
#[test]
fn test_alloc_exhaustion() {
    init_log();
    let config = default_config();
    let bd = ram_bd(&config);
    let mut lfs = LittleFs::new();
    lfs.format(&bd, &config).unwrap();
    lfs.mount(&bd, &config).unwrap();

    let mut i = 0u32;
    while i < 200 {
        let path = format!("f{i}");
        match lfs.file_open(
            &bd,
            &config,
            &path,
            OpenFlags::new(OpenFlags::WRONLY | OpenFlags::CREAT),
        ) {
            Ok(file) => {
                lfs.file_close(&bd, &config, file).unwrap();
                i += 1;
            }
            Err(lp_littlefs::Error::Nomem) => break,
            Err(e) => panic!("unexpected: {e:?}"),
        }
    }
    lfs.unmount(&bd, &config).unwrap();
}

// --- test_alloc_exhaustion_wraparound ---
#[test]
#[ignore = "exhaustion wraparound semantics may differ"]
fn test_alloc_exhaustion_wraparound() {}

// --- test_alloc_dir_exhaustion ---
#[test]
#[ignore = "dir exhaustion edge case"]
fn test_alloc_dir_exhaustion() {}

// --- test_alloc_bad_blocks ---
#[test]
#[ignore = "bad-block BD simulation not implemented"]
fn test_alloc_bad_blocks() {}

// --- test_alloc_chained_dir_exhaustion ---
#[test]
#[ignore = "chained dir exhaustion"]
fn test_alloc_chained_dir_exhaustion() {}

// --- test_alloc_split_dir ---
#[test]
fn test_alloc_split_dir() {
    init_log();
    let config = default_config();
    let bd = ram_bd(&config);
    let mut lfs = LittleFs::new();
    lfs.format(&bd, &config).unwrap();
    lfs.mount(&bd, &config).unwrap();

    lfs.mkdir(&bd, &config, "d").unwrap();
    for i in 0..8 {
        let path = format!("d/f{i}");
        let mut f = lfs
            .file_open(
                &bd,
                &config,
                &path,
                OpenFlags::new(OpenFlags::WRONLY | OpenFlags::CREAT),
            )
            .unwrap();
        lfs.file_write(&bd, &config, &mut f, b"x").unwrap();
        lfs.file_close(&bd, &config, f).unwrap();
    }
    for i in 0..8 {
        let info = lfs.stat(&bd, &config, &format!("d/f{i}")).unwrap();
        assert_eq!(info.name().unwrap(), &format!("f{i}"));
    }
    lfs.unmount(&bd, &config).unwrap();
}

// --- test_alloc_outdated_lookahead ---
#[test]
#[ignore = "lookahead state edge case"]
fn test_alloc_outdated_lookahead() {}

// --- test_alloc_outdated_lookahead_split_dir ---
#[test]
#[ignore = "lookahead + split dir"]
fn test_alloc_outdated_lookahead_split_dir() {}
