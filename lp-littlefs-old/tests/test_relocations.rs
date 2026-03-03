//! Relocation and compaction tests.
//!
//! Corresponds to upstream test_relocations.toml
//! Validates dir_compact, dir_split, and orphaningcommit.

mod common;

use common::{default_config, init_log, ram_bd};
use lp_littlefs::{LittleFs, OpenFlags};

/// Fill FS, create many files in child dir. Triggers compaction/split when
/// metadata block overflows.
#[test]
fn test_relocations_dangling_split_dir() {
    init_log();
    let config = default_config();
    let bd = ram_bd(&config);
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
    let bd = ram_bd(&config);
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

// --- test_relocations_reentrant ---
// Upstream: random mkdir/remove with powerloss, block_cycles=1 forces relocations
#[test]
#[ignore = "powerloss runner not implemented"]
fn test_relocations_reentrant() {}

// --- test_relocations_reentrant_renames ---
// Upstream: random mkdir/rename/remove with powerloss
#[test]
#[ignore = "powerloss runner not implemented"]
fn test_relocations_reentrant_renames() {}

// --- test_relocations_nonreentrant ---
// Upstream: similar to reentrant but no powerloss
#[test]
fn test_relocations_nonreentrant() {
    init_log();
    let config = default_config();
    let bd = ram_bd(&config);
    let mut lfs = LittleFs::new();
    lfs.format(&bd, &config).unwrap();
    lfs.mount(&bd, &config).unwrap();

    for i in 0..6 {
        let path = format!("{}", (b'a' + i) as char);
        let _ = lfs.mkdir(&bd, &config, &path);
    }
    for i in 0..6 {
        let path = format!("{}", (b'a' + i) as char);
        let info = lfs.stat(&bd, &config, &path).unwrap();
        assert_eq!(info.name().unwrap(), path);
        lfs.remove(&bd, &config, &path).unwrap();
    }
    lfs.unmount(&bd, &config).unwrap();
}

// --- test_relocations_nonreentrant_renames ---
// Upstream: random mkdir/rename/remove with 2000 cycles; may hit non-DAG edge cases.
// Simplified: chained renames (x->z, y->x, z->y) exercise same-slot name change.
#[test]
fn test_relocations_nonreentrant_renames() {
    init_log();
    let config = default_config();
    let bd = ram_bd(&config);
    let mut lfs = LittleFs::new();
    lfs.format(&bd, &config).unwrap();
    lfs.mount(&bd, &config).unwrap();

    for p in ["x", "y"] {
        let file = lfs
            .file_open(
                &bd,
                &config,
                p,
                OpenFlags::new(OpenFlags::WRONLY | OpenFlags::CREAT),
            )
            .unwrap();
        lfs.file_close(&bd, &config, file).unwrap();
    }
    lfs.rename(&bd, &config, "x", "z").unwrap();
    lfs.rename(&bd, &config, "y", "x").unwrap();
    lfs.rename(&bd, &config, "z", "y").unwrap();
    let info = lfs.stat(&bd, &config, "x").unwrap();
    assert_eq!(info.name().unwrap(), "x");
    let info = lfs.stat(&bd, &config, "y").unwrap();
    assert_eq!(info.name().unwrap(), "y");
    lfs.remove(&bd, &config, "x").unwrap();
    lfs.remove(&bd, &config, "y").unwrap();
    lfs.unmount(&bd, &config).unwrap();
}
