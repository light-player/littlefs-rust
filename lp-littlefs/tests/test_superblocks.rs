//! Superblock and format/mount tests.
//!
//! Corresponds to upstream test_superblocks.toml
//! Source: https://github.com/littlefs-project/littlefs/blob/master/tests/test_superblocks.toml

use lp_littlefs::{BlockDevice, Config, LittleFs, RamBlockDevice, MAGIC, MAGIC_OFFSET};

fn default_config() -> Config {
    Config::default_for_tests(128)
}

// --- test_superblocks_format ---
// Upstream: lfs_format(&lfs, cfg) => 0
#[test]
fn test_superblocks_format() {
    let config = default_config();
    let bd = RamBlockDevice::new(config.block_size, config.block_count);
    let mut lfs = LittleFs::new();
    lfs.format(&bd, &config).unwrap();
}

// --- test_superblocks_mount ---
// Upstream: format, mount, unmount
#[test]
fn test_superblocks_mount() {
    let config = default_config();
    let bd = RamBlockDevice::new(config.block_size, config.block_count);
    let mut lfs = LittleFs::new();
    lfs.format(&bd, &config).unwrap();
    lfs.mount(&bd, &config).unwrap();
    lfs.unmount().unwrap();
}

// --- test_superblocks_magic ---
// Upstream: format, then raw read to verify "littlefs" at MAGIC_OFFSET in both blocks
#[test]
fn test_superblocks_magic() {
    let config = default_config();
    let bd = RamBlockDevice::new(config.block_size, config.block_count);
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
    let bd = RamBlockDevice::new(config.block_size, config.block_count);
    let mut lfs = LittleFs::new();
    let err = lfs.mount(&bd, &config).unwrap_err();
    assert!(matches!(err, lp_littlefs::Error::Corrupt));
}
