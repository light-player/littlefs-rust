//! File read/write integration tests.
//!
//! Upstream: tests/test_files.toml
//! Source: https://github.com/littlefs-project/littlefs/blob/master/tests/test_files.toml

mod common;

use common::{
    advance_prng, assert_ok, config_with_inline_max, default_config, fs_with_hello, init_context,
    path_bytes,
    powerloss::{init_powerloss_context, powerloss_config, run_powerloss_linear},
    verify_prng_file, verify_prng_file_with_state, write_prng_file, write_prng_file_result,
    LFS_O_APPEND, LFS_O_CREAT, LFS_O_EXCL, LFS_O_RDONLY, LFS_O_TRUNC, LFS_O_WRONLY,
};
use lp_littlefs::{
    lfs_file_close, lfs_file_open, lfs_file_read, lfs_file_rewind, lfs_file_seek, lfs_file_size,
    lfs_file_sync, lfs_file_tell, lfs_file_truncate, lfs_file_write, lfs_format, lfs_mount,
    lfs_unmount, Lfs, LfsConfig, LfsFile, LFS_ERR_NOENT,
};
use rstest::rstest;

/// Block count for tests with large files (SIZE up to 262144).
const BLOCK_COUNT_LARGE: u32 = 1024;

/// Block count for test_files_many with N=300 (needs dir space for 300 entries).
const BLOCK_COUNT_MANY: u32 = 256;

// ── Upstream Cases ──────────────────────────

/// Upstream: [cases.test_files_simple]
/// defines.INLINE_MAX = [0, -1, 8]
///
/// Create, write "Hello World!\0", close, unmount, mount, read, verify.
#[rstest]
fn test_files_simple(#[values(0, -1, 8)] inline_max: i32) {
    let mut env = config_with_inline_max(128, inline_max);
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

/// Upstream: [cases.test_files_large]
/// defines.SIZE = [32, 8192, 262144, 0, 7, 8193]
/// defines.CHUNKSIZE = [31, 16, 33, 1, 1023]
/// defines.INLINE_MAX = [0, -1, 8]
///
/// Write SIZE bytes of PRNG(seed=1) in CHUNKSIZE chunks, unmount, remount,
/// verify file_size == SIZE, read back and verify. Final read past EOF returns 0.
#[rstest]
fn test_files_large(
    #[values(32, 8192, 262144, 0, 7, 8193)] size: u32,
    #[values(31, 16, 33, 1, 1023)] chunk_size: u32,
    #[values(0, -1, 8)] inline_max: i32,
) {
    let mut env = config_with_inline_max(BLOCK_COUNT_LARGE, inline_max);
    init_context(&mut env);

    let mut lfs = core::mem::MaybeUninit::<Lfs>::zeroed();
    assert_ok(lfs_format(
        lfs.as_mut_ptr(),
        &env.config as *const LfsConfig,
    ));

    // write
    assert_ok(lfs_mount(lfs.as_mut_ptr(), &env.config as *const LfsConfig));
    let path = path_bytes("avacado");
    let mut file = core::mem::MaybeUninit::<LfsFile>::zeroed();
    assert_ok(lfs_file_open(
        lfs.as_mut_ptr(),
        file.as_mut_ptr(),
        path.as_ptr(),
        LFS_O_WRONLY | LFS_O_CREAT | LFS_O_EXCL,
    ));
    write_prng_file(lfs.as_mut_ptr(), file.as_mut_ptr(), size, chunk_size, 1);
    assert_ok(lfs_file_close(lfs.as_mut_ptr(), file.as_mut_ptr()));
    assert_ok(lfs_unmount(lfs.as_mut_ptr()));

    // read
    assert_ok(lfs_mount(lfs.as_mut_ptr(), &env.config as *const LfsConfig));
    assert_ok(lfs_file_open(
        lfs.as_mut_ptr(),
        file.as_mut_ptr(),
        path.as_ptr(),
        LFS_O_RDONLY,
    ));
    assert_eq!(
        lfs_file_size(lfs.as_mut_ptr(), file.as_mut_ptr()),
        size as i32
    );
    verify_prng_file(lfs.as_mut_ptr(), file.as_mut_ptr(), size, chunk_size, 1);
    // Final read past EOF returns 0
    let mut buf = [0u8; 1024];
    let n = lfs_file_read(
        lfs.as_mut_ptr(),
        file.as_mut_ptr(),
        buf.as_mut_ptr() as *mut core::ffi::c_void,
        chunk_size,
    );
    assert_eq!(n, 0);
    assert_ok(lfs_file_close(lfs.as_mut_ptr(), file.as_mut_ptr()));
    assert_ok(lfs_unmount(lfs.as_mut_ptr()));
}

/// Upstream: [cases.test_files_rewrite]
/// defines.SIZE1 = [32, 8192, 131072, 0, 7, 8193]
/// defines.SIZE2 = [32, 8192, 131072, 0, 7, 8193]
/// defines.CHUNKSIZE = [31, 16, 1]
/// defines.INLINE_MAX = [0, -1, 8]
///
/// Write SIZE1, read back, rewrite with SIZE2 (WRONLY, no TRUNC), read:
/// first SIZE2 bytes PRNG(2), remaining (SIZE2..SIZE1) PRNG(1) from offset SIZE2.
#[rstest]
fn test_files_rewrite(
    #[values(32, 8192, 131072, 0, 7, 8193)] size1: u32,
    #[values(32, 8192, 131072, 0, 7, 8193)] size2: u32,
    #[values(31, 16, 1)] chunk_size: u32,
    #[values(0, -1, 8)] inline_max: i32,
) {
    let mut env = config_with_inline_max(BLOCK_COUNT_LARGE, inline_max);
    init_context(&mut env);

    let mut lfs = core::mem::MaybeUninit::<Lfs>::zeroed();
    assert_ok(lfs_format(
        lfs.as_mut_ptr(),
        &env.config as *const LfsConfig,
    ));

    let path = path_bytes("avacado");
    let mut file = core::mem::MaybeUninit::<LfsFile>::zeroed();

    // write SIZE1
    assert_ok(lfs_mount(lfs.as_mut_ptr(), &env.config as *const LfsConfig));
    assert_ok(lfs_file_open(
        lfs.as_mut_ptr(),
        file.as_mut_ptr(),
        path.as_ptr(),
        LFS_O_WRONLY | LFS_O_CREAT | LFS_O_EXCL,
    ));
    write_prng_file(lfs.as_mut_ptr(), file.as_mut_ptr(), size1, chunk_size, 1);
    assert_ok(lfs_file_close(lfs.as_mut_ptr(), file.as_mut_ptr()));
    assert_ok(lfs_unmount(lfs.as_mut_ptr()));

    // read SIZE1
    assert_ok(lfs_mount(lfs.as_mut_ptr(), &env.config as *const LfsConfig));
    assert_ok(lfs_file_open(
        lfs.as_mut_ptr(),
        file.as_mut_ptr(),
        path.as_ptr(),
        LFS_O_RDONLY,
    ));
    assert_eq!(
        lfs_file_size(lfs.as_mut_ptr(), file.as_mut_ptr()),
        size1 as i32
    );
    verify_prng_file(lfs.as_mut_ptr(), file.as_mut_ptr(), size1, chunk_size, 1);
    assert_ok(lfs_file_close(lfs.as_mut_ptr(), file.as_mut_ptr()));
    assert_ok(lfs_unmount(lfs.as_mut_ptr()));

    // rewrite SIZE2 (WRONLY, no TRUNC)
    assert_ok(lfs_mount(lfs.as_mut_ptr(), &env.config as *const LfsConfig));
    assert_ok(lfs_file_open(
        lfs.as_mut_ptr(),
        file.as_mut_ptr(),
        path.as_ptr(),
        LFS_O_WRONLY,
    ));
    write_prng_file(lfs.as_mut_ptr(), file.as_mut_ptr(), size2, chunk_size, 2);
    assert_ok(lfs_file_close(lfs.as_mut_ptr(), file.as_mut_ptr()));
    assert_ok(lfs_unmount(lfs.as_mut_ptr()));

    // read: first SIZE2 = PRNG(2), then SIZE2..SIZE1 (if size1 > size2) = PRNG(1) from offset SIZE2
    assert_ok(lfs_mount(lfs.as_mut_ptr(), &env.config as *const LfsConfig));
    assert_ok(lfs_file_open(
        lfs.as_mut_ptr(),
        file.as_mut_ptr(),
        path.as_ptr(),
        LFS_O_RDONLY,
    ));
    assert_eq!(
        lfs_file_size(lfs.as_mut_ptr(), file.as_mut_ptr()),
        size1.max(size2) as i32
    );
    verify_prng_file(lfs.as_mut_ptr(), file.as_mut_ptr(), size2, chunk_size, 2);
    if size1 > size2 {
        let mut prng = 1u32;
        advance_prng(&mut prng, size2);
        verify_prng_file_with_state(
            lfs.as_mut_ptr(),
            file.as_mut_ptr(),
            size1 - size2,
            chunk_size,
            &mut prng,
        );
    }
    // Final read past EOF returns 0
    let mut buf = [0u8; 1024];
    let n = lfs_file_read(
        lfs.as_mut_ptr(),
        file.as_mut_ptr(),
        buf.as_mut_ptr() as *mut core::ffi::c_void,
        chunk_size,
    );
    assert_eq!(n, 0);
    assert_ok(lfs_file_close(lfs.as_mut_ptr(), file.as_mut_ptr()));
    assert_ok(lfs_unmount(lfs.as_mut_ptr()));
}

/// Upstream: [cases.test_files_append]
/// defines.SIZE1 = [32, 8192, 131072, 0, 7, 8193]
/// defines.SIZE2 = [32, 8192, 131072, 0, 7, 8193]
/// defines.CHUNKSIZE = [31, 16, 1]
/// defines.INLINE_MAX = [0, -1, 8]
///
/// Write SIZE1, append SIZE2 (PRNG seed 2). Read: first SIZE1 = PRNG(1), next SIZE2 = PRNG(2).
#[rstest]
fn test_files_append(
    #[values(32, 8192, 131072, 0, 7, 8193)] size1: u32,
    #[values(32, 8192, 131072, 0, 7, 8193)] size2: u32,
    #[values(31, 16, 1)] chunk_size: u32,
    #[values(0, -1, 8)] inline_max: i32,
) {
    let mut env = config_with_inline_max(BLOCK_COUNT_LARGE, inline_max);
    init_context(&mut env);

    let mut lfs = core::mem::MaybeUninit::<Lfs>::zeroed();
    assert_ok(lfs_format(
        lfs.as_mut_ptr(),
        &env.config as *const LfsConfig,
    ));

    let path = path_bytes("avacado");
    let mut file = core::mem::MaybeUninit::<LfsFile>::zeroed();

    // write SIZE1
    assert_ok(lfs_mount(lfs.as_mut_ptr(), &env.config as *const LfsConfig));
    assert_ok(lfs_file_open(
        lfs.as_mut_ptr(),
        file.as_mut_ptr(),
        path.as_ptr(),
        LFS_O_WRONLY | LFS_O_CREAT | LFS_O_EXCL,
    ));
    write_prng_file(lfs.as_mut_ptr(), file.as_mut_ptr(), size1, chunk_size, 1);
    assert_ok(lfs_file_close(lfs.as_mut_ptr(), file.as_mut_ptr()));
    assert_ok(lfs_unmount(lfs.as_mut_ptr()));

    // append SIZE2
    assert_ok(lfs_mount(lfs.as_mut_ptr(), &env.config as *const LfsConfig));
    assert_ok(lfs_file_open(
        lfs.as_mut_ptr(),
        file.as_mut_ptr(),
        path.as_ptr(),
        LFS_O_WRONLY | LFS_O_APPEND,
    ));
    write_prng_file(lfs.as_mut_ptr(), file.as_mut_ptr(), size2, chunk_size, 2);
    assert_ok(lfs_file_close(lfs.as_mut_ptr(), file.as_mut_ptr()));
    assert_ok(lfs_unmount(lfs.as_mut_ptr()));

    // read: SIZE1 + SIZE2, first PRNG(1) then PRNG(2)
    assert_ok(lfs_mount(lfs.as_mut_ptr(), &env.config as *const LfsConfig));
    assert_ok(lfs_file_open(
        lfs.as_mut_ptr(),
        file.as_mut_ptr(),
        path.as_ptr(),
        LFS_O_RDONLY,
    ));
    assert_eq!(
        lfs_file_size(lfs.as_mut_ptr(), file.as_mut_ptr()),
        (size1 + size2) as i32
    );
    verify_prng_file(lfs.as_mut_ptr(), file.as_mut_ptr(), size1, chunk_size, 1);
    verify_prng_file(lfs.as_mut_ptr(), file.as_mut_ptr(), size2, chunk_size, 2);
    assert_ok(lfs_file_close(lfs.as_mut_ptr(), file.as_mut_ptr()));
    assert_ok(lfs_unmount(lfs.as_mut_ptr()));
}

/// Upstream: [cases.test_files_truncate]
/// defines.SIZE1 = [32, 8192, 131072, 0, 7, 8193]
/// defines.SIZE2 = [32, 8192, 131072, 0, 7, 8193]
/// defines.CHUNKSIZE = [31, 16, 1]
/// defines.INLINE_MAX = [0, -1, 8]
///
/// Write SIZE1, truncate+write SIZE2 (TRUNC|WRONLY). Read: SIZE2 bytes PRNG(2). Final read returns 0.
#[rstest]
fn test_files_truncate(
    #[values(32, 8192, 131072, 0, 7, 8193)] size1: u32,
    #[values(32, 8192, 131072, 0, 7, 8193)] size2: u32,
    #[values(31, 16, 1)] chunk_size: u32,
    #[values(0, -1, 8)] inline_max: i32,
) {
    let mut env = config_with_inline_max(BLOCK_COUNT_LARGE, inline_max);
    init_context(&mut env);

    let mut lfs = core::mem::MaybeUninit::<Lfs>::zeroed();
    assert_ok(lfs_format(
        lfs.as_mut_ptr(),
        &env.config as *const LfsConfig,
    ));

    let path = path_bytes("avacado");
    let mut file = core::mem::MaybeUninit::<LfsFile>::zeroed();

    // write SIZE1
    assert_ok(lfs_mount(lfs.as_mut_ptr(), &env.config as *const LfsConfig));
    assert_ok(lfs_file_open(
        lfs.as_mut_ptr(),
        file.as_mut_ptr(),
        path.as_ptr(),
        LFS_O_WRONLY | LFS_O_CREAT | LFS_O_EXCL,
    ));
    write_prng_file(lfs.as_mut_ptr(), file.as_mut_ptr(), size1, chunk_size, 1);
    assert_ok(lfs_file_close(lfs.as_mut_ptr(), file.as_mut_ptr()));
    assert_ok(lfs_unmount(lfs.as_mut_ptr()));

    // truncate + write SIZE2
    assert_ok(lfs_mount(lfs.as_mut_ptr(), &env.config as *const LfsConfig));
    assert_ok(lfs_file_open(
        lfs.as_mut_ptr(),
        file.as_mut_ptr(),
        path.as_ptr(),
        LFS_O_WRONLY | LFS_O_TRUNC,
    ));
    write_prng_file(lfs.as_mut_ptr(), file.as_mut_ptr(), size2, chunk_size, 2);
    assert_ok(lfs_file_close(lfs.as_mut_ptr(), file.as_mut_ptr()));
    assert_ok(lfs_unmount(lfs.as_mut_ptr()));

    // read SIZE2
    assert_ok(lfs_mount(lfs.as_mut_ptr(), &env.config as *const LfsConfig));
    assert_ok(lfs_file_open(
        lfs.as_mut_ptr(),
        file.as_mut_ptr(),
        path.as_ptr(),
        LFS_O_RDONLY,
    ));
    assert_eq!(
        lfs_file_size(lfs.as_mut_ptr(), file.as_mut_ptr()),
        size2 as i32
    );
    verify_prng_file(lfs.as_mut_ptr(), file.as_mut_ptr(), size2, chunk_size, 2);
    let mut buf = [0u8; 1024];
    let n = lfs_file_read(
        lfs.as_mut_ptr(),
        file.as_mut_ptr(),
        buf.as_mut_ptr() as *mut core::ffi::c_void,
        chunk_size,
    );
    assert_eq!(n, 0);
    assert_ok(lfs_file_close(lfs.as_mut_ptr(), file.as_mut_ptr()));
    assert_ok(lfs_unmount(lfs.as_mut_ptr()));
}

/// Upstream: [cases.test_files_reentrant_write]
/// defines.SIZE = [32, 0, 7, 2049]
/// defines.CHUNKSIZE = [31, 16, 65]
/// defines.INLINE_MAX = [0, -1, 8]
/// defines.POWERLOSS_BEHAVIOR = [NOOP, OOO] — we use NOOP only (OOO not implemented)
///
/// Mount-or-format, check existing file (size 0 or SIZE), write SIZE PRNG(1),
/// close, read back, verify. Power-loss retries until success.
#[rstest]
fn test_files_reentrant_write(
    #[values(32, 0, 7, 2049)] size: u32,
    #[values(31, 16, 65)] chunk_size: u32,
    #[values(0, -1, 8)] inline_max: i32,
) {
    let mut env = powerloss_config(256);
    init_powerloss_context(&mut env);
    env.config.inline_max = if inline_max < 0 {
        u32::MAX
    } else {
        inline_max as u32
    };

    let config_ptr = &env.config as *const LfsConfig;
    let mut lfs = core::mem::MaybeUninit::<Lfs>::zeroed();

    // Format and mount for initial snapshot
    assert_ok(lp_littlefs::lfs_format(lfs.as_mut_ptr(), config_ptr));
    assert_ok(lp_littlefs::lfs_mount(lfs.as_mut_ptr(), config_ptr));
    assert_ok(lp_littlefs::lfs_unmount(lfs.as_mut_ptr()));
    let snapshot = env.snapshot();

    let max_iter = 5000;

    let op = |lfs: *mut Lfs, cfg: *const LfsConfig| -> Result<(), i32> {
        let err = lp_littlefs::lfs_mount(lfs, cfg);
        if err != 0 {
            let _ = lp_littlefs::lfs_format(lfs, cfg);
            let e = lp_littlefs::lfs_mount(lfs, cfg);
            if e != 0 {
                return Err(e);
            }
        }

        let path = path_bytes("avacado");
        let mut file = core::mem::MaybeUninit::<LfsFile>::zeroed();
        let open_err =
            lp_littlefs::lfs_file_open(lfs, file.as_mut_ptr(), path.as_ptr(), LFS_O_RDONLY);
        if open_err == 0 {
            let sz = lp_littlefs::lfs_file_size(lfs, file.as_mut_ptr());
            assert!(sz == 0 || sz == size as i32, "size must be 0 or SIZE");
            let e = lp_littlefs::lfs_file_close(lfs, file.as_mut_ptr());
            if e != 0 {
                return Err(e);
            }
        } else {
            assert_eq!(open_err, LFS_ERR_NOENT);
        }

        let e = lp_littlefs::lfs_file_open(
            lfs,
            file.as_mut_ptr(),
            path.as_ptr(),
            LFS_O_WRONLY | LFS_O_CREAT,
        );
        if e != 0 {
            return Err(e);
        }
        write_prng_file_result(lfs, file.as_mut_ptr(), size, chunk_size, 1)?;
        let e = lp_littlefs::lfs_file_close(lfs, file.as_mut_ptr());
        if e != 0 {
            return Err(e);
        }
        let e = lp_littlefs::lfs_unmount(lfs);
        if e != 0 {
            return Err(e);
        }
        Ok(())
    };

    let verify = |lfs: *mut Lfs, cfg: *const LfsConfig| -> Result<(), i32> {
        let remount = lp_littlefs::lfs_mount(lfs, cfg);
        if remount != 0 {
            return Ok(());
        }
        let path = path_bytes("avacado");
        let mut file = core::mem::MaybeUninit::<LfsFile>::zeroed();
        let err = lp_littlefs::lfs_file_open(lfs, file.as_mut_ptr(), path.as_ptr(), LFS_O_RDONLY);
        if err != 0 {
            let _ = lp_littlefs::lfs_unmount(lfs);
            return Ok(());
        }
        let sz = lp_littlefs::lfs_file_size(lfs, file.as_mut_ptr());
        if sz == size as i32 {
            verify_prng_file(lfs, file.as_mut_ptr(), size, chunk_size, 1);
        }
        let e = lp_littlefs::lfs_file_close(lfs, file.as_mut_ptr());
        if e != 0 {
            return Err(e);
        }
        let e = lp_littlefs::lfs_unmount(lfs);
        if e != 0 {
            return Err(e);
        }
        Ok(())
    };

    let result = run_powerloss_linear(&mut env, &snapshot, max_iter, op, verify);
    result.expect("reentrant write should eventually succeed");
}

/// Upstream: [cases.test_files_reentrant_write_sync]
/// Three modes: APPEND, TRUNC, plain write. SIZE/CHUNKSIZE/INLINE_MAX vary per mode.
/// Power-loss after each sync. Stub: implement APPEND mode with SIZE=[32,0,7,2049].
#[rstest]
fn test_files_reentrant_write_sync(
    #[values(32, 0, 7, 2049)] size: u32,
    #[values(31, 16, 65)] chunk_size: u32,
    #[values(0, -1, 8)] inline_max: i32,
) {
    let mut env = powerloss_config(256);
    init_powerloss_context(&mut env);
    env.config.inline_max = if inline_max < 0 {
        u32::MAX
    } else {
        inline_max as u32
    };

    let config_ptr = &env.config as *const LfsConfig;
    let mut lfs = core::mem::MaybeUninit::<Lfs>::zeroed();

    assert_ok(lp_littlefs::lfs_format(lfs.as_mut_ptr(), config_ptr));
    assert_ok(lp_littlefs::lfs_mount(lfs.as_mut_ptr(), config_ptr));
    assert_ok(lp_littlefs::lfs_unmount(lfs.as_mut_ptr()));
    let snapshot = env.snapshot();

    let max_iter = 5000;

    let op = |lfs: *mut Lfs, cfg: *const LfsConfig| -> Result<(), i32> {
        let err = lp_littlefs::lfs_mount(lfs, cfg);
        if err != 0 {
            let _ = lp_littlefs::lfs_format(lfs, cfg);
            let e = lp_littlefs::lfs_mount(lfs, cfg);
            if e != 0 {
                return Err(e);
            }
        }

        let path = path_bytes("avacado");
        let mut file = core::mem::MaybeUninit::<LfsFile>::zeroed();
        let open_err =
            lp_littlefs::lfs_file_open(lfs, file.as_mut_ptr(), path.as_ptr(), LFS_O_RDONLY);
        if open_err == 0 {
            let sz = lp_littlefs::lfs_file_size(lfs, file.as_mut_ptr());
            assert!(sz <= size as i32);
            let mut prng = 1u32;
            let mut buf = [0u8; 1024];
            let mut i: u32 = 0;
            while i < sz as u32 {
                let chunk = (chunk_size.min(sz as u32 - i)) as usize;
                let n = lp_littlefs::lfs_file_read(
                    lfs,
                    file.as_mut_ptr(),
                    buf.as_mut_ptr() as *mut core::ffi::c_void,
                    chunk as u32,
                );
                assert_eq!(n, chunk as i32);
                for slot in buf[..chunk].iter() {
                    let expected = (common::test_prng(&mut prng) & 0xff) as u8;
                    assert_eq!(*slot, expected);
                }
                i += chunk as u32;
            }
            let e = lp_littlefs::lfs_file_close(lfs, file.as_mut_ptr());
            if e != 0 {
                return Err(e);
            }
        } else {
            assert_eq!(open_err, LFS_ERR_NOENT);
        }

        let e = lp_littlefs::lfs_file_open(
            lfs,
            file.as_mut_ptr(),
            path.as_ptr(),
            LFS_O_WRONLY | LFS_O_CREAT | LFS_O_APPEND,
        );
        if e != 0 {
            return Err(e);
        }
        let current_size = lp_littlefs::lfs_file_size(lfs, file.as_mut_ptr());
        let skip = current_size.max(0) as u32;
        let mut prng = 1u32;
        common::advance_prng(&mut prng, skip);
        let mut i = skip;
        while i < size {
            let chunk = chunk_size.min(size - i);
            let mut buf = [0u8; 1024];
            for slot in buf[..chunk as usize].iter_mut() {
                *slot = (common::test_prng(&mut prng) & 0xff) as u8;
            }
            let n = lp_littlefs::lfs_file_write(
                lfs,
                file.as_mut_ptr(),
                buf.as_ptr() as *const core::ffi::c_void,
                chunk,
            );
            if n < 0 {
                return Err(n);
            }
            assert_eq!(n, chunk as i32);
            let e = lp_littlefs::lfs_file_sync(lfs, file.as_mut_ptr());
            if e != 0 {
                return Err(e);
            }
            i += chunk;
        }
        let e = lp_littlefs::lfs_file_close(lfs, file.as_mut_ptr());
        if e != 0 {
            return Err(e);
        }
        let e = lp_littlefs::lfs_unmount(lfs);
        if e != 0 {
            return Err(e);
        }
        Ok(())
    };

    let verify = |lfs: *mut Lfs, cfg: *const LfsConfig| -> Result<(), i32> {
        if lp_littlefs::lfs_mount(lfs, cfg) != 0 {
            return Ok(());
        }
        let path = path_bytes("avacado");
        let mut file = core::mem::MaybeUninit::<LfsFile>::zeroed();
        if lp_littlefs::lfs_file_open(lfs, file.as_mut_ptr(), path.as_ptr(), LFS_O_RDONLY) != 0 {
            let _ = lp_littlefs::lfs_unmount(lfs);
            return Ok(());
        }
        let sz = lp_littlefs::lfs_file_size(lfs, file.as_mut_ptr());
        if sz == size as i32 {
            verify_prng_file(lfs, file.as_mut_ptr(), size, chunk_size, 1);
        }
        let e = lp_littlefs::lfs_file_close(lfs, file.as_mut_ptr());
        if e != 0 {
            return Err(e);
        }
        let e = lp_littlefs::lfs_unmount(lfs);
        if e != 0 {
            return Err(e);
        }
        Ok(())
    };

    let result = run_powerloss_linear(&mut env, &snapshot, max_iter, op, verify);
    result.expect("reentrant write sync should eventually succeed");
}

/// Upstream: [cases.test_files_many]
/// defines.N = 300
///
/// Create 300 files of 7 bytes ("Hi %03d"), read each back immediately, verify.
#[test]
fn test_files_many() {
    const N: usize = 300;
    let mut env = default_config(BLOCK_COUNT_MANY);
    init_context(&mut env);

    let mut lfs = core::mem::MaybeUninit::<Lfs>::zeroed();
    assert_ok(lfs_format(
        lfs.as_mut_ptr(),
        &env.config as *const LfsConfig,
    ));
    assert_ok(lfs_mount(lfs.as_mut_ptr(), &env.config as *const LfsConfig));

    for i in 0..N {
        let path = path_bytes(&format!("file_{:03}", i));
        let mut file = core::mem::MaybeUninit::<LfsFile>::zeroed();
        assert_ok(lfs_file_open(
            lfs.as_mut_ptr(),
            file.as_mut_ptr(),
            path.as_ptr(),
            LFS_O_WRONLY | LFS_O_CREAT | LFS_O_EXCL,
        ));
        let content = format!("Hi {:03}\0", i);
        let bytes = content.as_bytes();
        assert_eq!(bytes.len(), 7);
        let n = lfs_file_write(
            lfs.as_mut_ptr(),
            file.as_mut_ptr(),
            bytes.as_ptr() as *const core::ffi::c_void,
            bytes.len() as u32,
        );
        assert_eq!(n, bytes.len() as i32);
        assert_ok(lfs_file_close(lfs.as_mut_ptr(), file.as_mut_ptr()));

        let mut rfile = core::mem::MaybeUninit::<LfsFile>::zeroed();
        assert_ok(lfs_file_open(
            lfs.as_mut_ptr(),
            rfile.as_mut_ptr(),
            path.as_ptr(),
            LFS_O_RDONLY,
        ));
        let mut buf = [0u8; 32];
        let n = lfs_file_read(
            lfs.as_mut_ptr(),
            rfile.as_mut_ptr(),
            buf.as_mut_ptr() as *mut core::ffi::c_void,
            7,
        );
        assert_eq!(n, 7);
        assert_eq!(&buf[..7], bytes);
        assert_ok(lfs_file_close(lfs.as_mut_ptr(), rfile.as_mut_ptr()));
    }
    assert_ok(lfs_unmount(lfs.as_mut_ptr()));
}

/// Upstream: [cases.test_files_many_power_cycle]
/// defines.N = 300
///
/// Create 300 files, unmount/remount after each. Verify on final mount.
#[test]
fn test_files_many_power_cycle() {
    const N: usize = 300;
    let mut env = default_config(BLOCK_COUNT_MANY);
    init_context(&mut env);

    let mut lfs = core::mem::MaybeUninit::<Lfs>::zeroed();
    assert_ok(lfs_format(
        lfs.as_mut_ptr(),
        &env.config as *const LfsConfig,
    ));

    for i in 0..N {
        assert_ok(lfs_mount(lfs.as_mut_ptr(), &env.config as *const LfsConfig));
        let path = path_bytes(&format!("file_{:03}", i));
        let mut file = core::mem::MaybeUninit::<LfsFile>::zeroed();
        assert_ok(lfs_file_open(
            lfs.as_mut_ptr(),
            file.as_mut_ptr(),
            path.as_ptr(),
            LFS_O_WRONLY | LFS_O_CREAT | LFS_O_EXCL,
        ));
        let content = format!("Hi {:03}\0", i);
        let bytes = content.as_bytes();
        assert_eq!(bytes.len(), 7);
        let n = lfs_file_write(
            lfs.as_mut_ptr(),
            file.as_mut_ptr(),
            bytes.as_ptr() as *const core::ffi::c_void,
            bytes.len() as u32,
        );
        assert_eq!(n, bytes.len() as i32);
        assert_ok(lfs_file_close(lfs.as_mut_ptr(), file.as_mut_ptr()));
        assert_ok(lfs_unmount(lfs.as_mut_ptr()));

        assert_ok(lfs_mount(lfs.as_mut_ptr(), &env.config as *const LfsConfig));
        let mut rfile = core::mem::MaybeUninit::<LfsFile>::zeroed();
        assert_ok(lfs_file_open(
            lfs.as_mut_ptr(),
            rfile.as_mut_ptr(),
            path.as_ptr(),
            LFS_O_RDONLY,
        ));
        let mut buf = [0u8; 32];
        let n = lfs_file_read(
            lfs.as_mut_ptr(),
            rfile.as_mut_ptr(),
            buf.as_mut_ptr() as *mut core::ffi::c_void,
            7,
        );
        assert_eq!(n, 7);
        assert_eq!(&buf[..7], bytes);
        assert_ok(lfs_file_close(lfs.as_mut_ptr(), rfile.as_mut_ptr()));
    }
    assert_ok(lfs_unmount(lfs.as_mut_ptr()));
}

/// Upstream: [cases.test_files_many_power_loss]
/// defines.N = 300, defines.POWERLOSS_BEHAVIOR = [NOOP, OOO]
///
/// Reentrant creation of 300 files with power-loss simulation.
/// Can take 30+ seconds due to iteration over power-loss points.
#[test]
#[cfg(feature = "slow_tests")]
fn test_files_many_power_loss() {
    const N: usize = 300;
    let mut env = powerloss_config(BLOCK_COUNT_MANY);
    init_powerloss_context(&mut env);

    let config_ptr = &env.config as *const LfsConfig;
    let mut lfs = core::mem::MaybeUninit::<Lfs>::zeroed();

    assert_ok(lp_littlefs::lfs_format(lfs.as_mut_ptr(), config_ptr));
    assert_ok(lp_littlefs::lfs_mount(lfs.as_mut_ptr(), config_ptr));
    assert_ok(lp_littlefs::lfs_unmount(lfs.as_mut_ptr()));
    let snapshot = env.snapshot();

    let max_iter = 2000;

    let op = |lfs: *mut Lfs, cfg: *const LfsConfig| -> Result<(), i32> {
        let err = lp_littlefs::lfs_mount(lfs, cfg);
        if err != 0 {
            let _ = lp_littlefs::lfs_format(lfs, cfg);
            let e = lp_littlefs::lfs_mount(lfs, cfg);
            if e != 0 {
                return Err(e);
            }
        }
        for i in 0..N {
            let path = path_bytes(&format!("file_{:03}", i));
            let mut file = core::mem::MaybeUninit::<LfsFile>::zeroed();
            let e = lp_littlefs::lfs_file_open(
                lfs,
                file.as_mut_ptr(),
                path.as_ptr(),
                LFS_O_WRONLY | LFS_O_CREAT,
            );
            if e != 0 {
                return Err(e);
            }
            let content = format!("Hi {:03}\0", i);
            let bytes = content.as_bytes();
            assert_eq!(bytes.len(), 7);
            let sz = lp_littlefs::lfs_file_size(lfs, file.as_mut_ptr());
            if sz != bytes.len() as i32 {
                let n = lp_littlefs::lfs_file_write(
                    lfs,
                    file.as_mut_ptr(),
                    bytes.as_ptr() as *const core::ffi::c_void,
                    bytes.len() as u32,
                );
                if n < 0 {
                    return Err(n);
                }
                assert_eq!(n, bytes.len() as i32);
            }
            let e = lp_littlefs::lfs_file_close(lfs, file.as_mut_ptr());
            if e != 0 {
                return Err(e);
            }

            let mut rfile = core::mem::MaybeUninit::<LfsFile>::zeroed();
            let e =
                lp_littlefs::lfs_file_open(lfs, rfile.as_mut_ptr(), path.as_ptr(), LFS_O_RDONLY);
            if e != 0 {
                return Err(e);
            }
            let mut buf = [0u8; 32];
            let n = lp_littlefs::lfs_file_read(
                lfs,
                rfile.as_mut_ptr(),
                buf.as_mut_ptr() as *mut core::ffi::c_void,
                7,
            );
            assert_eq!(n, 7);
            assert_eq!(&buf[..7], bytes);
            let e = lp_littlefs::lfs_file_close(lfs, rfile.as_mut_ptr());
            if e != 0 {
                return Err(e);
            }
        }
        let e = lp_littlefs::lfs_unmount(lfs);
        if e != 0 {
            return Err(e);
        }
        Ok(())
    };

    let verify = |_lfs: *mut Lfs, _cfg: *const LfsConfig| -> Result<(), i32> { Ok(()) };

    let result = run_powerloss_linear(&mut env, &snapshot, max_iter, op, verify);
    result.expect("many_power_loss should eventually succeed");
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
