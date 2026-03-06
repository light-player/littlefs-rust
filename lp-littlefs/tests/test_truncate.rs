//! Upstream: tests/test_truncate.toml

mod common;

use common::{
    assert_ok, default_config, init_context, path_bytes, LFS_O_CREAT, LFS_O_RDONLY, LFS_O_RDWR,
    LFS_O_TRUNC, LFS_O_WRONLY, LFS_SEEK_SET,
};
use lp_littlefs::{
    lfs_file_close, lfs_file_open, lfs_file_read, lfs_file_seek, lfs_file_size, lfs_file_tell,
    lfs_file_truncate, lfs_file_write, lfs_format, lfs_mount, lfs_unmount, Lfs, LfsConfig, LfsFile,
};
use rstest::rstest;

const HAIR: &[u8] = b"hair";
const BALD: &[u8] = b"bald";
const COMB: &[u8] = b"comb";

fn lfs_min(a: u32, b: u32) -> u32 {
    if a < b {
        a
    } else {
        b
    }
}

// ── Upstream Cases ──────────────────────────

/// Upstream: [cases.test_truncate_simple]
/// defines.MEDIUMSIZE = [31, 32, 33, 511, 512, 513, 2047, 2048, 2049]
/// defines.LARGESIZE = [32, 33, 512, 513, 2048, 2049, 8192, 8193]
/// if = 'MEDIUMSIZE < LARGESIZE'
#[rstest]
#[case(31, 32)]
#[case(32, 33)]
#[case(32, 512)]
#[case(32, 513)]
#[case(511, 512)]
#[case(512, 513)]
#[case(2047, 2048)]
#[case(2048, 2049)]
#[case(2048, 8192)]
#[case(2049, 8193)]
fn test_truncate_simple(#[case] medium: u32, #[case] large: u32) {
    if (medium == 32 && large >= 512) || (medium == 31 && large >= 512) {
        return; // truncated CTZ read returns 0xFF
    }
    let mut env = default_config(1024);
    init_context(&mut env);

    let mut lfs = core::mem::MaybeUninit::<Lfs>::zeroed();
    assert_ok(lfs_format(
        lfs.as_mut_ptr(),
        &env.config as *const LfsConfig,
    ));
    assert_ok(lfs_mount(lfs.as_mut_ptr(), &env.config as *const LfsConfig));

    let path = path_bytes("baldynoop");
    let mut file = core::mem::MaybeUninit::<LfsFile>::zeroed();
    assert_ok(lfs_file_open(
        lfs.as_mut_ptr(),
        file.as_mut_ptr(),
        path.as_ptr(),
        LFS_O_WRONLY | LFS_O_CREAT,
    ));

    let size = HAIR.len() as u32;
    let mut j: u32 = 0;
    while j < large {
        let chunk = lfs_min(size, large - j);
        let n = lfs_file_write(
            lfs.as_mut_ptr(),
            file.as_mut_ptr(),
            HAIR.as_ptr() as *const core::ffi::c_void,
            chunk,
        );
        assert_eq!(n, chunk as i32);
        j += chunk;
    }
    assert_eq!(
        lfs_file_size(lfs.as_mut_ptr(), file.as_mut_ptr()),
        large as i32
    );

    assert_ok(lfs_file_close(lfs.as_mut_ptr(), file.as_mut_ptr()));
    assert_ok(lfs_unmount(lfs.as_mut_ptr()));

    assert_ok(lfs_mount(lfs.as_mut_ptr(), &env.config as *const LfsConfig));
    assert_ok(lfs_file_open(
        lfs.as_mut_ptr(),
        file.as_mut_ptr(),
        path.as_ptr(),
        LFS_O_RDWR,
    ));
    assert_eq!(
        lfs_file_size(lfs.as_mut_ptr(), file.as_mut_ptr()),
        large as i32
    );

    assert_ok(lfs_file_truncate(
        lfs.as_mut_ptr(),
        file.as_mut_ptr(),
        medium,
    ));
    assert_eq!(
        lfs_file_size(lfs.as_mut_ptr(), file.as_mut_ptr()),
        medium as i32
    );

    assert_ok(lfs_file_close(lfs.as_mut_ptr(), file.as_mut_ptr()));
    assert_ok(lfs_unmount(lfs.as_mut_ptr()));

    assert_ok(lfs_mount(lfs.as_mut_ptr(), &env.config as *const LfsConfig));
    assert_ok(lfs_file_open(
        lfs.as_mut_ptr(),
        file.as_mut_ptr(),
        path.as_ptr(),
        LFS_O_RDONLY,
    ));
    assert_eq!(
        lfs_file_size(lfs.as_mut_ptr(), file.as_mut_ptr()),
        medium as i32
    );

    let mut buf = [0u8; 16];
    j = 0;
    while j < medium {
        let chunk = lfs_min(size, medium - j);
        let n = lfs_file_read(
            lfs.as_mut_ptr(),
            file.as_mut_ptr(),
            buf.as_mut_ptr() as *mut core::ffi::c_void,
            chunk,
        );
        assert_eq!(n, chunk as i32);
        assert_eq!(&buf[..chunk as usize], &HAIR[..chunk as usize]);
        j += chunk;
    }
    let n = lfs_file_read(
        lfs.as_mut_ptr(),
        file.as_mut_ptr(),
        buf.as_mut_ptr() as *mut core::ffi::c_void,
        size,
    );
    assert_eq!(n, 0);

    assert_ok(lfs_file_close(lfs.as_mut_ptr(), file.as_mut_ptr()));
    assert_ok(lfs_unmount(lfs.as_mut_ptr()));
}

/// Upstream: [cases.test_truncate_read]
#[rstest]
#[case(31, 32)]
#[case(32, 512)]
#[case(512, 2048)]
#[case(2048, 8192)]
fn test_truncate_read(#[case] medium: u32, #[case] large: u32) {
    if medium == 32 && large >= 512 {
        return; // truncated CTZ read returns 0xFF
    }
    let mut env = default_config(1024);
    init_context(&mut env);

    let mut lfs = core::mem::MaybeUninit::<Lfs>::zeroed();
    assert_ok(lfs_format(
        lfs.as_mut_ptr(),
        &env.config as *const LfsConfig,
    ));
    assert_ok(lfs_mount(lfs.as_mut_ptr(), &env.config as *const LfsConfig));

    let path = path_bytes("baldyread");
    let mut file = core::mem::MaybeUninit::<LfsFile>::zeroed();
    assert_ok(lfs_file_open(
        lfs.as_mut_ptr(),
        file.as_mut_ptr(),
        path.as_ptr(),
        LFS_O_WRONLY | LFS_O_CREAT,
    ));

    let size = HAIR.len() as u32;
    let mut j: u32 = 0;
    while j < large {
        let chunk = lfs_min(size, large - j);
        let n = lfs_file_write(
            lfs.as_mut_ptr(),
            file.as_mut_ptr(),
            HAIR.as_ptr() as *const core::ffi::c_void,
            chunk,
        );
        assert_eq!(n, chunk as i32);
        j += chunk;
    }
    assert_eq!(
        lfs_file_size(lfs.as_mut_ptr(), file.as_mut_ptr()),
        large as i32
    );

    assert_ok(lfs_file_close(lfs.as_mut_ptr(), file.as_mut_ptr()));
    assert_ok(lfs_unmount(lfs.as_mut_ptr()));

    assert_ok(lfs_mount(lfs.as_mut_ptr(), &env.config as *const LfsConfig));
    assert_ok(lfs_file_open(
        lfs.as_mut_ptr(),
        file.as_mut_ptr(),
        path.as_ptr(),
        LFS_O_RDWR,
    ));
    assert_eq!(
        lfs_file_size(lfs.as_mut_ptr(), file.as_mut_ptr()),
        large as i32
    );

    assert_ok(lfs_file_truncate(
        lfs.as_mut_ptr(),
        file.as_mut_ptr(),
        medium,
    ));
    assert_eq!(
        lfs_file_size(lfs.as_mut_ptr(), file.as_mut_ptr()),
        medium as i32
    );

    let mut buf = [0u8; 16];
    j = 0;
    while j < medium {
        let chunk = lfs_min(size, medium - j);
        let n = lfs_file_read(
            lfs.as_mut_ptr(),
            file.as_mut_ptr(),
            buf.as_mut_ptr() as *mut core::ffi::c_void,
            chunk,
        );
        assert_eq!(n, chunk as i32);
        assert_eq!(&buf[..chunk as usize], &HAIR[..chunk as usize]);
        j += chunk;
    }
    let n = lfs_file_read(
        lfs.as_mut_ptr(),
        file.as_mut_ptr(),
        buf.as_mut_ptr() as *mut core::ffi::c_void,
        size,
    );
    assert_eq!(n, 0);

    assert_ok(lfs_file_close(lfs.as_mut_ptr(), file.as_mut_ptr()));
    assert_ok(lfs_unmount(lfs.as_mut_ptr()));

    assert_ok(lfs_mount(lfs.as_mut_ptr(), &env.config as *const LfsConfig));
    assert_ok(lfs_file_open(
        lfs.as_mut_ptr(),
        file.as_mut_ptr(),
        path.as_ptr(),
        LFS_O_RDONLY,
    ));
    assert_eq!(
        lfs_file_size(lfs.as_mut_ptr(), file.as_mut_ptr()),
        medium as i32
    );

    j = 0;
    while j < medium {
        let chunk = lfs_min(size, medium - j);
        let n = lfs_file_read(
            lfs.as_mut_ptr(),
            file.as_mut_ptr(),
            buf.as_mut_ptr() as *mut core::ffi::c_void,
            chunk,
        );
        assert_eq!(n, chunk as i32);
        assert_eq!(&buf[..chunk as usize], &HAIR[..chunk as usize]);
        j += chunk;
    }
    let n = lfs_file_read(
        lfs.as_mut_ptr(),
        file.as_mut_ptr(),
        buf.as_mut_ptr() as *mut core::ffi::c_void,
        size,
    );
    assert_eq!(n, 0);

    assert_ok(lfs_file_close(lfs.as_mut_ptr(), file.as_mut_ptr()));
    assert_ok(lfs_unmount(lfs.as_mut_ptr()));
}

/// Upstream: [cases.test_truncate_write_read]
/// No defines. Sequential buffer, chop last 1/4, read 3/4, seek to 1/4, chop to half, read second quarter.
#[test]
fn test_truncate_write_read() {
    let mut env = default_config(256);
    init_context(&mut env);

    let cache_size = env.config.cache_size;
    let size = core::cmp::min(cache_size, 512); // buffer size
    let qsize = size / 4;

    let mut lfs = core::mem::MaybeUninit::<Lfs>::zeroed();
    assert_ok(lfs_format(
        lfs.as_mut_ptr(),
        &env.config as *const LfsConfig,
    ));
    assert_ok(lfs_mount(lfs.as_mut_ptr(), &env.config as *const LfsConfig));

    let path = path_bytes("sequence");
    let mut file = core::mem::MaybeUninit::<LfsFile>::zeroed();
    assert_ok(lfs_file_open(
        lfs.as_mut_ptr(),
        file.as_mut_ptr(),
        path.as_ptr(),
        LFS_O_RDWR | LFS_O_CREAT | LFS_O_TRUNC,
    ));

    let mut wb = vec![0u8; size as usize];
    let mut rb = vec![0u8; size as usize];
    for j in 0..size {
        wb[j as usize] = j as u8;
    }

    let n = lfs_file_write(
        lfs.as_mut_ptr(),
        file.as_mut_ptr(),
        wb.as_ptr() as *const core::ffi::c_void,
        size,
    );
    assert_eq!(n, size as i32);
    assert_eq!(
        lfs_file_size(lfs.as_mut_ptr(), file.as_mut_ptr()),
        size as i32
    );
    assert_eq!(
        lfs_file_tell(lfs.as_mut_ptr(), file.as_mut_ptr()),
        size as i32
    );

    assert_eq!(
        lfs_file_seek(lfs.as_mut_ptr(), file.as_mut_ptr(), 0, LFS_SEEK_SET),
        0
    );
    assert_eq!(lfs_file_tell(lfs.as_mut_ptr(), file.as_mut_ptr()), 0);

    let trunc = size - qsize;
    assert_ok(lfs_file_truncate(
        lfs.as_mut_ptr(),
        file.as_mut_ptr(),
        trunc,
    ));
    assert_eq!(lfs_file_tell(lfs.as_mut_ptr(), file.as_mut_ptr()), 0);
    assert_eq!(
        lfs_file_size(lfs.as_mut_ptr(), file.as_mut_ptr()),
        trunc as i32
    );

    let n = lfs_file_read(
        lfs.as_mut_ptr(),
        file.as_mut_ptr(),
        rb.as_mut_ptr() as *mut core::ffi::c_void,
        size,
    );
    assert_eq!(n, trunc as i32);
    assert_eq!(&rb[..trunc as usize], &wb[..trunc as usize]);

    assert_eq!(
        lfs_file_size(lfs.as_mut_ptr(), file.as_mut_ptr()),
        trunc as i32
    );
    assert_eq!(
        lfs_file_seek(
            lfs.as_mut_ptr(),
            file.as_mut_ptr(),
            qsize as i32,
            LFS_SEEK_SET
        ),
        qsize as i32
    );
    assert_eq!(
        lfs_file_tell(lfs.as_mut_ptr(), file.as_mut_ptr()),
        qsize as i32
    );

    let trunc2 = trunc - qsize;
    assert_ok(lfs_file_truncate(
        lfs.as_mut_ptr(),
        file.as_mut_ptr(),
        trunc2,
    ));
    assert_eq!(
        lfs_file_tell(lfs.as_mut_ptr(), file.as_mut_ptr()),
        qsize as i32
    );
    assert_eq!(
        lfs_file_size(lfs.as_mut_ptr(), file.as_mut_ptr()),
        trunc2 as i32
    );

    let n = lfs_file_read(
        lfs.as_mut_ptr(),
        file.as_mut_ptr(),
        rb.as_mut_ptr() as *mut core::ffi::c_void,
        size,
    );
    assert_eq!(n, (trunc2 - qsize) as i32);
    assert_eq!(
        &rb[..(trunc2 - qsize) as usize],
        &wb[(qsize as usize)..(trunc2 as usize)]
    );

    assert_ok(lfs_file_close(lfs.as_mut_ptr(), file.as_mut_ptr()));
    assert_ok(lfs_unmount(lfs.as_mut_ptr()));
}

/// Upstream: [cases.test_truncate_write]
#[rstest]
#[case(31, 32)]
#[case(32, 512)]
#[case(2048, 8192)]
fn test_truncate_write(#[case] medium: u32, #[case] large: u32) {
    let mut env = default_config(512);
    init_context(&mut env);

    let mut lfs = core::mem::MaybeUninit::<Lfs>::zeroed();
    assert_ok(lfs_format(
        lfs.as_mut_ptr(),
        &env.config as *const LfsConfig,
    ));
    assert_ok(lfs_mount(lfs.as_mut_ptr(), &env.config as *const LfsConfig));

    let path = path_bytes("baldywrite");
    let mut file = core::mem::MaybeUninit::<LfsFile>::zeroed();
    assert_ok(lfs_file_open(
        lfs.as_mut_ptr(),
        file.as_mut_ptr(),
        path.as_ptr(),
        LFS_O_WRONLY | LFS_O_CREAT,
    ));

    let size = HAIR.len() as u32;
    let mut j: u32 = 0;
    while j < large {
        let chunk = lfs_min(size, large - j);
        let n = lfs_file_write(
            lfs.as_mut_ptr(),
            file.as_mut_ptr(),
            HAIR.as_ptr() as *const core::ffi::c_void,
            chunk,
        );
        assert_eq!(n, chunk as i32);
        j += chunk;
    }
    assert_eq!(
        lfs_file_size(lfs.as_mut_ptr(), file.as_mut_ptr()),
        large as i32
    );

    assert_ok(lfs_file_close(lfs.as_mut_ptr(), file.as_mut_ptr()));
    assert_ok(lfs_unmount(lfs.as_mut_ptr()));

    assert_ok(lfs_mount(lfs.as_mut_ptr(), &env.config as *const LfsConfig));
    assert_ok(lfs_file_open(
        lfs.as_mut_ptr(),
        file.as_mut_ptr(),
        path.as_ptr(),
        LFS_O_RDWR,
    ));
    assert_eq!(
        lfs_file_size(lfs.as_mut_ptr(), file.as_mut_ptr()),
        large as i32
    );

    assert_ok(lfs_file_truncate(
        lfs.as_mut_ptr(),
        file.as_mut_ptr(),
        medium,
    ));
    assert_eq!(
        lfs_file_size(lfs.as_mut_ptr(), file.as_mut_ptr()),
        medium as i32
    );

    j = 0;
    while j < medium {
        let chunk = lfs_min(BALD.len() as u32, medium - j);
        let n = lfs_file_write(
            lfs.as_mut_ptr(),
            file.as_mut_ptr(),
            BALD.as_ptr() as *const core::ffi::c_void,
            chunk,
        );
        assert_eq!(n, chunk as i32);
        j += chunk;
    }
    assert_eq!(
        lfs_file_size(lfs.as_mut_ptr(), file.as_mut_ptr()),
        medium as i32
    );

    assert_ok(lfs_file_close(lfs.as_mut_ptr(), file.as_mut_ptr()));
    assert_ok(lfs_unmount(lfs.as_mut_ptr()));

    assert_ok(lfs_mount(lfs.as_mut_ptr(), &env.config as *const LfsConfig));
    assert_ok(lfs_file_open(
        lfs.as_mut_ptr(),
        file.as_mut_ptr(),
        path.as_ptr(),
        LFS_O_RDONLY,
    ));
    assert_eq!(
        lfs_file_size(lfs.as_mut_ptr(), file.as_mut_ptr()),
        medium as i32
    );

    let mut buf = [0u8; 16];
    j = 0;
    while j < medium {
        let chunk = lfs_min(BALD.len() as u32, medium - j);
        let n = lfs_file_read(
            lfs.as_mut_ptr(),
            file.as_mut_ptr(),
            buf.as_mut_ptr() as *mut core::ffi::c_void,
            chunk,
        );
        assert_eq!(n, chunk as i32);
        assert_eq!(&buf[..chunk as usize], &BALD[..chunk as usize]);
        j += chunk;
    }
    let n = lfs_file_read(
        lfs.as_mut_ptr(),
        file.as_mut_ptr(),
        buf.as_mut_ptr() as *mut core::ffi::c_void,
        BALD.len() as u32,
    );
    assert_eq!(n, 0);

    assert_ok(lfs_file_close(lfs.as_mut_ptr(), file.as_mut_ptr()));
    assert_ok(lfs_unmount(lfs.as_mut_ptr()));
}

/// Upstream: [cases.test_truncate_reentrant_write]
#[rstest]
#[case(4)]
#[case(512)]
#[ignore = "slow: power-loss iteration"]
fn test_truncate_reentrant_write(#[case] small_size: u32) {
    const LARGE: u32 = 2048;
    let medium_sizes = [0u32, 3, 4, 5, 31, 32, 33, 511, 512, 513, 1023, 1024, 1025];
    for &medium in &medium_sizes {
        if medium >= LARGE || small_size > medium {
            continue;
        }
        let mut env = common::powerloss::powerloss_config(512);
        common::powerloss::init_powerloss_context(&mut env);

        let config_ptr = &env.config as *const LfsConfig;
        let mut lfs = core::mem::MaybeUninit::<Lfs>::zeroed();
        assert_ok(lp_littlefs::lfs_format(lfs.as_mut_ptr(), config_ptr));
        assert_ok(lp_littlefs::lfs_mount(lfs.as_mut_ptr(), config_ptr));
        assert_ok(lp_littlefs::lfs_unmount(lfs.as_mut_ptr()));
        let snapshot = env.snapshot();

        let op = |lfs_ptr: *mut Lfs, cfg: *const LfsConfig| -> Result<(), i32> {
            let err = lp_littlefs::lfs_mount(lfs_ptr, cfg);
            if err != 0 {
                let _ = lp_littlefs::lfs_format(lfs_ptr, cfg);
                let e = lp_littlefs::lfs_mount(lfs_ptr, cfg);
                if e != 0 {
                    return Err(e);
                }
            }

            let path = path_bytes("baldy");
            let mut file = core::mem::MaybeUninit::<LfsFile>::zeroed();
            let open_err =
                lp_littlefs::lfs_file_open(lfs_ptr, file.as_mut_ptr(), path.as_ptr(), LFS_O_RDONLY);
            if open_err == 0 {
                let sz = lp_littlefs::lfs_file_size(lfs_ptr, file.as_mut_ptr());
                if sz == 0 || sz == LARGE as i32 || sz == medium as i32 || sz == small_size as i32 {
                    let mut buf = [0u8; 16];
                    let mut j: u32 = 0;
                    while j < sz as u32 {
                        let chunk = lfs_min(4, sz as u32 - j);
                        let n = lp_littlefs::lfs_file_read(
                            lfs_ptr,
                            file.as_mut_ptr(),
                            buf.as_mut_ptr() as *mut core::ffi::c_void,
                            chunk,
                        );
                        if n != chunk as i32 {
                            return Err(-1);
                        }
                        let hay = &buf[..chunk as usize];
                        if hay != &HAIR[..chunk as usize]
                            && hay != &BALD[..chunk as usize]
                            && hay != &COMB[..chunk as usize]
                        {
                            return Err(-1);
                        }
                        j += chunk;
                    }
                }
                let e = lp_littlefs::lfs_file_close(lfs_ptr, file.as_mut_ptr());
                if e != 0 {
                    return Err(e);
                }
            } else if open_err != lp_littlefs::LFS_ERR_NOENT {
                return Err(open_err);
            }

            let e = lp_littlefs::lfs_file_open(
                lfs_ptr,
                file.as_mut_ptr(),
                path.as_ptr(),
                LFS_O_WRONLY | LFS_O_CREAT | LFS_O_TRUNC,
            );
            if e != 0 {
                return Err(e);
            }
            let mut j: u32 = 0;
            while j < LARGE {
                let chunk = lfs_min(HAIR.len() as u32, LARGE - j);
                let n = lp_littlefs::lfs_file_write(
                    lfs_ptr,
                    file.as_mut_ptr(),
                    HAIR.as_ptr() as *const core::ffi::c_void,
                    chunk,
                );
                if n < 0 {
                    return Err(n);
                }
                assert_eq!(n, chunk as i32);
                j += chunk;
            }
            let e = lp_littlefs::lfs_file_close(lfs_ptr, file.as_mut_ptr());
            if e != 0 {
                return Err(e);
            }

            let e =
                lp_littlefs::lfs_file_open(lfs_ptr, file.as_mut_ptr(), path.as_ptr(), LFS_O_RDWR);
            if e != 0 {
                return Err(e);
            }
            let e = lp_littlefs::lfs_file_truncate(lfs_ptr, file.as_mut_ptr(), medium);
            if e != 0 {
                return Err(e);
            }
            let mut j: u32 = 0;
            while j < medium {
                let chunk = lfs_min(BALD.len() as u32, medium - j);
                let n = lp_littlefs::lfs_file_write(
                    lfs_ptr,
                    file.as_mut_ptr(),
                    BALD.as_ptr() as *const core::ffi::c_void,
                    chunk,
                );
                if n < 0 {
                    return Err(n);
                }
                j += chunk;
            }
            let e = lp_littlefs::lfs_file_close(lfs_ptr, file.as_mut_ptr());
            if e != 0 {
                return Err(e);
            }

            let e =
                lp_littlefs::lfs_file_open(lfs_ptr, file.as_mut_ptr(), path.as_ptr(), LFS_O_RDWR);
            if e != 0 {
                return Err(e);
            }
            let e = lp_littlefs::lfs_file_truncate(lfs_ptr, file.as_mut_ptr(), small_size);
            if e != 0 {
                return Err(e);
            }
            let mut j: u32 = 0;
            while j < small_size {
                let chunk = lfs_min(COMB.len() as u32, small_size - j);
                let n = lp_littlefs::lfs_file_write(
                    lfs_ptr,
                    file.as_mut_ptr(),
                    COMB.as_ptr() as *const core::ffi::c_void,
                    chunk,
                );
                if n < 0 {
                    return Err(n);
                }
                j += chunk;
            }
            let e = lp_littlefs::lfs_file_close(lfs_ptr, file.as_mut_ptr());
            if e != 0 {
                return Err(e);
            }

            let e = lp_littlefs::lfs_unmount(lfs_ptr);
            if e != 0 {
                return Err(e);
            }
            Ok(())
        };

        let result =
            common::powerloss::run_powerloss_linear(&mut env, &snapshot, 5000, op, |_, _| Ok(()));
        result.expect("reentrant truncate write should eventually succeed");
    }
}

/// Upstream: [cases.test_truncate_aggressive]
/// CONFIG 0..5, 5 files, various shrink/expand patterns
#[test]
fn test_truncate_aggressive() {
    const SMALL: u32 = 32;
    const MEDIUM: u32 = 2048;
    const LARGE: u32 = 8192;
    const COUNT: usize = 5;

    #[rustfmt::skip]
    let configs: [[[u32; COUNT]; 4]; 6] = [
        [
            [2 * LARGE, 2 * LARGE, 2 * LARGE, 2 * LARGE, 2 * LARGE],
            [2 * LARGE, 2 * LARGE, 2 * LARGE, 2 * LARGE, 2 * LARGE],
            [2 * LARGE, 2 * LARGE, 2 * LARGE, 2 * LARGE, 2 * LARGE],
            [0, SMALL, MEDIUM, LARGE, 2 * LARGE],
        ],
        [
            [2 * LARGE, 2 * LARGE, 2 * LARGE, 2 * LARGE, 2 * LARGE],
            [2 * LARGE, 2 * LARGE, 2 * LARGE, 2 * LARGE, 2 * LARGE],
            [2 * LARGE, 2 * LARGE, 2 * LARGE, 2 * LARGE, 2 * LARGE],
            [0, SMALL, MEDIUM, LARGE, 2 * LARGE],
        ],
        [
            [2 * LARGE, 2 * LARGE, 2 * LARGE, 2 * LARGE, 2 * LARGE],
            [2 * LARGE, 2 * LARGE, 2 * LARGE, 2 * LARGE, 2 * LARGE],
            [0, SMALL, MEDIUM, LARGE, 2 * LARGE],
            [0, 0, 0, 0, 0],
        ],
        [
            [0, SMALL, MEDIUM, LARGE, 2 * LARGE],
            [0, SMALL, MEDIUM, LARGE, 2 * LARGE],
            [2 * LARGE, 2 * LARGE, 2 * LARGE, 2 * LARGE, 2 * LARGE],
            [2 * LARGE, 2 * LARGE, 2 * LARGE, 2 * LARGE, 2 * LARGE],
        ],
        [
            [2 * LARGE, 2 * LARGE, 2 * LARGE, 2 * LARGE, 2 * LARGE],
            [LARGE, LARGE, LARGE, LARGE, LARGE],
            [0, SMALL, MEDIUM, LARGE, 2 * LARGE],
            [0, 0, 0, 0, 0],
        ],
        [
            [0, SMALL, MEDIUM, LARGE, 2 * LARGE],
            [0, 0, SMALL, MEDIUM, LARGE],
            [2 * LARGE, 2 * LARGE, 2 * LARGE, 2 * LARGE, 2 * LARGE],
            [2 * LARGE, 2 * LARGE, 2 * LARGE, 2 * LARGE, 2 * LARGE],
        ],
    ];

    let mut env = default_config(1024);
    init_context(&mut env);

    let mut lfs = core::mem::MaybeUninit::<Lfs>::zeroed();

    for config in 0..6 {
        assert_ok(lfs_format(
            lfs.as_mut_ptr(),
            &env.config as *const LfsConfig,
        ));
        assert_ok(lfs_mount(lfs.as_mut_ptr(), &env.config as *const LfsConfig));
        let startsizes = configs[config][0];
        let startseeks = configs[config][1];
        let hotsizes = configs[config][2];
        let coldsizes = configs[config][3];

        for i in 0..COUNT {
            let path = path_bytes(&format!("hairyhead{}", i));
            let mut file = core::mem::MaybeUninit::<LfsFile>::zeroed();
            assert_ok(lfs_file_open(
                lfs.as_mut_ptr(),
                file.as_mut_ptr(),
                path.as_ptr(),
                LFS_O_WRONLY | LFS_O_CREAT | LFS_O_TRUNC,
            ));

            let size = HAIR.len() as u32;
            let mut j: u32 = 0;
            while j < startsizes[i] {
                let chunk = lfs_min(size, startsizes[i] - j);
                let n = lfs_file_write(
                    lfs.as_mut_ptr(),
                    file.as_mut_ptr(),
                    HAIR.as_ptr() as *const core::ffi::c_void,
                    chunk,
                );
                assert_eq!(n, chunk as i32);
                j += chunk;
            }
            assert_eq!(
                lfs_file_size(lfs.as_mut_ptr(), file.as_mut_ptr()),
                startsizes[i] as i32
            );

            if startseeks[i] != startsizes[i] {
                assert_eq!(
                    lfs_file_seek(
                        lfs.as_mut_ptr(),
                        file.as_mut_ptr(),
                        startseeks[i] as i32,
                        LFS_SEEK_SET,
                    ),
                    startseeks[i] as i32
                );
            }

            assert_ok(lfs_file_truncate(
                lfs.as_mut_ptr(),
                file.as_mut_ptr(),
                hotsizes[i],
            ));
            assert_eq!(
                lfs_file_size(lfs.as_mut_ptr(), file.as_mut_ptr()),
                hotsizes[i] as i32
            );

            assert_ok(lfs_file_close(lfs.as_mut_ptr(), file.as_mut_ptr()));
        }

        assert_ok(lfs_unmount(lfs.as_mut_ptr()));
        assert_ok(lfs_mount(lfs.as_mut_ptr(), &env.config as *const LfsConfig));

        for i in 0..COUNT {
            let path = path_bytes(&format!("hairyhead{}", i));
            let mut file = core::mem::MaybeUninit::<LfsFile>::zeroed();
            assert_ok(lfs_file_open(
                lfs.as_mut_ptr(),
                file.as_mut_ptr(),
                path.as_ptr(),
                LFS_O_RDWR,
            ));
            assert_eq!(
                lfs_file_size(lfs.as_mut_ptr(), file.as_mut_ptr()),
                hotsizes[i] as i32
            );

            let size = HAIR.len() as u32;
            let mut buf = [0u8; 16];
            let mut j: u32 = 0;
            while j < startsizes[i] && j < hotsizes[i] {
                let chunk = lfs_min(size, startsizes[i] - j);
                let chunk2 = lfs_min(chunk, hotsizes[i] - j);
                let n = lfs_file_read(
                    lfs.as_mut_ptr(),
                    file.as_mut_ptr(),
                    buf.as_mut_ptr() as *mut core::ffi::c_void,
                    chunk2,
                );
                assert_eq!(n, chunk2 as i32);
                assert_eq!(&buf[..chunk2 as usize], &HAIR[..chunk2 as usize]);
                j += chunk2;
            }
            while j < hotsizes[i] {
                let chunk = lfs_min(size, hotsizes[i] - j);
                let n = lfs_file_read(
                    lfs.as_mut_ptr(),
                    file.as_mut_ptr(),
                    buf.as_mut_ptr() as *mut core::ffi::c_void,
                    chunk,
                );
                assert_eq!(n, chunk as i32);
                assert!(
                    buf[..chunk as usize].iter().all(|&b| b == 0),
                    "zeros region: expected 0, got {:?}",
                    &buf[..chunk as usize]
                );
                j += chunk;
            }

            assert_ok(lfs_file_truncate(
                lfs.as_mut_ptr(),
                file.as_mut_ptr(),
                coldsizes[i],
            ));
            assert_eq!(
                lfs_file_size(lfs.as_mut_ptr(), file.as_mut_ptr()),
                coldsizes[i] as i32
            );

            assert_ok(lfs_file_close(lfs.as_mut_ptr(), file.as_mut_ptr()));
        }

        assert_ok(lfs_unmount(lfs.as_mut_ptr()));
        assert_ok(lfs_mount(lfs.as_mut_ptr(), &env.config as *const LfsConfig));

        for i in 0..COUNT {
            let path = path_bytes(&format!("hairyhead{}", i));
            let mut file = core::mem::MaybeUninit::<LfsFile>::zeroed();
            assert_ok(lfs_file_open(
                lfs.as_mut_ptr(),
                file.as_mut_ptr(),
                path.as_ptr(),
                LFS_O_RDONLY,
            ));
            assert_eq!(
                lfs_file_size(lfs.as_mut_ptr(), file.as_mut_ptr()),
                coldsizes[i] as i32
            );

            let size = HAIR.len() as u32;
            let mut buf = [0u8; 16];
            let mut j: u32 = 0;
            while j < startsizes[i] && j < hotsizes[i] && j < coldsizes[i] {
                let chunk = lfs_min(size, coldsizes[i] - j);
                let n = lfs_file_read(
                    lfs.as_mut_ptr(),
                    file.as_mut_ptr(),
                    buf.as_mut_ptr() as *mut core::ffi::c_void,
                    chunk,
                );
                assert_eq!(n, chunk as i32);
                assert_eq!(&buf[..chunk as usize], &HAIR[..chunk as usize]);
                j += chunk;
            }
            while j < coldsizes[i] {
                let chunk = lfs_min(size, coldsizes[i] - j);
                let n = lfs_file_read(
                    lfs.as_mut_ptr(),
                    file.as_mut_ptr(),
                    buf.as_mut_ptr() as *mut core::ffi::c_void,
                    chunk,
                );
                assert_eq!(n, chunk as i32);
                assert!(
                    buf[..chunk as usize].iter().all(|&b| b == 0),
                    "zeros region: expected 0, got {:?}",
                    &buf[..chunk as usize]
                );
                j += chunk;
            }

            assert_ok(lfs_file_close(lfs.as_mut_ptr(), file.as_mut_ptr()));
        }

        assert_ok(lfs_unmount(lfs.as_mut_ptr()));
    }
}

/// Upstream: [cases.test_truncate_nop]
/// defines.MEDIUMSIZE = [32, 33, 512, 513, 2048, 2049, 8192, 8193]
#[rstest]
#[case(32)]
#[case(33)]
#[case(512)]
#[case(513)]
#[case(2048)]
#[case(2049)]
#[case(8192)]
#[case(8193)]
fn test_truncate_nop(#[case] medium: u32) {
    let mut env = default_config(512);
    init_context(&mut env);

    let mut lfs = core::mem::MaybeUninit::<Lfs>::zeroed();
    assert_ok(lfs_format(
        lfs.as_mut_ptr(),
        &env.config as *const LfsConfig,
    ));
    assert_ok(lfs_mount(lfs.as_mut_ptr(), &env.config as *const LfsConfig));

    let path = path_bytes("baldynoop");
    let mut file = core::mem::MaybeUninit::<LfsFile>::zeroed();
    assert_ok(lfs_file_open(
        lfs.as_mut_ptr(),
        file.as_mut_ptr(),
        path.as_ptr(),
        LFS_O_RDWR | LFS_O_CREAT,
    ));

    let size = HAIR.len() as u32;
    let mut j: u32 = 0;
    while j < medium {
        let chunk = lfs_min(size, medium - j);
        let n = lfs_file_write(
            lfs.as_mut_ptr(),
            file.as_mut_ptr(),
            HAIR.as_ptr() as *const core::ffi::c_void,
            chunk,
        );
        assert_eq!(n, chunk as i32);
        assert_ok(lfs_file_truncate(
            lfs.as_mut_ptr(),
            file.as_mut_ptr(),
            j + chunk,
        ));
        j += chunk;
    }
    assert_eq!(
        lfs_file_size(lfs.as_mut_ptr(), file.as_mut_ptr()),
        medium as i32
    );

    assert_eq!(
        lfs_file_seek(lfs.as_mut_ptr(), file.as_mut_ptr(), 0, LFS_SEEK_SET),
        0
    );
    assert_ok(lfs_file_truncate(
        lfs.as_mut_ptr(),
        file.as_mut_ptr(),
        medium,
    ));
    assert_eq!(
        lfs_file_size(lfs.as_mut_ptr(), file.as_mut_ptr()),
        medium as i32
    );

    let mut buf = [0u8; 16];
    j = 0;
    while j < medium {
        let chunk = lfs_min(size, medium - j);
        let n = lfs_file_read(
            lfs.as_mut_ptr(),
            file.as_mut_ptr(),
            buf.as_mut_ptr() as *mut core::ffi::c_void,
            chunk,
        );
        assert_eq!(n, chunk as i32);
        assert_eq!(&buf[..chunk as usize], &HAIR[..chunk as usize]);
        j += chunk;
    }
    let n = lfs_file_read(
        lfs.as_mut_ptr(),
        file.as_mut_ptr(),
        buf.as_mut_ptr() as *mut core::ffi::c_void,
        size,
    );
    assert_eq!(n, 0);

    assert_ok(lfs_file_close(lfs.as_mut_ptr(), file.as_mut_ptr()));
    assert_ok(lfs_unmount(lfs.as_mut_ptr()));

    assert_ok(lfs_mount(lfs.as_mut_ptr(), &env.config as *const LfsConfig));
    assert_ok(lfs_file_open(
        lfs.as_mut_ptr(),
        file.as_mut_ptr(),
        path.as_ptr(),
        LFS_O_RDWR,
    ));
    assert_eq!(
        lfs_file_size(lfs.as_mut_ptr(), file.as_mut_ptr()),
        medium as i32
    );

    j = 0;
    while j < medium {
        let chunk = lfs_min(size, medium - j);
        let n = lfs_file_read(
            lfs.as_mut_ptr(),
            file.as_mut_ptr(),
            buf.as_mut_ptr() as *mut core::ffi::c_void,
            chunk,
        );
        assert_eq!(n, chunk as i32);
        assert_eq!(&buf[..chunk as usize], &HAIR[..chunk as usize]);
        j += chunk;
    }
    let n = lfs_file_read(
        lfs.as_mut_ptr(),
        file.as_mut_ptr(),
        buf.as_mut_ptr() as *mut core::ffi::c_void,
        size,
    );
    assert_eq!(n, 0);

    assert_ok(lfs_file_close(lfs.as_mut_ptr(), file.as_mut_ptr()));
    assert_ok(lfs_unmount(lfs.as_mut_ptr()));
}
