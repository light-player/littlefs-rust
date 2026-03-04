//! Path resolution integration tests.
//!
//! Upstream: tests/test_paths.toml
//! Source: https://github.com/littlefs-project/littlefs/blob/master/tests/test_paths.toml
//!
//! Many edge cases (dots, UTF-8, etc.) deferred with #[ignore] per roadmap 07a.

mod common;

use common::{assert_err, assert_ok, default_config, init_context, init_logger, path_bytes};
use lp_littlefs::lfs_type::lfs_type::{LFS_TYPE_DIR, LFS_TYPE_REG};
use lp_littlefs::{
    lfs_dir_close, lfs_dir_open, lfs_format, lfs_mkdir, lfs_mount, lfs_stat, lfs_unmount, Lfs,
    LfsConfig, LfsDir, LfsInfo, LFS_ERR_NOENT,
};
use lp_littlefs::{lfs_file_close, lfs_file_open, LfsFile};

use common::{LFS_O_CREAT, LFS_O_EXCL, LFS_O_RDONLY, LFS_O_WRONLY};

const PATHS: &[&str] = &[
    "drip",
    "coldbrew",
    "turkish",
    "tubruk",
    "vietnamese",
    "thai",
];

// --- test_paths_simple_dirs ---
#[test]
fn test_paths_simple_dirs() {
    init_logger();
    let mut env = default_config(128);
    init_context(&mut env);

    let mut lfs = core::mem::MaybeUninit::<Lfs>::zeroed();
    assert_ok(lfs_format(
        lfs.as_mut_ptr(),
        &env.config as *const LfsConfig,
    ));
    assert_ok(lfs_mount(lfs.as_mut_ptr(), &env.config as *const LfsConfig));

    let coffee = path_bytes("coffee");
    assert_ok(lfs_mkdir(lfs.as_mut_ptr(), coffee.as_ptr()));

    for name in PATHS {
        let path = path_bytes(&format!("coffee/{name}"));
        assert_ok(lfs_mkdir(lfs.as_mut_ptr(), path.as_ptr()));
        let mut info = core::mem::MaybeUninit::<LfsInfo>::zeroed();
        assert_ok(lfs_stat(lfs.as_mut_ptr(), path.as_ptr(), info.as_mut_ptr()));
        let info = unsafe { info.assume_init() };
        let nul = info.name.iter().position(|&b| b == 0).unwrap_or(256);
        assert_eq!(core::str::from_utf8(&info.name[..nul]).unwrap(), *name);
        assert_eq!(info.type_, LFS_TYPE_DIR as u8);
    }
    assert_ok(lfs_unmount(lfs.as_mut_ptr()));
}

// --- test_paths_simple_files ---
#[test]
fn test_paths_simple_files() {
    init_logger();
    let mut env = default_config(128);
    init_context(&mut env);

    let mut lfs = core::mem::MaybeUninit::<Lfs>::zeroed();
    assert_ok(lfs_format(
        lfs.as_mut_ptr(),
        &env.config as *const LfsConfig,
    ));
    assert_ok(lfs_mount(lfs.as_mut_ptr(), &env.config as *const LfsConfig));

    let coffee = path_bytes("coffee");
    assert_ok(lfs_mkdir(lfs.as_mut_ptr(), coffee.as_ptr()));

    for name in PATHS {
        let path = path_bytes(&format!("coffee/{name}"));
        let mut file = core::mem::MaybeUninit::<LfsFile>::zeroed();
        assert_ok(lfs_file_open(
            lfs.as_mut_ptr(),
            file.as_mut_ptr(),
            path.as_ptr(),
            LFS_O_WRONLY | LFS_O_CREAT | LFS_O_EXCL,
        ));
        assert_ok(lfs_file_close(lfs.as_mut_ptr(), file.as_mut_ptr()));
        let mut info = core::mem::MaybeUninit::<LfsInfo>::zeroed();
        assert_ok(lfs_stat(lfs.as_mut_ptr(), path.as_ptr(), info.as_mut_ptr()));
        let info = unsafe { info.assume_init() };
        let nul = info.name.iter().position(|&b| b == 0).unwrap_or(256);
        assert_eq!(core::str::from_utf8(&info.name[..nul]).unwrap(), *name);
        assert_eq!(info.type_, LFS_TYPE_REG as u8);
    }
    assert_ok(lfs_unmount(lfs.as_mut_ptr()));
}

// --- test_paths_absolute_files ---
#[test]
fn test_paths_absolute_files() {
    init_logger();
    let mut env = default_config(128);
    init_context(&mut env);

    let mut lfs = core::mem::MaybeUninit::<Lfs>::zeroed();
    assert_ok(lfs_format(
        lfs.as_mut_ptr(),
        &env.config as *const LfsConfig,
    ));
    assert_ok(lfs_mount(lfs.as_mut_ptr(), &env.config as *const LfsConfig));

    let coffee = path_bytes("coffee");
    assert_ok(lfs_mkdir(lfs.as_mut_ptr(), coffee.as_ptr()));

    for name in PATHS {
        let path = path_bytes(&format!("/coffee/{name}"));
        let mut file = core::mem::MaybeUninit::<LfsFile>::zeroed();
        assert_ok(lfs_file_open(
            lfs.as_mut_ptr(),
            file.as_mut_ptr(),
            path.as_ptr(),
            LFS_O_WRONLY | LFS_O_CREAT | LFS_O_EXCL,
        ));
        assert_ok(lfs_file_close(lfs.as_mut_ptr(), file.as_mut_ptr()));
    }
    for name in PATHS {
        let path = path_bytes(&format!("/coffee/{name}"));
        let mut info = core::mem::MaybeUninit::<LfsInfo>::zeroed();
        assert_ok(lfs_stat(lfs.as_mut_ptr(), path.as_ptr(), info.as_mut_ptr()));
        let info = unsafe { info.assume_init() };
        let nul = info.name.iter().position(|&b| b == 0).unwrap_or(256);
        assert_eq!(core::str::from_utf8(&info.name[..nul]).unwrap(), *name);
        assert_eq!(info.type_, LFS_TYPE_REG as u8);
    }
    assert_ok(lfs_unmount(lfs.as_mut_ptr()));
}

// --- test_paths_absolute_dirs ---
#[test]
fn test_paths_absolute_dirs() {
    init_logger();
    let mut env = default_config(128);
    init_context(&mut env);

    let mut lfs = core::mem::MaybeUninit::<Lfs>::zeroed();
    assert_ok(lfs_format(
        lfs.as_mut_ptr(),
        &env.config as *const LfsConfig,
    ));
    assert_ok(lfs_mount(lfs.as_mut_ptr(), &env.config as *const LfsConfig));

    let coffee = path_bytes("coffee");
    assert_ok(lfs_mkdir(lfs.as_mut_ptr(), coffee.as_ptr()));

    for name in PATHS {
        let path = path_bytes(&format!("/coffee/{name}"));
        assert_ok(lfs_mkdir(lfs.as_mut_ptr(), path.as_ptr()));
    }
    for name in PATHS {
        let path = path_bytes(&format!("/coffee/{name}"));
        let mut info = core::mem::MaybeUninit::<LfsInfo>::zeroed();
        assert_ok(lfs_stat(lfs.as_mut_ptr(), path.as_ptr(), info.as_mut_ptr()));
        let info = unsafe { info.assume_init() };
        let nul = info.name.iter().position(|&b| b == 0).unwrap_or(256);
        assert_eq!(core::str::from_utf8(&info.name[..nul]).unwrap(), *name);
        assert_eq!(info.type_, LFS_TYPE_DIR as u8);
    }
    assert_ok(lfs_unmount(lfs.as_mut_ptr()));
}

// --- test_paths_noent ---
#[test]
fn test_paths_noent() {
    init_logger();
    let mut env = default_config(128);
    init_context(&mut env);

    let mut lfs = core::mem::MaybeUninit::<Lfs>::zeroed();
    assert_ok(lfs_format(
        lfs.as_mut_ptr(),
        &env.config as *const LfsConfig,
    ));
    assert_ok(lfs_mount(lfs.as_mut_ptr(), &env.config as *const LfsConfig));

    let coffee = path_bytes("coffee");
    assert_ok(lfs_mkdir(lfs.as_mut_ptr(), coffee.as_ptr()));
    for name in PATHS {
        let path = path_bytes(&format!("coffee/{name}"));
        assert_ok(lfs_mkdir(lfs.as_mut_ptr(), path.as_ptr()));
    }

    for bad in &[
        "_rip",
        "c_ldbrew",
        "tu_kish",
        "tub_uk",
        "_vietnamese",
        "thai_",
    ] {
        let path = path_bytes(&format!("coffee/{bad}"));
        let mut info = core::mem::MaybeUninit::<LfsInfo>::zeroed();
        let err = lfs_stat(lfs.as_mut_ptr(), path.as_ptr(), info.as_mut_ptr());
        assert_err(LFS_ERR_NOENT, err);

        let mut file = core::mem::MaybeUninit::<LfsFile>::zeroed();
        let err = lfs_file_open(
            lfs.as_mut_ptr(),
            file.as_mut_ptr(),
            path.as_ptr(),
            LFS_O_RDONLY,
        );
        assert_err(LFS_ERR_NOENT, err);
    }
    assert_ok(lfs_unmount(lfs.as_mut_ptr()));
}

// --- test_paths_root ---
#[test]
fn test_paths_root() {
    init_logger();
    let mut env = default_config(128);
    init_context(&mut env);

    let mut lfs = core::mem::MaybeUninit::<Lfs>::zeroed();
    assert_ok(lfs_format(
        lfs.as_mut_ptr(),
        &env.config as *const LfsConfig,
    ));
    assert_ok(lfs_mount(lfs.as_mut_ptr(), &env.config as *const LfsConfig));

    let root_path = path_bytes("/");
    let mut dir = core::mem::MaybeUninit::<LfsDir>::zeroed();
    assert_ok(lfs_dir_open(
        lfs.as_mut_ptr(),
        dir.as_mut_ptr(),
        root_path.as_ptr(),
    ));
    assert_ok(lfs_dir_close(lfs.as_mut_ptr(), dir.as_mut_ptr()));

    let mut info = core::mem::MaybeUninit::<LfsInfo>::zeroed();
    assert_ok(lfs_stat(
        lfs.as_mut_ptr(),
        root_path.as_ptr(),
        info.as_mut_ptr(),
    ));
    let info = unsafe { info.assume_init() };
    assert_eq!(info.type_, LFS_TYPE_DIR as u8);

    assert_ok(lfs_unmount(lfs.as_mut_ptr()));
}

// --- Deferred edge-case tests (per roadmap 07a) ---

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
#[ignore = "spaces in paths"]
fn test_paths_spaces() {}

#[test]
#[ignore = "nonprintable"]
fn test_paths_nonprintable() {}

#[test]
#[ignore = "non-UTF8"]
fn test_paths_nonutf8() {}
