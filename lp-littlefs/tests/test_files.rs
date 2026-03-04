//! File read/write integration tests.
//!
//! Upstream: tests/test_files.toml
//! Source: https://github.com/littlefs-project/littlefs/blob/master/tests/test_files.toml

mod common;

use common::{assert_ok, default_config, fs_with_hello, init_context, path_bytes};
use lp_littlefs::{
    lfs_file_close, lfs_file_open, lfs_file_read, lfs_file_rewind, lfs_file_seek, lfs_file_size,
    lfs_file_tell, lfs_file_write, lfs_mount, lfs_unmount, Lfs, LfsConfig, LfsFile,
};

#[test]
fn test_files_same_session() {
    let mut env = default_config(128);
    init_context(&mut env);

    let mut lfs = core::mem::MaybeUninit::<Lfs>::zeroed();
    assert_ok(lp_littlefs::lfs_format(
        lfs.as_mut_ptr(),
        &env.config as *const LfsConfig,
    ));
    assert_ok(lfs_mount(lfs.as_mut_ptr(), &env.config as *const LfsConfig));

    let path = path_bytes("hello");
    let data = b"Hello World!\0";
    let mut file = core::mem::MaybeUninit::<LfsFile>::zeroed();
    assert_ok(lfs_file_open(
        lfs.as_mut_ptr(),
        file.as_mut_ptr(),
        path.as_ptr(),
        0x0100 | 2,
    ));
    let n = lfs_file_write(
        lfs.as_mut_ptr(),
        file.as_mut_ptr(),
        data.as_ptr() as *const core::ffi::c_void,
        data.len() as u32,
    );
    assert_eq!(n, data.len() as i32);
    assert_ok(lfs_file_close(lfs.as_mut_ptr(), file.as_mut_ptr()));

    let mut file2 = core::mem::MaybeUninit::<LfsFile>::zeroed();
    assert_ok(lfs_file_open(
        lfs.as_mut_ptr(),
        file2.as_mut_ptr(),
        path.as_ptr(),
        1,
    ));
    assert_eq!(lfs_file_size(lfs.as_mut_ptr(), file2.as_mut_ptr()), 13);
    let mut buf = [0u8; 32];
    let n = lfs_file_read(
        lfs.as_mut_ptr(),
        file2.as_mut_ptr(),
        buf.as_mut_ptr() as *mut core::ffi::c_void,
        32,
    );
    assert_eq!(n, 13);
    assert_eq!(&buf[..13], b"Hello World!\0");
    assert_ok(lfs_file_close(lfs.as_mut_ptr(), file2.as_mut_ptr()));
}

#[test]
fn test_files_simple_read() {
    let mut env = default_config(128);
    fs_with_hello(&mut env).expect("fs_with_hello");
    init_context(&mut env);

    let mut lfs = core::mem::MaybeUninit::<Lfs>::zeroed();
    assert_ok(lfs_mount(lfs.as_mut_ptr(), &env.config as *const LfsConfig));

    let path = path_bytes("hello");
    let mut file = core::mem::MaybeUninit::<LfsFile>::zeroed();
    assert_ok(lfs_file_open(
        lfs.as_mut_ptr(),
        file.as_mut_ptr(),
        path.as_ptr(),
        1,
    ));

    assert_eq!(lfs_file_size(lfs.as_mut_ptr(), file.as_mut_ptr()), 13);
    assert_eq!(lfs_file_tell(lfs.as_mut_ptr(), file.as_mut_ptr()), 0);

    let mut buf = [0u8; 32];
    let n = lfs_file_read(
        lfs.as_mut_ptr(),
        file.as_mut_ptr(),
        buf.as_mut_ptr() as *mut core::ffi::c_void,
        32,
    );
    assert_eq!(n, 13);
    assert_eq!(&buf[..13], b"Hello World!\0");

    let n2 = lfs_file_read(
        lfs.as_mut_ptr(),
        file.as_mut_ptr(),
        buf.as_mut_ptr() as *mut core::ffi::c_void,
        32,
    );
    assert_eq!(n2, 0);

    assert_ok(lfs_file_close(lfs.as_mut_ptr(), file.as_mut_ptr()));
    assert_ok(lfs_unmount(lfs.as_mut_ptr()));
}

#[test]
fn test_files_seek_tell() {
    let mut env = default_config(128);
    fs_with_hello(&mut env).expect("fs_with_hello");
    init_context(&mut env);

    let mut lfs = core::mem::MaybeUninit::<Lfs>::zeroed();
    assert_ok(lfs_mount(lfs.as_mut_ptr(), &env.config as *const LfsConfig));

    let path = path_bytes("hello");
    let mut file = core::mem::MaybeUninit::<LfsFile>::zeroed();
    assert_ok(lfs_file_open(
        lfs.as_mut_ptr(),
        file.as_mut_ptr(),
        path.as_ptr(),
        1,
    ));

    let mut buf = [0u8; 4];
    let n = lfs_file_read(
        lfs.as_mut_ptr(),
        file.as_mut_ptr(),
        buf.as_mut_ptr() as *mut core::ffi::c_void,
        4,
    );
    assert_eq!(n, 4);
    assert_eq!(&buf[..4], b"Hell");
    assert_eq!(lfs_file_tell(lfs.as_mut_ptr(), file.as_mut_ptr()), 4);

    assert_ok(lfs_file_rewind(lfs.as_mut_ptr(), file.as_mut_ptr()));
    assert_eq!(lfs_file_tell(lfs.as_mut_ptr(), file.as_mut_ptr()), 0);

    let n2 = lfs_file_read(
        lfs.as_mut_ptr(),
        file.as_mut_ptr(),
        buf.as_mut_ptr() as *mut core::ffi::c_void,
        4,
    );
    assert_eq!(n2, 4);
    assert_eq!(&buf[..4], b"Hell");

    let pos = lfs_file_seek(lfs.as_mut_ptr(), file.as_mut_ptr(), 6, 0);
    assert_eq!(pos, 6);
    let n3 = lfs_file_read(
        lfs.as_mut_ptr(),
        file.as_mut_ptr(),
        buf.as_mut_ptr() as *mut core::ffi::c_void,
        4,
    );
    assert_eq!(n3, 4);
    assert_eq!(&buf[..4], b"Worl");

    assert_ok(lfs_file_close(lfs.as_mut_ptr(), file.as_mut_ptr()));
    assert_ok(lfs_unmount(lfs.as_mut_ptr()));
}
