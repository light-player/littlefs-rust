//! Path resolution integration tests.
//!
//! Per upstream test_paths.toml. Many edge cases (dots, UTF-8, etc.) may need #[ignore].

mod common;

use common::fresh_fs;
use lp_littlefs::{Error, FileType, OpenFlags};

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

#[test]
fn test_paths_absolute_files() {
    let (bd, config, mut fs) = fresh_fs();
    fs.mkdir(&bd, &config, "coffee").unwrap();
    for name in PATHS {
        let path = format!("/coffee/{name}");
        let file = fs
            .file_open(
                &bd,
                &config,
                &path,
                OpenFlags::new(OpenFlags::WRONLY | OpenFlags::CREAT | OpenFlags::EXCL),
            )
            .unwrap();
        fs.file_close(&bd, &config, file).unwrap();
    }
    for name in PATHS {
        let info = fs.stat(&bd, &config, &format!("/coffee/{name}")).unwrap();
        assert_eq!(info.name().unwrap(), *name);
        assert_eq!(info.typ, FileType::Reg);
    }
}

#[test]
fn test_paths_absolute_dirs() {
    let (bd, config, mut fs) = fresh_fs();
    fs.mkdir(&bd, &config, "coffee").unwrap();
    for name in PATHS {
        fs.mkdir(&bd, &config, &format!("/coffee/{name}")).unwrap();
    }
    for name in PATHS {
        let info = fs.stat(&bd, &config, &format!("/coffee/{name}")).unwrap();
        assert_eq!(info.name().unwrap(), *name);
        assert_eq!(info.typ, FileType::Dir);
    }
}

#[test]
fn test_paths_noent() {
    let (bd, config, mut fs) = fresh_fs();
    fs.mkdir(&bd, &config, "coffee").unwrap();
    for name in PATHS {
        let path = format!("coffee/{name}");
        fs.mkdir(&bd, &config, &path).unwrap();
    }
    for bad in &[
        "_rip",
        "c_ldbrew",
        "tu_kish",
        "tub_uk",
        "_vietnamese",
        "thai_",
    ] {
        let path = format!("coffee/{bad}");
        assert_eq!(fs.stat(&bd, &config, &path).unwrap_err(), Error::Noent);
        let err = fs
            .file_open(&bd, &config, &path, OpenFlags::new(OpenFlags::RDONLY))
            .err()
            .unwrap();
        assert_eq!(err, Error::Noent);
    }
}

#[test]
fn test_paths_root() {
    let (bd, config, fs) = fresh_fs();
    let _ = fs.dir_open(&bd, &config, "/").unwrap();
    let info = fs.stat(&bd, &config, "/").unwrap();
    assert!(matches!(info.typ, FileType::Dir));
}

#[test]
#[ignore = "redundant slashes may behave differently"]
fn test_paths_redundant_slashes() {}

#[test]
#[ignore = "trailing slashes edge case"]
fn test_paths_trailing_slashes() {}

#[test]
#[ignore = "dot path components"]
fn test_paths_dots() {}

#[test]
#[ignore = "trailing dots"]
fn test_paths_trailing_dots() {}

#[test]
#[ignore = "dotdot path components"]
fn test_paths_dotdots() {}

#[test]
#[ignore = "dot dotdots"]
fn test_paths_trailing_dotdots() {}

#[test]
#[ignore = "dotdotdots"]
fn test_paths_dot_dotdots() {}

#[test]
#[ignore = "leading dots"]
fn test_paths_dotdotdots() {}

#[test]
#[ignore = "root dotdots"]
fn test_paths_leading_dots() {}

#[test]
#[ignore = "noent parent"]
fn test_paths_root_dotdots() {}

#[test]
#[ignore = "noent parent path"]
fn test_paths_noent_parent() {}

#[test]
#[ignore = "notdir parent"]
fn test_paths_notdir_parent() {}

#[test]
#[ignore = "noent trailing slashes"]
fn test_paths_noent_trailing_slashes() {}

#[test]
#[ignore = "noent trailing dots"]
fn test_paths_noent_trailing_dots() {}

#[test]
#[ignore = "noent trailing dotdots"]
fn test_paths_noent_trailing_dotdots() {}

#[test]
#[ignore = "empty path"]
fn test_paths_empty() {}

#[test]
#[ignore = "root aliases"]
fn test_paths_root_aliases() {}

#[test]
#[ignore = "magic noent"]
fn test_paths_magic_noent() {}

#[test]
#[ignore = "magic conflict"]
fn test_paths_magic_conflict() {}

#[test]
#[ignore = "name too long"]
fn test_paths_nametoolong() {}

#[test]
#[ignore = "name just long enough"]
fn test_paths_namejustlongenough() {}

#[test]
#[ignore = "UTF-8 paths"]
fn test_paths_utf8() {}

#[test]
#[ignore = "UTF-8 IPA"]
fn test_paths_utf8_ipa() {}

#[test]
#[ignore = "spaces in paths"]
fn test_paths_spaces() {}

#[test]
#[ignore = "oops all spaces"]
fn test_paths_oopsallspaces() {}

#[test]
#[ignore = "nonprintable"]
fn test_paths_nonprintable() {}

#[test]
#[ignore = "oops all dels"]
fn test_paths_oopsalldels() {}

#[test]
#[ignore = "non-UTF8"]
fn test_paths_nonutf8() {}

#[test]
#[ignore = "oops all 0xff"]
fn test_paths_oopsallffs() {}
