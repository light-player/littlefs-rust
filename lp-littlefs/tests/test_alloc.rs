//! Allocator and block allocation tests.
//!
//! Upstream: tests/test_alloc.toml
//! Source: https://github.com/littlefs-project/littlefs/blob/master/tests/test_alloc.toml

mod common;

use common::{
    assert_ok, default_config, init_context, init_logger, path_bytes, LFS_O_CREAT, LFS_O_WRONLY,
};
use lp_littlefs::{
    lfs_file_close, lfs_file_open, lfs_file_write, lfs_format, lfs_mkdir, lfs_mount, lfs_remove,
    lfs_stat, lfs_unmount, Lfs, LfsConfig, LfsFile, LfsInfo,
};

// --- test_alloc_parallel ---
#[test]
fn test_alloc_parallel() {
    init_logger();
    let mut env = default_config(128);
    init_context(&mut env);

    let mut lfs = core::mem::MaybeUninit::<Lfs>::zeroed();
    assert_ok(lfs_format(
        lfs.as_mut_ptr(),
        &env.config as *const LfsConfig,
    ));
    assert_ok(lfs_mount(lfs.as_mut_ptr(), &env.config as *const LfsConfig));

    for i in 0..4 {
        assert_ok(lfs_mkdir(
            lfs.as_mut_ptr(),
            path_bytes(&format!("d{i}")).as_ptr(),
        ));
    }
    for i in 0..4 {
        let path = path_bytes(&format!("d{i}/f"));
        let mut file = core::mem::MaybeUninit::<LfsFile>::zeroed();
        assert_ok(lfs_file_open(
            lfs.as_mut_ptr(),
            file.as_mut_ptr(),
            path.as_ptr(),
            LFS_O_WRONLY | LFS_O_CREAT,
        ));
        assert_ok(lfs_file_close(lfs.as_mut_ptr(), file.as_mut_ptr()));
    }
    for i in 0..4 {
        let path = path_bytes(&format!("d{i}/f"));
        let mut info = core::mem::MaybeUninit::<LfsInfo>::zeroed();
        assert_ok(lfs_stat(lfs.as_mut_ptr(), path.as_ptr(), info.as_mut_ptr()));
        let info = unsafe { info.assume_init() };
        let nul = info.name.iter().position(|&b| b == 0).unwrap_or(256);
        assert_eq!(core::str::from_utf8(&info.name[..nul]).unwrap(), "f");
    }

    assert_ok(lfs_unmount(lfs.as_mut_ptr()));
}

// --- test_alloc_serial ---
#[test]
fn test_alloc_serial() {
    init_logger();
    let mut env = default_config(128);
    init_context(&mut env);

    let mut lfs = core::mem::MaybeUninit::<Lfs>::zeroed();
    assert_ok(lfs_format(
        lfs.as_mut_ptr(),
        &env.config as *const LfsConfig,
    ));
    assert_ok(lfs_mount(lfs.as_mut_ptr(), &env.config as *const LfsConfig));

    assert_ok(lfs_mkdir(lfs.as_mut_ptr(), path_bytes("d0").as_ptr()));
    for i in 0..8 {
        let path = path_bytes(&format!("d0/f{i}"));
        let mut file = core::mem::MaybeUninit::<LfsFile>::zeroed();
        assert_ok(lfs_file_open(
            lfs.as_mut_ptr(),
            file.as_mut_ptr(),
            path.as_ptr(),
            LFS_O_WRONLY | LFS_O_CREAT,
        ));
        assert_ok(lfs_file_close(lfs.as_mut_ptr(), file.as_mut_ptr()));
    }

    assert_ok(lfs_unmount(lfs.as_mut_ptr()));
}

// --- test_alloc_parallel_reuse ---
#[test]
fn test_alloc_parallel_reuse() {
    init_logger();
    let mut env = default_config(128);
    init_context(&mut env);

    let mut lfs = core::mem::MaybeUninit::<Lfs>::zeroed();
    assert_ok(lfs_format(
        lfs.as_mut_ptr(),
        &env.config as *const LfsConfig,
    ));
    assert_ok(lfs_mount(lfs.as_mut_ptr(), &env.config as *const LfsConfig));

    assert_ok(lfs_mkdir(lfs.as_mut_ptr(), path_bytes("a").as_ptr()));
    assert_ok(lfs_mkdir(lfs.as_mut_ptr(), path_bytes("b").as_ptr()));
    let mut file = core::mem::MaybeUninit::<LfsFile>::zeroed();
    assert_ok(lfs_file_open(
        lfs.as_mut_ptr(),
        file.as_mut_ptr(),
        path_bytes("a/x").as_ptr(),
        LFS_O_WRONLY | LFS_O_CREAT,
    ));
    assert_ok(lfs_file_close(lfs.as_mut_ptr(), file.as_mut_ptr()));
    assert_ok(lfs_remove(lfs.as_mut_ptr(), path_bytes("a/x").as_ptr()));
    let mut file = core::mem::MaybeUninit::<LfsFile>::zeroed();
    assert_ok(lfs_file_open(
        lfs.as_mut_ptr(),
        file.as_mut_ptr(),
        path_bytes("b/y").as_ptr(),
        LFS_O_WRONLY | LFS_O_CREAT,
    ));
    assert_ok(lfs_file_close(lfs.as_mut_ptr(), file.as_mut_ptr()));

    assert_ok(lfs_unmount(lfs.as_mut_ptr()));
}

// --- test_alloc_serial_reuse ---
#[test]
fn test_alloc_serial_reuse() {
    init_logger();
    let mut env = default_config(128);
    init_context(&mut env);

    let mut lfs = core::mem::MaybeUninit::<Lfs>::zeroed();
    assert_ok(lfs_format(
        lfs.as_mut_ptr(),
        &env.config as *const LfsConfig,
    ));
    assert_ok(lfs_mount(lfs.as_mut_ptr(), &env.config as *const LfsConfig));

    for i in 0..4 {
        let path = path_bytes(&format!("f{i}"));
        let mut file = core::mem::MaybeUninit::<LfsFile>::zeroed();
        assert_ok(lfs_file_open(
            lfs.as_mut_ptr(),
            file.as_mut_ptr(),
            path.as_ptr(),
            LFS_O_WRONLY | LFS_O_CREAT,
        ));
        assert_ok(lfs_file_close(lfs.as_mut_ptr(), file.as_mut_ptr()));
    }
    for i in 0..4 {
        assert_ok(lfs_remove(
            lfs.as_mut_ptr(),
            path_bytes(&format!("f{i}")).as_ptr(),
        ));
    }
    for i in 0..4 {
        let path = path_bytes(&format!("g{i}"));
        let mut file = core::mem::MaybeUninit::<LfsFile>::zeroed();
        assert_ok(lfs_file_open(
            lfs.as_mut_ptr(),
            file.as_mut_ptr(),
            path.as_ptr(),
            LFS_O_WRONLY | LFS_O_CREAT,
        ));
        assert_ok(lfs_file_close(lfs.as_mut_ptr(), file.as_mut_ptr()));
    }

    assert_ok(lfs_unmount(lfs.as_mut_ptr()));
}

// --- test_alloc_exhaustion ---
#[test]
fn test_alloc_exhaustion() {
    init_logger();
    let mut env = default_config(128);
    init_context(&mut env);

    let mut lfs = core::mem::MaybeUninit::<Lfs>::zeroed();
    assert_ok(lfs_format(
        lfs.as_mut_ptr(),
        &env.config as *const LfsConfig,
    ));
    assert_ok(lfs_mount(lfs.as_mut_ptr(), &env.config as *const LfsConfig));

    let mut i = 0u32;
    while i < 200 {
        let path = path_bytes(&format!("f{i}"));
        let mut file = core::mem::MaybeUninit::<LfsFile>::zeroed();
        let err = lfs_file_open(
            lfs.as_mut_ptr(),
            file.as_mut_ptr(),
            path.as_ptr(),
            LFS_O_WRONLY | LFS_O_CREAT,
        );
        if err == 0 {
            assert_ok(lfs_file_close(lfs.as_mut_ptr(), file.as_mut_ptr()));
            i += 1;
        } else {
            // Exhaustion: NOMEM, NOSPC, or NAMETOOLONG (dir full) etc.
            break;
        }
    }

    assert_ok(lfs_unmount(lfs.as_mut_ptr()));
}

// --- test_alloc_split_dir ---
#[test]
fn test_alloc_split_dir() {
    init_logger();
    let mut env = default_config(128);
    init_context(&mut env);

    let mut lfs = core::mem::MaybeUninit::<Lfs>::zeroed();
    assert_ok(lfs_format(
        lfs.as_mut_ptr(),
        &env.config as *const LfsConfig,
    ));
    assert_ok(lfs_mount(lfs.as_mut_ptr(), &env.config as *const LfsConfig));

    assert_ok(lfs_mkdir(lfs.as_mut_ptr(), path_bytes("d").as_ptr()));
    for i in 0..8 {
        let path = path_bytes(&format!("d/f{i}"));
        let mut file = core::mem::MaybeUninit::<LfsFile>::zeroed();
        assert_ok(lfs_file_open(
            lfs.as_mut_ptr(),
            file.as_mut_ptr(),
            path.as_ptr(),
            LFS_O_WRONLY | LFS_O_CREAT,
        ));
        let n = lfs_file_write(
            lfs.as_mut_ptr(),
            file.as_mut_ptr(),
            b"x".as_ptr() as *const core::ffi::c_void,
            1,
        );
        assert_eq!(n, 1);
        assert_ok(lfs_file_close(lfs.as_mut_ptr(), file.as_mut_ptr()));
    }
    for i in 0..8 {
        let path = path_bytes(&format!("d/f{i}"));
        let mut info = core::mem::MaybeUninit::<LfsInfo>::zeroed();
        assert_ok(lfs_stat(lfs.as_mut_ptr(), path.as_ptr(), info.as_mut_ptr()));
        let info = unsafe { info.assume_init() };
        let nul = info.name.iter().position(|&b| b == 0).unwrap_or(256);
        assert_eq!(
            core::str::from_utf8(&info.name[..nul]).unwrap(),
            format!("f{i}")
        );
    }

    assert_ok(lfs_unmount(lfs.as_mut_ptr()));
}

// --- Deferred ---

#[test]
#[ignore = "exhaustion wraparound semantics may differ"]
fn test_alloc_exhaustion_wraparound() {}

#[test]
#[ignore = "dir exhaustion edge case"]
fn test_alloc_dir_exhaustion() {}

#[test]
#[ignore = "bad-block BD simulation not implemented"]
fn test_alloc_bad_blocks() {}

#[test]
#[ignore = "chained dir exhaustion"]
fn test_alloc_chained_dir_exhaustion() {}

#[test]
#[ignore = "lookahead state edge case"]
fn test_alloc_outdated_lookahead() {}

#[test]
#[ignore = "lookahead + split dir"]
fn test_alloc_outdated_lookahead_split_dir() {}
