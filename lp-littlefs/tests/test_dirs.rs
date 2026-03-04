//! Directory iteration tests.
//!
//! Upstream: tests/test_dirs.toml
//! Source: https://github.com/littlefs-project/littlefs/blob/master/tests/test_dirs.toml

mod common;

use common::{assert_ok, default_config, init_context, init_logger};
use lp_littlefs::lfs_type::lfs_type::LFS_TYPE_DIR;
use lp_littlefs::{
    lfs_dir_close, lfs_dir_open, lfs_dir_read, lfs_format, lfs_mount, lfs_unmount, Lfs, LfsConfig,
    LfsDir, LfsInfo,
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
