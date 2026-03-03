//! Shared test helpers for lp-littlefs integration tests.
//!
//! Reduces duplication across test files. Use `mod common; use common::*;` in each test.
//! Each test binary uses a subset of helpers; allow dead_code for the rest.
#![allow(dead_code)]

use lp_littlefs::{
    create_inline_file, BlockDevice, Config, Dir, Error, FileType, Info, LittleFs, RamBlockDevice,
};

/// Default test config. Block count 128.
pub fn default_config() -> Config {
    Config::default_for_tests(128)
}

/// RAM block device for tests. Cache is internal to the FS.
pub fn ram_bd(config: &Config) -> RamBlockDevice {
    RamBlockDevice::new(config.block_size, config.block_count)
}

/// Alias for ram_bd. For tests that need raw access (e.g. power-loss simulation).
pub fn uncached_bd(config: &Config) -> RamBlockDevice {
    ram_bd(config)
}

/// Initialize logger for trace output.
/// Run tests with: RUST_LOG=lp_littlefs=trace cargo test --features trace
pub fn init_log() {
    let _ = env_logger::builder().is_test(true).try_init();
}

/// Format and mount a fresh FS. Returns (bd, config, fs).
pub fn fresh_fs() -> (RamBlockDevice, Config, LittleFs) {
    let config = default_config();
    let bd = ram_bd(&config);
    let mut lfs = LittleFs::new();
    lfs.format(&bd, &config).unwrap();
    lfs.mount(&bd, &config).unwrap();
    (bd, config, lfs)
}

/// Format, create inline "hello" file, and mount. Returns (bd, config, fs).
pub fn fs_with_hello() -> (RamBlockDevice, Config, LittleFs) {
    let config = default_config();
    let bd = ram_bd(&config);
    let mut fs = LittleFs::new();

    fs.format(&bd, &config).unwrap();
    create_inline_file(&bd, &config, "hello", b"Hello World!\0").unwrap();
    fs.mount(&bd, &config).unwrap();

    (bd, config, fs)
}

/// Open dir at path, skip ".", "..", collect entry names. Returns names in order.
pub fn dir_entry_names<B: BlockDevice>(
    lfs: &mut LittleFs,
    bd: &B,
    config: &Config,
    path: &str,
) -> Result<Vec<String>, Error> {
    let mut dir: Dir = lfs.dir_open(bd, config, path)?;
    let mut info = Info::new(FileType::Reg, 0);

    let _ = lfs.dir_read(bd, config, &mut dir, &mut info)?;
    let _ = lfs.dir_read(bd, config, &mut dir, &mut info)?;

    let mut names = Vec::new();
    loop {
        let n = lfs.dir_read(bd, config, &mut dir, &mut info)?;
        if n == 0 {
            break;
        }
        names.push(info.name().unwrap().to_string());
    }
    Ok(names)
}
