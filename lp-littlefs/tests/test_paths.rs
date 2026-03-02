//! Path resolution integration tests.
//!
//! Per test_paths_simple: mkdir coffee, mkdir coffee/drip etc, stat each path.

mod common;

use common::fresh_fs;
use lp_littlefs::{FileType, OpenFlags};

const PATHS: &[&str] = &[
    "drip",
    "coldbrew",
    "turkish",
    "tubruk",
    "vietnamese",
    "thai",
];

#[test]
fn test_paths_simple_dirs() {
    let (bd, config, mut fs) = fresh_fs();

    fs.mkdir(&bd, &config, "coffee").unwrap();
    for name in PATHS {
        let path = format!("coffee/{name}");
        fs.mkdir(&bd, &config, &path).unwrap();
        let info = fs.stat(&bd, &config, &path).unwrap();
        assert_eq!(info.name().unwrap(), *name, "stat {path}");
        assert_eq!(info.typ, FileType::Dir, "stat {path}");
    }
}

#[test]
fn test_paths_simple_files() {
    let (bd, config, mut fs) = fresh_fs();

    fs.mkdir(&bd, &config, "coffee").unwrap();
    for name in PATHS {
        let path = format!("coffee/{name}");
        let file = fs
            .file_open(
                &bd,
                &config,
                &path,
                OpenFlags::new(OpenFlags::WRONLY | OpenFlags::CREAT | OpenFlags::EXCL),
            )
            .unwrap();
        fs.file_close(&bd, &config, file).unwrap();
        let info = fs.stat(&bd, &config, &path).unwrap();
        assert_eq!(info.name().unwrap(), *name, "stat {path}");
        assert_eq!(info.typ, FileType::Reg, "stat {path}");
    }
}
