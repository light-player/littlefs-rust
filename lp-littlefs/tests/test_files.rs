//! File read/write integration tests.
//!
//! Upstream: tests/test_files.toml
//! Source: https://github.com/littlefs-project/littlefs/blob/master/tests/test_files.toml

mod common;

use common::{
    assert_ok, default_config, fs_with_hello, init_context, path_bytes, LFS_O_CREAT, LFS_O_EXCL,
    LFS_O_RDONLY, LFS_O_WRONLY,
};
use lp_littlefs::{
    lfs_file_close, lfs_file_open, lfs_file_read, lfs_file_rewind, lfs_file_seek, lfs_file_size,
    lfs_file_sync, lfs_file_tell, lfs_file_truncate, lfs_file_write, lfs_format, lfs_mount,
    lfs_unmount, Lfs, LfsConfig, LfsFile,
};
use rstest::rstest;

// ── Upstream Cases ──────────────────────────

// --- test_files_simple ---
// Upstream: [cases.test_files_simple] Create, write, close, mount, read.
#[test]
fn test_files_simple() {
    let mut env = default_config(128);
    init_context(&mut env);

    let mut lfs = core::mem::MaybeUninit::<Lfs>::zeroed();
    assert_ok(lfs_format(
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
        LFS_O_WRONLY | LFS_O_CREAT | LFS_O_EXCL,
    ));
    let n = lfs_file_write(
        lfs.as_mut_ptr(),
        file.as_mut_ptr(),
        data.as_ptr() as *const core::ffi::c_void,
        data.len() as u32,
    );
    assert_eq!(n, data.len() as i32);
    assert_ok(lfs_file_close(lfs.as_mut_ptr(), file.as_mut_ptr()));
    assert_ok(lfs_unmount(lfs.as_mut_ptr()));

    assert_ok(lfs_mount(lfs.as_mut_ptr(), &env.config as *const LfsConfig));
    let mut file = core::mem::MaybeUninit::<LfsFile>::zeroed();
    assert_ok(lfs_file_open(
        lfs.as_mut_ptr(),
        file.as_mut_ptr(),
        path.as_ptr(),
        LFS_O_RDONLY,
    ));
    let mut buf = [0u8; 32];
    let n = lfs_file_read(
        lfs.as_mut_ptr(),
        file.as_mut_ptr(),
        buf.as_mut_ptr() as *mut core::ffi::c_void,
        32,
    );
    assert_eq!(n, data.len() as i32);
    assert_eq!(&buf[..n as usize], data);
    assert_ok(lfs_file_close(lfs.as_mut_ptr(), file.as_mut_ptr()));
    assert_ok(lfs_unmount(lfs.as_mut_ptr()));
}

// --- test_files_append ---
// Upstream: [cases.test_files_append] APPEND flag.
#[test]
#[ignore = "stub"]
fn test_files_append() {
    todo!()
}

// --- test_files_truncate ---
// Upstream: [cases.test_files_truncate] TRUNC flag.
#[test]
#[ignore = "stub"]
fn test_files_truncate() {
    todo!()
}

/// Upstream: [cases.test_files_large]
/// defines.SIZE = [...], defines.CHUNKSIZE = [...], defines.INLINE_MAX = [...]
///
/// Large file write/read.
#[rstest]
#[ignore = "stub"]
fn test_files_large(
    #[values(1024, 4096)] _size: u32,
    #[values(64, 512)] _chunk_size: u32,
    #[values(0, 256)] _inline_max: u32,
) {
    todo!()
}

/// Upstream: [cases.test_files_rewrite]
/// defines.SIZE1, SIZE2, CHUNKSIZE, INLINE_MAX
///
/// Rewrite file with different size.
#[rstest]
#[ignore = "stub"]
fn test_files_rewrite(
    #[values(100, 200)] _size1: u32,
    #[values(50, 300)] _size2: u32,
    #[values(64, 256)] _chunk_size: u32,
    #[values(0, 128)] _inline_max: u32,
) {
    todo!()
}

/// Upstream: [cases.test_files_reentrant_write]
/// defines.SIZE, CHUNKSIZE, INLINE_MAX
///
/// Power-loss during write.
#[rstest]
#[ignore = "stub"]
fn test_files_reentrant_write(
    #[values(512, 1024)] _size: u32,
    #[values(64, 256)] _chunk_size: u32,
    #[values(0, 256)] _inline_max: u32,
) {
    todo!()
}

/// Upstream: [cases.test_files_reentrant_write_sync]
///
/// Power-loss during write with sync points.
#[test]
#[ignore = "stub"]
fn test_files_reentrant_write_sync() {
    todo!()
}

/// Upstream: [cases.test_files_many_power_cycle]
/// defines.N = 300
///
/// Many files with power cycles.
#[test]
#[ignore = "stub"]
fn test_files_many_power_cycle() {
    todo!()
}

/// Upstream: [cases.test_files_many_power_loss]
/// defines.N = 300, defines.POWERLOSS_BEHAVIOR
///
/// Many files with power loss simulation.
#[test]
#[ignore = "stub"]
fn test_files_many_power_loss() {
    todo!()
}

// --- test_files_many ---
// Upstream: [cases.test_files_many] Many small files.
#[test]
fn test_files_many() {
    let mut env = default_config(128);
    init_context(&mut env);

    let mut lfs = core::mem::MaybeUninit::<Lfs>::zeroed();
    assert_ok(lfs_format(
        lfs.as_mut_ptr(),
        &env.config as *const LfsConfig,
    ));
    assert_ok(lfs_mount(lfs.as_mut_ptr(), &env.config as *const LfsConfig));

    let n_files = 5usize; // Fits in one dir block; more would need lfs_dir_split
    for i in 0..n_files {
        let path = path_bytes(&format!("f{i}"));
        let mut file = core::mem::MaybeUninit::<LfsFile>::zeroed();
        assert_ok(lfs_file_open(
            lfs.as_mut_ptr(),
            file.as_mut_ptr(),
            path.as_ptr(),
            LFS_O_WRONLY | LFS_O_CREAT | LFS_O_EXCL,
        ));
        let content = format!("data{i}");
        let bytes = content.as_bytes();
        let _ = lfs_file_write(
            lfs.as_mut_ptr(),
            file.as_mut_ptr(),
            bytes.as_ptr() as *const core::ffi::c_void,
            bytes.len() as u32,
        );
        assert_ok(lfs_file_close(lfs.as_mut_ptr(), file.as_mut_ptr()));
    }

    assert_ok(lfs_unmount(lfs.as_mut_ptr()));
    assert_ok(lfs_mount(lfs.as_mut_ptr(), &env.config as *const LfsConfig));

    for i in 0..n_files {
        let path = path_bytes(&format!("f{i}"));
        let mut file = core::mem::MaybeUninit::<LfsFile>::zeroed();
        assert_ok(lfs_file_open(
            lfs.as_mut_ptr(),
            file.as_mut_ptr(),
            path.as_ptr(),
            LFS_O_RDONLY,
        ));
        let expected = format!("data{i}");
        assert_eq!(
            lfs_file_size(lfs.as_mut_ptr(), file.as_mut_ptr()),
            expected.len() as i32
        );
        let mut buf = [0u8; 32];
        let n = lfs_file_read(
            lfs.as_mut_ptr(),
            file.as_mut_ptr(),
            buf.as_mut_ptr() as *mut core::ffi::c_void,
            32,
        );
        assert_eq!(n, expected.len() as i32);
        assert_eq!(core::str::from_utf8(&buf[..n as usize]).unwrap(), expected);
        assert_ok(lfs_file_close(lfs.as_mut_ptr(), file.as_mut_ptr()));
    }
    assert_ok(lfs_unmount(lfs.as_mut_ptr()));
}

// ── Rust-specific extras ────────────────────
// Bug reproducers, debug helpers, unit tests. Not in upstream.

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

#[test]
fn test_files_truncate_api() {
    let mut env = default_config(128);
    init_context(&mut env);

    let mut lfs = core::mem::MaybeUninit::<Lfs>::zeroed();
    assert_ok(lfs_format(
        lfs.as_mut_ptr(),
        &env.config as *const LfsConfig,
    ));
    assert_ok(lfs_mount(lfs.as_mut_ptr(), &env.config as *const LfsConfig));

    let path = path_bytes("x");
    let mut file = core::mem::MaybeUninit::<LfsFile>::zeroed();
    assert_ok(lfs_file_open(
        lfs.as_mut_ptr(),
        file.as_mut_ptr(),
        path.as_ptr(),
        LFS_O_WRONLY | LFS_O_CREAT | LFS_O_EXCL,
    ));
    let data = b"hello world";
    let _ = lfs_file_write(
        lfs.as_mut_ptr(),
        file.as_mut_ptr(),
        data.as_ptr() as *const core::ffi::c_void,
        data.len() as u32,
    );
    assert_ok(lfs_file_truncate(lfs.as_mut_ptr(), file.as_mut_ptr(), 5));
    assert_ok(lfs_file_sync(lfs.as_mut_ptr(), file.as_mut_ptr()));
    assert_ok(lfs_file_close(lfs.as_mut_ptr(), file.as_mut_ptr()));

    assert_ok(lfs_unmount(lfs.as_mut_ptr()));
    assert_ok(lfs_mount(lfs.as_mut_ptr(), &env.config as *const LfsConfig));

    let mut file = core::mem::MaybeUninit::<LfsFile>::zeroed();
    assert_ok(lfs_file_open(
        lfs.as_mut_ptr(),
        file.as_mut_ptr(),
        path.as_ptr(),
        LFS_O_RDONLY,
    ));
    assert_eq!(lfs_file_size(lfs.as_mut_ptr(), file.as_mut_ptr()), 5);
    let mut buf = [0u8; 32];
    let n = lfs_file_read(
        lfs.as_mut_ptr(),
        file.as_mut_ptr(),
        buf.as_mut_ptr() as *mut core::ffi::c_void,
        32,
    );
    assert_eq!(n, 5);
    assert_eq!(&buf[..5], b"hello");
    assert_ok(lfs_file_close(lfs.as_mut_ptr(), file.as_mut_ptr()));
    assert_ok(lfs_unmount(lfs.as_mut_ptr()));
}
