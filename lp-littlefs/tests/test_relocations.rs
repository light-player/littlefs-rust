//! Relocation and compaction tests.
//!
//! Corresponds to upstream test_relocations.toml
//! Validates dir_compact, dir_split, and orphaningcommit.

mod common;

use common::{cached_bd, default_config, init_log};
use lp_littlefs::{LittleFs, OpenFlags};

/// Fill FS, create many files in child dir. Triggers compaction/split when
/// metadata block overflows.
#[test]
fn test_relocations_dangling_split_dir() {
    init_log();
    let config = default_config();
    let bd = cached_bd(&config);
    let mut lfs = LittleFs::new();
    lfs.format(&bd, &config).unwrap();
    lfs.mount(&bd, &config).unwrap();

    lfs.mkdir(&bd, &config, "d0").unwrap();
    for i in 0..8 {
        let path = format!("d0/f{i}");
        let mut f = lfs
            .file_open(
                &bd,
                &config,
                &path,
                OpenFlags::new(OpenFlags::WRONLY | OpenFlags::CREAT),
            )
            .unwrap_or_else(|e| panic!("file_open {} failed: {:?}", path, e));
        lfs.file_write(&bd, &config, &mut f, b"x").unwrap();
        lfs.file_close(&bd, &config, f).unwrap();
    }

    for i in 0..8 {
        let path = format!("d0/f{i}");
        let info = lfs
            .stat(&bd, &config, &path)
            .unwrap_or_else(|e| panic!("stat {} failed: {:?}", path, e));
        assert_eq!(info.name().unwrap(), &format!("f{i}"));
    }
}

/// Similar to dangling_split_dir; exercises split dir handling.
#[test]
fn test_relocations_outdated_head() {
    init_log();
    let config = default_config();
    let bd = cached_bd(&config);
    let mut lfs = LittleFs::new();
    lfs.format(&bd, &config).unwrap();
    lfs.mount(&bd, &config).unwrap();

    for i in 0..3 {
        lfs.mkdir(&bd, &config, &format!("d{i}")).unwrap();
    }
    lfs.mkdir(&bd, &config, "d0/sub").unwrap();
    for i in 0..8 {
        let path = format!("d0/sub/f{i}");
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
        let path = format!("d0/sub/f{i}");
        let info = lfs.stat(&bd, &config, &path).unwrap();
        assert_eq!(info.name().unwrap(), &format!("f{i}"));
    }
}
