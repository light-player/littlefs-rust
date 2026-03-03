//! Superblock and format/mount tests.
//!
//! Corresponds to upstream test_superblocks.toml
//! Source: https://github.com/littlefs-project/littlefs/blob/master/tests/test_superblocks.toml

mod common;

use common::{default_config, ram_bd};
use lp_littlefs::{BlockDevice, FsInfo, LittleFs, MAGIC, MAGIC_OFFSET};

// --- test_superblocks_format ---
// Upstream: lfs_format(&lfs, cfg) => 0
#[test]
fn test_superblocks_format() {
    let config = default_config();
    let bd = ram_bd(&config);
    let mut lfs = LittleFs::new();
    lfs.format(&bd, &config).unwrap();
}

// --- test_superblocks_mount ---
// Upstream: format, mount, unmount
#[test]
fn test_superblocks_mount() {
    let config = default_config();
    let bd = ram_bd(&config);
    let mut lfs = LittleFs::new();
    lfs.format(&bd, &config).unwrap();
    lfs.mount(&bd, &config).unwrap();
    lfs.unmount(&bd, &config).unwrap();
}

// --- test_fs_size_traverse ---
// Phase 06: fs_size and fs_traverse APIs
#[test]
fn test_fs_size_traverse() {
    let config = default_config();
    let bd = ram_bd(&config);
    let mut lfs = LittleFs::new();
    lfs.format(&bd, &config).unwrap();
    lfs.mount(&bd, &config).unwrap();

    let size = lfs.fs_size(&bd, &config).unwrap();
    assert!(
        size >= 2,
        "fs_size should count at least root pair (2 blocks)"
    );

    let mut count = 0u32;
    lfs.fs_traverse(&bd, &config, |_block| {
        count += 1;
        Ok(())
    })
    .unwrap();
    assert!(count >= 2);
}

// --- test_fs_mkconsistent ---
// Phase 06: fs_mkconsistent persists gstate; remount succeeds
#[test]
fn test_fs_mkconsistent() {
    let config = default_config();
    let bd = ram_bd(&config);
    let mut lfs = LittleFs::new();
    lfs.format(&bd, &config).unwrap();
    lfs.mount(&bd, &config).unwrap();
    lfs.mkdir(&bd, &config, "d0").unwrap();
    lfs.fs_mkconsistent(&bd, &config).unwrap();
    lfs.unmount(&bd, &config).unwrap();

    lfs.mount(&bd, &config).unwrap();
    let info = lfs.stat(&bd, &config, "d0").unwrap();
    assert_eq!(info.name().unwrap(), "d0");
}

// --- test_superblocks_magic ---
// Upstream: format, then raw read to verify "littlefs" at MAGIC_OFFSET in both blocks
#[test]
fn test_superblocks_magic() {
    let config = default_config();
    let bd = ram_bd(&config);
    let mut lfs = LittleFs::new();
    lfs.format(&bd, &config).unwrap();

    let mut buf = [0u8; 24];
    bd.read(0, 0, &mut buf).unwrap();
    assert_eq!(&buf[MAGIC_OFFSET as usize..][..8], MAGIC);
    bd.read(1, 0, &mut buf).unwrap();
    assert_eq!(&buf[MAGIC_OFFSET as usize..][..8], MAGIC);
}

// --- test_superblocks_invalid_mount ---
// Upstream: mount on blank device => LFS_ERR_CORRUPT
#[test]
fn test_superblocks_invalid_mount() {
    let config = default_config();
    let bd = ram_bd(&config);
    let mut lfs = LittleFs::new();
    let err = lfs.mount(&bd, &config).unwrap_err();
    assert!(matches!(err, lp_littlefs::Error::Corrupt));
}

// --- test_superblocks_stat ---
// Upstream: fs_stat after format/mount returns correct values
#[test]
fn test_superblocks_stat() {
    let config = default_config();
    let bd = ram_bd(&config);
    let mut lfs = LittleFs::new();
    lfs.format(&bd, &config).unwrap();
    lfs.mount(&bd, &config).unwrap();

    let info: FsInfo = lfs.fs_stat(&bd, &config).unwrap();
    assert_eq!(info.block_size, config.block_size);
    assert_eq!(info.block_count, config.block_count);
    assert_eq!(info.disk_version, 0x0002_0001);
    assert_eq!(info.name_max, 255);
    assert_eq!(info.file_max, 2_147_483_647);
    assert_eq!(info.attr_max, 1022);
}
