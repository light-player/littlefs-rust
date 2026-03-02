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

// --- test_dirs_one_mkdir ---
#[test]
fn test_dirs_one_mkdir() {
    let config = default_config();
    let bd = cached_bd(&config);
    let mut lfs = LittleFs::new();
    lfs.format(&bd, &config).unwrap();
    lfs.mount(&bd, &config).unwrap();

    lfs.mkdir(&bd, &config, "d0").unwrap();

    let info = lfs.stat(&bd, &config, "d0").unwrap();
    assert_eq!(info.name().unwrap(), "d0");
    assert!(matches!(info.typ, FileType::Dir));

    let mut dir = lfs.dir_open(&bd, &config, "/").unwrap();
    let mut info = Info::new(FileType::Reg, 0);
    let _ = lfs.dir_read(&bd, &config, &mut dir, &mut info).unwrap();
    let _ = lfs.dir_read(&bd, &config, &mut dir, &mut info).unwrap();
    let n = lfs.dir_read(&bd, &config, &mut dir, &mut info).unwrap();
    assert_eq!(n, 1, "expected d0 entry");
    assert_eq!(info.name().unwrap(), "d0");
}

// --- test_dirs_many_creation ---
// Upstream: mkdir N dirs, dir_read lists them
#[test]
fn test_dirs_many_creation() {
    let config = default_config();
    let bd = cached_bd(&config);
    let mut lfs = LittleFs::new();
    lfs.format(&bd, &config).unwrap();
    lfs.mount(&bd, &config).unwrap();

    for i in 0..5 {
        lfs.mkdir(&bd, &config, &format!("d{i}"))
            .unwrap_or_else(|e| panic!("mkdir d{i} failed: {e:?}"));
    }

    let mut dir = lfs.dir_open(&bd, &config, "/").unwrap();
    let mut info = Info::new(FileType::Reg, 0);

    let _ = lfs.dir_read(&bd, &config, &mut dir, &mut info).unwrap();
    assert_eq!(info.name().unwrap(), ".");
    let _ = lfs.dir_read(&bd, &config, &mut dir, &mut info).unwrap();
    assert_eq!(info.name().unwrap(), "..");

    let mut names: Vec<String> = Vec::new();
    loop {
        let n = lfs.dir_read(&bd, &config, &mut dir, &mut info).unwrap();
        if n == 0 {
            break;
        }
        names.push(info.name().unwrap().to_string());
    }

    assert_eq!(names.len(), 5);
    assert_eq!(names, ["d0", "d1", "d2", "d3", "d4"]);
}

// --- test_dirs_many_removal ---
// Upstream: mkdir N, remove all, dir_read empty
#[test]
fn test_dirs_many_removal() {
    let config = default_config();
    let bd = cached_bd(&config);
    let mut lfs = LittleFs::new();
    lfs.format(&bd, &config).unwrap();
    lfs.mount(&bd, &config).unwrap();

    for i in 0..5 {
        lfs.mkdir(&bd, &config, &format!("d{i}")).unwrap();
    }

    for i in (0..5).rev() {
        lfs.remove(&bd, &config, &format!("d{i}"))
            .unwrap_or_else(|e| panic!("remove d{i} failed: {e:?}"));
    }

    let mut dir = lfs.dir_open(&bd, &config, "/").unwrap();
    let mut info = Info::new(FileType::Reg, 0);
    let _ = lfs.dir_read(&bd, &config, &mut dir, &mut info).unwrap();
    let _ = lfs.dir_read(&bd, &config, &mut dir, &mut info).unwrap();
    let n = lfs.dir_read(&bd, &config, &mut dir, &mut info).unwrap();
    assert_eq!(n, 0);
}

// --- test_dirs_one_rename ---
#[test]
fn test_dirs_one_rename() {
    let config = default_config();
    let bd = cached_bd(&config);
    let mut lfs = LittleFs::new();
    lfs.format(&bd, &config).unwrap();
    lfs.mount(&bd, &config).unwrap();

    lfs.mkdir(&bd, &config, "d0").unwrap();
    lfs.rename(&bd, &config, "d0", "x0")
        .unwrap_or_else(|e| panic!("rename failed: {e:?}"));

    let info = lfs.stat(&bd, &config, "x0").unwrap();
    assert_eq!(info.name().unwrap(), "x0");

    let mut dir = lfs.dir_open(&bd, &config, "/").unwrap();
    let mut info = Info::new(FileType::Reg, 0);
    let _ = lfs.dir_read(&bd, &config, &mut dir, &mut info).unwrap();
    let _ = lfs.dir_read(&bd, &config, &mut dir, &mut info).unwrap();
    let n = lfs.dir_read(&bd, &config, &mut dir, &mut info).unwrap();
    assert_eq!(n, 1);
    assert_eq!(info.name().unwrap(), "x0");
}

// --- test_dirs_many_rename ---
// Upstream: mkdir N, rename each, verify
// Ignored: rename with multiple entries needs further investigation
#[test]
fn test_dirs_many_rename() {
    let config = default_config();
    let bd = cached_bd(&config);
    let mut lfs = LittleFs::new();
    lfs.format(&bd, &config).unwrap();
    lfs.mount(&bd, &config).unwrap();

    for i in 0..5 {
        lfs.mkdir(&bd, &config, &format!("d{i}")).unwrap();
    }

    for i in 0..5 {
        lfs.rename(&bd, &config, &format!("d{i}"), &format!("x{i}"))
            .unwrap_or_else(|e| panic!("rename d{i} -> x{i} failed: {e:?}"));
    }

    let mut dir = lfs.dir_open(&bd, &config, "/").unwrap();
    let mut info = Info::new(FileType::Reg, 0);
    let _ = lfs.dir_read(&bd, &config, &mut dir, &mut info).unwrap();
    let _ = lfs.dir_read(&bd, &config, &mut dir, &mut info).unwrap();

    let mut names: Vec<String> = Vec::new();
    loop {
        let n = lfs.dir_read(&bd, &config, &mut dir, &mut info).unwrap();
        if n == 0 {
            break;
        }
        names.push(info.name().unwrap().to_string());
    }

    assert_eq!(names.len(), 5);
    assert_eq!(names, ["x0", "x1", "x2", "x3", "x4"]);
}
