//! Custom attributes tests.
//!
//! Upstream: tests/test_attrs.toml
//! Source: https://github.com/littlefs-project/littlefs/blob/master/tests/test_attrs.toml
//!
//! All tests require lfs_getattr, lfs_setattr, lfs_removeattr. Deferred per phase 07e.

mod common;

use common::{
    assert_err, assert_ok, default_config, init_context, init_logger, path_bytes, LFS_O_CREAT,
    LFS_O_RDONLY, LFS_O_WRONLY,
};
use lp_littlefs::{
    lfs_file_close, lfs_file_open, lfs_file_read, lfs_file_write, lfs_format, lfs_getattr,
    lfs_mkdir, lfs_mount, lfs_removeattr, lfs_setattr, lfs_unmount, Lfs, LfsConfig, LfsFile,
    LFS_ERR_NOATTR, LFS_ERR_NOSPC,
};

/// attr_max from config; tests use ATTR_MAX+1 for NOSPC check.
const ATTR_MAX: usize = 1022;

// --- test_attrs_get_set ---
#[test]
#[ignore = "getattr/setattr/removeattr not implemented"]
fn test_attrs_get_set() {
    init_logger();
    let mut env = default_config(128);
    init_context(&mut env);

    let mut lfs = core::mem::MaybeUninit::<Lfs>::zeroed();
    assert_ok(lfs_format(
        lfs.as_mut_ptr(),
        &env.config as *const LfsConfig,
    ));
    assert_ok(lfs_mount(lfs.as_mut_ptr(), &env.config as *const LfsConfig));

    assert_ok(lfs_mkdir(lfs.as_mut_ptr(), path_bytes("hello").as_ptr()));
    let mut file = core::mem::MaybeUninit::<LfsFile>::zeroed();
    assert_ok(lfs_file_open(
        lfs.as_mut_ptr(),
        file.as_mut_ptr(),
        path_bytes("hello/hello").as_ptr(),
        LFS_O_WRONLY | LFS_O_CREAT,
    ));
    let n = lfs_file_write(
        lfs.as_mut_ptr(),
        file.as_mut_ptr(),
        b"hello".as_ptr() as *const core::ffi::c_void,
        5,
    );
    assert_eq!(n, 5);
    assert_ok(lfs_file_close(lfs.as_mut_ptr(), file.as_mut_ptr()));
    assert_ok(lfs_unmount(lfs.as_mut_ptr()));

    assert_ok(lfs_mount(lfs.as_mut_ptr(), &env.config as *const LfsConfig));
    let mut buffer = [0u8; 1024];

    assert_ok(lfs_setattr(
        lfs.as_mut_ptr(),
        path_bytes("hello").as_ptr(),
        b'A',
        b"aaaa".as_ptr() as *const core::ffi::c_void,
        4,
    ));
    assert_ok(lfs_setattr(
        lfs.as_mut_ptr(),
        path_bytes("hello").as_ptr(),
        b'B',
        b"bbbbbb".as_ptr() as *const core::ffi::c_void,
        6,
    ));
    assert_ok(lfs_setattr(
        lfs.as_mut_ptr(),
        path_bytes("hello").as_ptr(),
        b'C',
        b"ccccc".as_ptr() as *const core::ffi::c_void,
        5,
    ));

    let n = lfs_getattr(
        lfs.as_mut_ptr(),
        path_bytes("hello").as_ptr(),
        b'A',
        buffer[..4].as_mut_ptr() as *mut core::ffi::c_void,
        4,
    );
    assert_eq!(n, 4);
    let n = lfs_getattr(
        lfs.as_mut_ptr(),
        path_bytes("hello").as_ptr(),
        b'B',
        buffer[4..10].as_mut_ptr() as *mut core::ffi::c_void,
        6,
    );
    assert_eq!(n, 6);
    let n = lfs_getattr(
        lfs.as_mut_ptr(),
        path_bytes("hello").as_ptr(),
        b'C',
        buffer[10..15].as_mut_ptr() as *mut core::ffi::c_void,
        5,
    );
    assert_eq!(n, 5);
    assert_eq!(&buffer[0..4], b"aaaa");
    assert_eq!(&buffer[4..10], b"bbbbbb");
    assert_eq!(&buffer[10..15], b"ccccc");

    assert_ok(lfs_setattr(
        lfs.as_mut_ptr(),
        path_bytes("hello").as_ptr(),
        b'B',
        b"".as_ptr() as *const core::ffi::c_void,
        0,
    ));
    let n = lfs_getattr(
        lfs.as_mut_ptr(),
        path_bytes("hello").as_ptr(),
        b'B',
        buffer[4..10].as_mut_ptr() as *mut core::ffi::c_void,
        6,
    );
    assert_eq!(n, 0);
    assert_eq!(&buffer[4..10], b"\0\0\0\0\0\0");

    assert_ok(lfs_removeattr(
        lfs.as_mut_ptr(),
        path_bytes("hello").as_ptr(),
        b'B',
    ));
    let err = lfs_getattr(
        lfs.as_mut_ptr(),
        path_bytes("hello").as_ptr(),
        b'B',
        buffer[4..10].as_mut_ptr() as *mut core::ffi::c_void,
        6,
    );
    assert_err(LFS_ERR_NOATTR, err as i32);

    assert_ok(lfs_setattr(
        lfs.as_mut_ptr(),
        path_bytes("hello").as_ptr(),
        b'B',
        b"dddddd".as_ptr() as *const core::ffi::c_void,
        6,
    ));
    assert_ok(lfs_setattr(
        lfs.as_mut_ptr(),
        path_bytes("hello").as_ptr(),
        b'B',
        b"eee".as_ptr() as *const core::ffi::c_void,
        3,
    ));

    let oversized = vec![0u8; ATTR_MAX + 1];
    let err = lfs_setattr(
        lfs.as_mut_ptr(),
        path_bytes("hello").as_ptr(),
        b'A',
        oversized.as_ptr() as *const core::ffi::c_void,
        (ATTR_MAX + 1) as u32,
    );
    assert_err(LFS_ERR_NOSPC, err);

    assert_ok(lfs_setattr(
        lfs.as_mut_ptr(),
        path_bytes("hello").as_ptr(),
        b'B',
        b"fffffffff".as_ptr() as *const core::ffi::c_void,
        9,
    ));
    assert_ok(lfs_unmount(lfs.as_mut_ptr()));

    assert_ok(lfs_mount(lfs.as_mut_ptr(), &env.config as *const LfsConfig));
    let n = lfs_getattr(
        lfs.as_mut_ptr(),
        path_bytes("hello").as_ptr(),
        b'B',
        buffer[4..13].as_mut_ptr() as *mut core::ffi::c_void,
        9,
    );
    assert_eq!(n, 9);
    assert_eq!(&buffer[4..13], b"fffffffff");

    let mut file = core::mem::MaybeUninit::<LfsFile>::zeroed();
    assert_ok(lfs_file_open(
        lfs.as_mut_ptr(),
        file.as_mut_ptr(),
        path_bytes("hello/hello").as_ptr(),
        LFS_O_RDONLY,
    ));
    let n = lfs_file_read(
        lfs.as_mut_ptr(),
        file.as_mut_ptr(),
        buffer.as_mut_ptr() as *mut core::ffi::c_void,
        32,
    );
    assert_eq!(n, 5);
    assert_eq!(&buffer[..5], b"hello");
    assert_ok(lfs_file_close(lfs.as_mut_ptr(), file.as_mut_ptr()));
    assert_ok(lfs_unmount(lfs.as_mut_ptr()));
}

// --- test_attrs_get_set_root ---
#[test]
#[ignore = "getattr/setattr/removeattr not implemented"]
fn test_attrs_get_set_root() {
    init_logger();
    let mut env = default_config(128);
    init_context(&mut env);

    let mut lfs = core::mem::MaybeUninit::<Lfs>::zeroed();
    assert_ok(lfs_format(
        lfs.as_mut_ptr(),
        &env.config as *const LfsConfig,
    ));
    assert_ok(lfs_mount(lfs.as_mut_ptr(), &env.config as *const LfsConfig));

    assert_ok(lfs_mkdir(lfs.as_mut_ptr(), path_bytes("hello").as_ptr()));
    let mut file = core::mem::MaybeUninit::<LfsFile>::zeroed();
    assert_ok(lfs_file_open(
        lfs.as_mut_ptr(),
        file.as_mut_ptr(),
        path_bytes("hello/hello").as_ptr(),
        LFS_O_WRONLY | LFS_O_CREAT,
    ));
    let n = lfs_file_write(
        lfs.as_mut_ptr(),
        file.as_mut_ptr(),
        b"hello".as_ptr() as *const core::ffi::c_void,
        5,
    );
    assert_eq!(n, 5);
    assert_ok(lfs_file_close(lfs.as_mut_ptr(), file.as_mut_ptr()));
    assert_ok(lfs_unmount(lfs.as_mut_ptr()));

    assert_ok(lfs_mount(lfs.as_mut_ptr(), &env.config as *const LfsConfig));
    let mut buffer = [0u8; 1024];

    assert_ok(lfs_setattr(
        lfs.as_mut_ptr(),
        path_bytes("/").as_ptr(),
        b'A',
        b"aaaa".as_ptr() as *const core::ffi::c_void,
        4,
    ));
    assert_ok(lfs_setattr(
        lfs.as_mut_ptr(),
        path_bytes("/").as_ptr(),
        b'B',
        b"bbbbbb".as_ptr() as *const core::ffi::c_void,
        6,
    ));
    assert_ok(lfs_setattr(
        lfs.as_mut_ptr(),
        path_bytes("/").as_ptr(),
        b'C',
        b"ccccc".as_ptr() as *const core::ffi::c_void,
        5,
    ));

    let n = lfs_getattr(
        lfs.as_mut_ptr(),
        path_bytes("/").as_ptr(),
        b'A',
        buffer[..4].as_mut_ptr() as *mut core::ffi::c_void,
        4,
    );
    assert_eq!(n, 4);
    let n = lfs_getattr(
        lfs.as_mut_ptr(),
        path_bytes("/").as_ptr(),
        b'B',
        buffer[4..10].as_mut_ptr() as *mut core::ffi::c_void,
        6,
    );
    assert_eq!(n, 6);
    let n = lfs_getattr(
        lfs.as_mut_ptr(),
        path_bytes("/").as_ptr(),
        b'C',
        buffer[10..15].as_mut_ptr() as *mut core::ffi::c_void,
        5,
    );
    assert_eq!(n, 5);
    assert_eq!(&buffer[0..4], b"aaaa");
    assert_eq!(&buffer[4..10], b"bbbbbb");
    assert_eq!(&buffer[10..15], b"ccccc");

    assert_ok(lfs_setattr(
        lfs.as_mut_ptr(),
        path_bytes("/").as_ptr(),
        b'B',
        b"".as_ptr() as *const core::ffi::c_void,
        0,
    ));
    assert_ok(lfs_removeattr(
        lfs.as_mut_ptr(),
        path_bytes("/").as_ptr(),
        b'B',
    ));
    assert_ok(lfs_setattr(
        lfs.as_mut_ptr(),
        path_bytes("/").as_ptr(),
        b'B',
        b"fffffffff".as_ptr() as *const core::ffi::c_void,
        9,
    ));
    assert_ok(lfs_unmount(lfs.as_mut_ptr()));

    assert_ok(lfs_mount(lfs.as_mut_ptr(), &env.config as *const LfsConfig));
    let mut buffer = [0u8; 1024];
    let n = lfs_getattr(
        lfs.as_mut_ptr(),
        path_bytes("/").as_ptr(),
        b'A',
        buffer[..4].as_mut_ptr() as *mut core::ffi::c_void,
        4,
    );
    assert_eq!(n, 4);
    let n = lfs_getattr(
        lfs.as_mut_ptr(),
        path_bytes("/").as_ptr(),
        b'B',
        buffer[4..13].as_mut_ptr() as *mut core::ffi::c_void,
        9,
    );
    assert_eq!(n, 9);
    assert_eq!(&buffer[4..13], b"fffffffff");

    let mut file = core::mem::MaybeUninit::<LfsFile>::zeroed();
    assert_ok(lfs_file_open(
        lfs.as_mut_ptr(),
        file.as_mut_ptr(),
        path_bytes("hello/hello").as_ptr(),
        LFS_O_RDONLY,
    ));
    let n = lfs_file_read(
        lfs.as_mut_ptr(),
        file.as_mut_ptr(),
        buffer.as_mut_ptr() as *mut core::ffi::c_void,
        32,
    );
    assert_eq!(n, 5);
    assert_eq!(&buffer[..5], b"hello");
    assert_ok(lfs_file_close(lfs.as_mut_ptr(), file.as_mut_ptr()));
    assert_ok(lfs_unmount(lfs.as_mut_ptr()));
}

// --- test_attrs_get_set_file ---
#[test]
#[ignore = "getattr/setattr/removeattr and file_opencfg attrs not implemented"]
fn test_attrs_get_set_file() {}

// --- test_attrs_deferred_file ---
#[test]
#[ignore = "getattr/setattr/removeattr and file_opencfg attrs not implemented"]
fn test_attrs_deferred_file() {}
