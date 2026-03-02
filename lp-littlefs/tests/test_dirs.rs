//! Directory iteration tests.
//!
//! Corresponds to upstream test_dirs.toml
//! Source: https://github.com/littlefs-project/littlefs/blob/master/tests/test_dirs.toml

use lp_littlefs::{CachedBlockDevice, Config, Dir, FileType, Info, LittleFs, RamBlockDevice};

fn default_config() -> Config {
    Config::default_for_tests(128)
}

fn cached_bd(config: &Config) -> CachedBlockDevice<RamBlockDevice> {
    let ram = RamBlockDevice::new(config.block_size, config.block_count);
    CachedBlockDevice::new(ram, config).unwrap()
}

// --- test_dirs_root ---
// Upstream: dir_open("/"), dir_read returns ".", "..", then 0
#[test]
fn test_dirs_root() {
    let config = default_config();
    let bd = cached_bd(&config);
    let mut lfs = LittleFs::new();
    lfs.format(&bd, &config).unwrap();
    lfs.mount(&bd, &config).unwrap();

    let mut dir: Dir = lfs.dir_open(&bd, &config, "/").unwrap();
    let mut info = Info::new(FileType::Reg, 0);

    let n = lfs.dir_read(&bd, &config, &mut dir, &mut info).unwrap();
    assert_eq!(n, 1);
    assert_eq!(info.name().unwrap(), ".");
    assert!(matches!(info.typ, FileType::Dir));

    let n = lfs.dir_read(&bd, &config, &mut dir, &mut info).unwrap();
    assert_eq!(n, 1);
    assert_eq!(info.name().unwrap(), "..");
    assert!(matches!(info.typ, FileType::Dir));

    let n = lfs.dir_read(&bd, &config, &mut dir, &mut info).unwrap();
    assert_eq!(n, 0);
}
