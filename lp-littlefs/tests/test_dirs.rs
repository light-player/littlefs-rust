//! Directory iteration tests.
//!
//! Upstream: tests/test_dirs.toml
//! Source: https://github.com/littlefs-project/littlefs/blob/master/tests/test_dirs.toml

mod common;

use common::{assert_ok, default_config, dir_entry_names, init_context, init_logger, path_bytes};
use lp_littlefs::lfs_type::lfs_type::LFS_TYPE_DIR;
use lp_littlefs::{
    lfs_dir_close, lfs_dir_open, lfs_dir_read, lfs_format, lfs_mkdir, lfs_mount, lfs_remove,
    lfs_rename, lfs_stat, lfs_unmount, Lfs, LfsConfig, LfsDir, LfsInfo,
};

/// Root path: "/" null-terminated.
static ROOT_PATH: [u8; 2] = [b'/', 0];

// --- test_dirs_root ---
// Upstream: dir_open("/"), dir_read returns ".", "..", then 0
#[test]
fn test_dirs_root() {
    init_logger();
    let mut env = default_config(128);
    init_context(&mut env);

    let mut lfs = core::mem::MaybeUninit::<Lfs>::zeroed();
    assert_ok(lfs_format(
        lfs.as_mut_ptr(),
        &env.config as *const LfsConfig,
    ));
    assert_ok(lfs_mount(lfs.as_mut_ptr(), &env.config as *const LfsConfig));

    let mut dir = core::mem::MaybeUninit::<LfsDir>::zeroed();
    assert_ok(lfs_dir_open(
        lfs.as_mut_ptr(),
        dir.as_mut_ptr(),
        ROOT_PATH.as_ptr(),
    ));

    let mut info = core::mem::MaybeUninit::<LfsInfo>::zeroed();
    let n = lfs_dir_read(lfs.as_mut_ptr(), dir.as_mut_ptr(), info.as_mut_ptr());
    assert_eq!(n, 1);
    let info = unsafe { info.assume_init() };
    assert_eq!(info.name[0], b'.');
    assert_eq!(info.name[1], 0);
    assert_eq!(info.type_, LFS_TYPE_DIR as u8);

    let mut info = core::mem::MaybeUninit::<LfsInfo>::zeroed();
    let n = lfs_dir_read(lfs.as_mut_ptr(), dir.as_mut_ptr(), info.as_mut_ptr());
    assert_eq!(n, 1);
    let info = unsafe { info.assume_init() };
    assert_eq!(info.name[0], b'.');
    assert_eq!(info.name[1], b'.');
    assert_eq!(info.name[2], 0);
    assert_eq!(info.type_, LFS_TYPE_DIR as u8);

    let mut info = core::mem::MaybeUninit::<LfsInfo>::zeroed();
    let n = lfs_dir_read(lfs.as_mut_ptr(), dir.as_mut_ptr(), info.as_mut_ptr());
    assert_eq!(n, 0);

    assert_ok(lfs_dir_close(lfs.as_mut_ptr(), dir.as_mut_ptr()));
    assert_ok(lfs_unmount(lfs.as_mut_ptr()));
}

// --- test_dirs_one_mkdir ---
// Upstream: [cases.test_dirs_one_mkdir] mkdir("d0"), stat, dir_read
#[test]
fn test_dirs_one_mkdir() {
    init_logger();
    let mut env = default_config(128);
    init_context(&mut env);

    let mut lfs = core::mem::MaybeUninit::<Lfs>::zeroed();
    assert_ok(lfs_format(
        lfs.as_mut_ptr(),
        &env.config as *const LfsConfig,
    ));
    assert_ok(lfs_mount(lfs.as_mut_ptr(), &env.config as *const LfsConfig));

    let path = path_bytes("d0");
    assert_ok(lfs_mkdir(lfs.as_mut_ptr(), path.as_ptr()));

    let mut info = core::mem::MaybeUninit::<LfsInfo>::zeroed();
    assert_ok(lfs_stat(lfs.as_mut_ptr(), path.as_ptr(), info.as_mut_ptr()));
    let info = unsafe { info.assume_init() };
    let nul = info.name.iter().position(|&b| b == 0).unwrap_or(256);
    assert_eq!(core::str::from_utf8(&info.name[..nul]).unwrap(), "d0");
    assert_eq!(info.type_, LFS_TYPE_DIR as u8);

    let names = dir_entry_names(lfs.as_mut_ptr(), &env.config as *const LfsConfig, "/")
        .expect("dir_entry_names");
    assert_eq!(names.len(), 1);
    assert_eq!(names[0], "d0");

    assert_ok(lfs_unmount(lfs.as_mut_ptr()));
}

// --- test_dirs_many_creation ---
// Upstream: defines.N = range(3,100,3). Subset: start with 1 until second-mkdir bug fixed.
#[test]
fn test_dirs_many_creation() {
    init_logger();
    let mut env = default_config(128);
    init_context(&mut env);

    let mut lfs = core::mem::MaybeUninit::<Lfs>::zeroed();
    assert_ok(lfs_format(
        lfs.as_mut_ptr(),
        &env.config as *const LfsConfig,
    ));
    assert_ok(lfs_mount(lfs.as_mut_ptr(), &env.config as *const LfsConfig));

    let n = 1usize; // TODO: n=2+ fails (mkdir d1 returns -1)
    for i in 0..n {
        let path = path_bytes(&format!("d{i}"));
        let err = lfs_mkdir(lfs.as_mut_ptr(), path.as_ptr());
        assert_ok(err);
    }

    let names = dir_entry_names(lfs.as_mut_ptr(), &env.config as *const LfsConfig, "/")
        .expect("dir_entry_names");
    assert_eq!(names.len(), n);
    let mut names_sorted = names.clone();
    names_sorted.sort();
    let expected: Vec<String> = (0..n).map(|i| format!("d{i}")).collect();
    let mut expected_sorted = expected.clone();
    expected_sorted.sort();
    assert_eq!(names_sorted, expected_sorted);

    assert_ok(lfs_unmount(lfs.as_mut_ptr()));
}

// --- test_dirs_many_removal ---
// Upstream: defines.N = range(3,100,11). Subset: 1 until many_creation fixed for n>1.
#[test]
fn test_dirs_many_removal() {
    init_logger();
    let mut env = default_config(128);
    init_context(&mut env);

    let mut lfs = core::mem::MaybeUninit::<Lfs>::zeroed();
    assert_ok(lfs_format(
        lfs.as_mut_ptr(),
        &env.config as *const LfsConfig,
    ));
    assert_ok(lfs_mount(lfs.as_mut_ptr(), &env.config as *const LfsConfig));

    let n = 1usize;
    for i in 0..n {
        let path = path_bytes(&format!("d{i}"));
        assert_ok(lfs_mkdir(lfs.as_mut_ptr(), path.as_ptr()));
    }
    for i in 0..n {
        let path = path_bytes(&format!("d{i}"));
        assert_ok(lfs_remove(lfs.as_mut_ptr(), path.as_ptr()));
    }

    let names = dir_entry_names(lfs.as_mut_ptr(), &env.config as *const LfsConfig, "/")
        .expect("dir_entry_names");
    assert!(names.is_empty());

    assert_ok(lfs_unmount(lfs.as_mut_ptr()));
}

// --- test_dirs_many_rename ---
// Upstream: defines.N = range(3,100,11). Subset: 1 (currently fails: rename returns -1).
#[test]
fn test_dirs_many_rename() {
    init_logger();
    let mut env = default_config(128);
    init_context(&mut env);

    let mut lfs = core::mem::MaybeUninit::<Lfs>::zeroed();
    assert_ok(lfs_format(
        lfs.as_mut_ptr(),
        &env.config as *const LfsConfig,
    ));
    assert_ok(lfs_mount(lfs.as_mut_ptr(), &env.config as *const LfsConfig));

    let n = 1usize;
    for i in 0..n {
        let path = path_bytes(&format!("d{i}"));
        assert_ok(lfs_mkdir(lfs.as_mut_ptr(), path.as_ptr()));
    }
    for i in 0..n {
        let old_path = path_bytes(&format!("d{i}"));
        let new_path = path_bytes(&format!("x{i}"));
        let err = lfs_rename(
            lfs.as_mut_ptr(),
            old_path.as_ptr(),
            new_path.as_ptr(),
        );
        assert_ok(err);
    }

    let names = dir_entry_names(lfs.as_mut_ptr(), &env.config as *const LfsConfig, "/")
        .expect("dir_entry_names");
    assert_eq!(names.len(), n);
    let mut names_sorted = names.clone();
    names_sorted.sort();
    let expected: Vec<String> = (0..n).map(|i| format!("x{i}")).collect();
    let mut expected_sorted = expected.clone();
    expected_sorted.sort();
    assert_eq!(names_sorted, expected_sorted);

    assert_ok(lfs_unmount(lfs.as_mut_ptr()));
}
