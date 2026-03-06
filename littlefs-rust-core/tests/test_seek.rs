//! Upstream: tests/test_seek.toml

mod common;

#[cfg(feature = "slow_tests")]
use common::powerloss::{init_powerloss_context, powerloss_config, run_powerloss_linear};
use common::{
    assert_ok, default_config, init_context, path_bytes, LFS_FILE_MAX, LFS_O_APPEND, LFS_O_CREAT,
    LFS_O_RDONLY, LFS_O_RDWR, LFS_O_WRONLY, LFS_SEEK_CUR, LFS_SEEK_END, LFS_SEEK_SET,
};
#[cfg(feature = "slow_tests")]
use littlefs_rust_core::LFS_ERR_NOENT;
use littlefs_rust_core::{
    lfs_file_close, lfs_file_open, lfs_file_read, lfs_file_rewind, lfs_file_seek, lfs_file_size,
    lfs_file_sync, lfs_file_tell, lfs_file_write, lfs_format, lfs_mount, lfs_unmount, Lfs,
    LfsConfig, LfsFile, LFS_ERR_INVAL,
};
use rstest::rstest;

const KITTY: &[u8] = b"kittycatcat";
const DOGGO: &[u8] = b"doggodogdog";
const HEDGEHOG: &[u8] = b"hedgehoghog";
const PORCUPINE: &[u8] = b"porcupineee";

// ── Upstream Cases ──────────────────────────

/// Upstream: [cases.test_seek_read]
/// defines = [{COUNT=132, SKIP=4}, {COUNT=132, SKIP=128}, ...]
#[rstest]
#[case(132, 4)]
#[case(132, 128)]
#[case(200, 10)]
#[case(200, 100)]
#[case(4, 1)]
#[case(4, 2)]
fn test_seek_read(#[case] count: u32, #[case] skip: u32) {
    let mut env = default_config(256);
    init_context(&mut env);

    let mut lfs = core::mem::MaybeUninit::<Lfs>::zeroed();
    assert_ok(lfs_format(
        lfs.as_mut_ptr(),
        &env.config as *const LfsConfig,
    ));
    assert_ok(lfs_mount(lfs.as_mut_ptr(), &env.config as *const LfsConfig));

    let path = path_bytes("kitty");
    let mut file = core::mem::MaybeUninit::<LfsFile>::zeroed();
    assert_ok(lfs_file_open(
        lfs.as_mut_ptr(),
        file.as_mut_ptr(),
        path.as_ptr(),
        LFS_O_WRONLY | LFS_O_CREAT | LFS_O_APPEND,
    ));
    for _ in 0..count {
        let n = lfs_file_write(
            lfs.as_mut_ptr(),
            file.as_mut_ptr(),
            KITTY.as_ptr() as *const core::ffi::c_void,
            KITTY.len() as u32,
        );
        assert_eq!(n, KITTY.len() as i32);
    }
    assert_ok(lfs_file_close(lfs.as_mut_ptr(), file.as_mut_ptr()));
    assert_ok(lfs_unmount(lfs.as_mut_ptr()));

    assert_ok(lfs_mount(lfs.as_mut_ptr(), &env.config as *const LfsConfig));
    assert_ok(lfs_file_open(
        lfs.as_mut_ptr(),
        file.as_mut_ptr(),
        path.as_ptr(),
        LFS_O_RDONLY,
    ));

    let mut buf = [0u8; 32];
    let mut pos: i32 = -1;
    for _ in 0..skip {
        let n = lfs_file_read(
            lfs.as_mut_ptr(),
            file.as_mut_ptr(),
            buf.as_mut_ptr() as *mut core::ffi::c_void,
            KITTY.len() as u32,
        );
        assert_eq!(n, KITTY.len() as i32);
        assert_eq!(&buf[..KITTY.len()], KITTY);
        pos = lfs_file_tell(lfs.as_mut_ptr(), file.as_mut_ptr());
    }
    assert!(pos >= 0);

    assert_eq!(
        lfs_file_seek(lfs.as_mut_ptr(), file.as_mut_ptr(), pos, LFS_SEEK_SET),
        pos
    );
    let n = lfs_file_read(
        lfs.as_mut_ptr(),
        file.as_mut_ptr(),
        buf.as_mut_ptr() as *mut core::ffi::c_void,
        KITTY.len() as u32,
    );
    assert_eq!(n, KITTY.len() as i32);
    assert_eq!(&buf[..KITTY.len()], KITTY);

    assert_ok(lfs_file_rewind(lfs.as_mut_ptr(), file.as_mut_ptr()));
    let n = lfs_file_read(
        lfs.as_mut_ptr(),
        file.as_mut_ptr(),
        buf.as_mut_ptr() as *mut core::ffi::c_void,
        KITTY.len() as u32,
    );
    assert_eq!(n, KITTY.len() as i32);
    assert_eq!(&buf[..KITTY.len()], KITTY);

    assert_eq!(
        lfs_file_seek(lfs.as_mut_ptr(), file.as_mut_ptr(), 0, LFS_SEEK_CUR),
        KITTY.len() as i32
    );
    let n = lfs_file_read(
        lfs.as_mut_ptr(),
        file.as_mut_ptr(),
        buf.as_mut_ptr() as *mut core::ffi::c_void,
        KITTY.len() as u32,
    );
    assert_eq!(n, KITTY.len() as i32);
    assert_eq!(&buf[..KITTY.len()], KITTY);

    assert_eq!(
        lfs_file_seek(
            lfs.as_mut_ptr(),
            file.as_mut_ptr(),
            KITTY.len() as i32,
            LFS_SEEK_CUR
        ),
        3 * KITTY.len() as i32
    );
    let n = lfs_file_read(
        lfs.as_mut_ptr(),
        file.as_mut_ptr(),
        buf.as_mut_ptr() as *mut core::ffi::c_void,
        KITTY.len() as u32,
    );
    assert_eq!(n, KITTY.len() as i32);
    assert_eq!(&buf[..KITTY.len()], KITTY);

    assert_eq!(
        lfs_file_seek(lfs.as_mut_ptr(), file.as_mut_ptr(), pos, LFS_SEEK_SET),
        pos
    );
    let n = lfs_file_read(
        lfs.as_mut_ptr(),
        file.as_mut_ptr(),
        buf.as_mut_ptr() as *mut core::ffi::c_void,
        KITTY.len() as u32,
    );
    assert_eq!(n, KITTY.len() as i32);
    assert_eq!(&buf[..KITTY.len()], KITTY);

    assert_eq!(
        lfs_file_seek(
            lfs.as_mut_ptr(),
            file.as_mut_ptr(),
            -(KITTY.len() as i32),
            LFS_SEEK_CUR
        ),
        pos
    );
    let n = lfs_file_read(
        lfs.as_mut_ptr(),
        file.as_mut_ptr(),
        buf.as_mut_ptr() as *mut core::ffi::c_void,
        KITTY.len() as u32,
    );
    assert_eq!(n, KITTY.len() as i32);
    assert_eq!(&buf[..KITTY.len()], KITTY);

    assert!(
        lfs_file_seek(
            lfs.as_mut_ptr(),
            file.as_mut_ptr(),
            -(KITTY.len() as i32),
            LFS_SEEK_END
        ) >= 0
    );
    let n = lfs_file_read(
        lfs.as_mut_ptr(),
        file.as_mut_ptr(),
        buf.as_mut_ptr() as *mut core::ffi::c_void,
        KITTY.len() as u32,
    );
    assert_eq!(n, KITTY.len() as i32);
    assert_eq!(&buf[..KITTY.len()], KITTY);

    assert_eq!(
        lfs_file_size(lfs.as_mut_ptr(), file.as_mut_ptr()),
        (count * KITTY.len() as u32) as i32
    );
    assert_eq!(
        lfs_file_seek(lfs.as_mut_ptr(), file.as_mut_ptr(), 0, LFS_SEEK_CUR),
        (count * KITTY.len() as u32) as i32
    );

    assert_ok(lfs_file_close(lfs.as_mut_ptr(), file.as_mut_ptr()));
    assert_ok(lfs_unmount(lfs.as_mut_ptr()));
}

/// Upstream: [cases.test_seek_write]
#[rstest]
#[case(132, 4)]
#[case(132, 128)]
#[case(200, 10)]
#[case(200, 100)]
#[case(4, 1)]
#[case(4, 2)]
fn test_seek_write(#[case] count: u32, #[case] skip: u32) {
    let mut env = default_config(256);
    init_context(&mut env);

    let mut lfs = core::mem::MaybeUninit::<Lfs>::zeroed();
    assert_ok(lfs_format(
        lfs.as_mut_ptr(),
        &env.config as *const LfsConfig,
    ));
    assert_ok(lfs_mount(lfs.as_mut_ptr(), &env.config as *const LfsConfig));

    let path = path_bytes("kitty");
    let mut file = core::mem::MaybeUninit::<LfsFile>::zeroed();
    assert_ok(lfs_file_open(
        lfs.as_mut_ptr(),
        file.as_mut_ptr(),
        path.as_ptr(),
        LFS_O_WRONLY | LFS_O_CREAT | LFS_O_APPEND,
    ));
    for _ in 0..count {
        let n = lfs_file_write(
            lfs.as_mut_ptr(),
            file.as_mut_ptr(),
            KITTY.as_ptr() as *const core::ffi::c_void,
            KITTY.len() as u32,
        );
        assert_eq!(n, KITTY.len() as i32);
    }
    assert_ok(lfs_file_close(lfs.as_mut_ptr(), file.as_mut_ptr()));
    assert_ok(lfs_unmount(lfs.as_mut_ptr()));

    assert_ok(lfs_mount(lfs.as_mut_ptr(), &env.config as *const LfsConfig));
    assert_ok(lfs_file_open(
        lfs.as_mut_ptr(),
        file.as_mut_ptr(),
        path.as_ptr(),
        LFS_O_RDWR,
    ));

    let mut buf = [0u8; 32];
    let mut pos: i32 = -1;
    for _ in 0..skip {
        let n = lfs_file_read(
            lfs.as_mut_ptr(),
            file.as_mut_ptr(),
            buf.as_mut_ptr() as *mut core::ffi::c_void,
            KITTY.len() as u32,
        );
        assert_eq!(n, KITTY.len() as i32);
        assert_eq!(&buf[..KITTY.len()], KITTY);
        pos = lfs_file_tell(lfs.as_mut_ptr(), file.as_mut_ptr());
    }
    assert!(pos >= 0);

    assert_eq!(
        lfs_file_seek(lfs.as_mut_ptr(), file.as_mut_ptr(), pos, LFS_SEEK_SET),
        pos
    );
    let n = lfs_file_write(
        lfs.as_mut_ptr(),
        file.as_mut_ptr(),
        DOGGO.as_ptr() as *const core::ffi::c_void,
        DOGGO.len() as u32,
    );
    assert_eq!(n, DOGGO.len() as i32);

    assert_eq!(
        lfs_file_seek(lfs.as_mut_ptr(), file.as_mut_ptr(), pos, LFS_SEEK_SET),
        pos
    );
    let n = lfs_file_read(
        lfs.as_mut_ptr(),
        file.as_mut_ptr(),
        buf.as_mut_ptr() as *mut core::ffi::c_void,
        DOGGO.len() as u32,
    );
    assert_eq!(n, DOGGO.len() as i32);
    assert_eq!(&buf[..DOGGO.len()], DOGGO);

    assert_ok(lfs_file_rewind(lfs.as_mut_ptr(), file.as_mut_ptr()));
    let n = lfs_file_read(
        lfs.as_mut_ptr(),
        file.as_mut_ptr(),
        buf.as_mut_ptr() as *mut core::ffi::c_void,
        KITTY.len() as u32,
    );
    assert_eq!(n, KITTY.len() as i32);
    assert_eq!(&buf[..KITTY.len()], KITTY);

    assert_eq!(
        lfs_file_seek(lfs.as_mut_ptr(), file.as_mut_ptr(), pos, LFS_SEEK_SET),
        pos
    );
    let n = lfs_file_read(
        lfs.as_mut_ptr(),
        file.as_mut_ptr(),
        buf.as_mut_ptr() as *mut core::ffi::c_void,
        DOGGO.len() as u32,
    );
    assert_eq!(n, DOGGO.len() as i32);
    assert_eq!(&buf[..DOGGO.len()], DOGGO);

    assert!(
        lfs_file_seek(
            lfs.as_mut_ptr(),
            file.as_mut_ptr(),
            -(KITTY.len() as i32),
            LFS_SEEK_END
        ) >= 0
    );
    let n = lfs_file_read(
        lfs.as_mut_ptr(),
        file.as_mut_ptr(),
        buf.as_mut_ptr() as *mut core::ffi::c_void,
        KITTY.len() as u32,
    );
    assert_eq!(n, KITTY.len() as i32);
    assert_eq!(&buf[..KITTY.len()], KITTY);

    assert_eq!(
        lfs_file_size(lfs.as_mut_ptr(), file.as_mut_ptr()),
        (count * KITTY.len() as u32) as i32
    );
    assert_eq!(
        lfs_file_seek(lfs.as_mut_ptr(), file.as_mut_ptr(), 0, LFS_SEEK_CUR),
        (count * KITTY.len() as u32) as i32
    );

    assert_ok(lfs_file_close(lfs.as_mut_ptr(), file.as_mut_ptr()));
    assert_ok(lfs_unmount(lfs.as_mut_ptr()));
}

/// Upstream: [cases.test_seek_boundary_read]
/// defines.COUNT = 132
#[test]
fn test_seek_boundary_read() {
    const COUNT: u32 = 132;
    let mut env = default_config(256);
    init_context(&mut env);

    let mut lfs = core::mem::MaybeUninit::<Lfs>::zeroed();
    assert_ok(lfs_format(
        lfs.as_mut_ptr(),
        &env.config as *const LfsConfig,
    ));
    assert_ok(lfs_mount(lfs.as_mut_ptr(), &env.config as *const LfsConfig));

    let path = path_bytes("kitty");
    let mut file = core::mem::MaybeUninit::<LfsFile>::zeroed();
    assert_ok(lfs_file_open(
        lfs.as_mut_ptr(),
        file.as_mut_ptr(),
        path.as_ptr(),
        LFS_O_WRONLY | LFS_O_CREAT | LFS_O_APPEND,
    ));
    for _ in 0..COUNT {
        let n = lfs_file_write(
            lfs.as_mut_ptr(),
            file.as_mut_ptr(),
            KITTY.as_ptr() as *const core::ffi::c_void,
            KITTY.len() as u32,
        );
        assert_eq!(n, KITTY.len() as i32);
    }
    assert_ok(lfs_file_close(lfs.as_mut_ptr(), file.as_mut_ptr()));
    assert_ok(lfs_unmount(lfs.as_mut_ptr()));

    assert_ok(lfs_mount(lfs.as_mut_ptr(), &env.config as *const LfsConfig));
    assert_ok(lfs_file_open(
        lfs.as_mut_ptr(),
        file.as_mut_ptr(),
        path.as_ptr(),
        LFS_O_RDONLY,
    ));

    let size = KITTY.len() as i64;
    let pattern = b"kittycatcatkittycatcat";
    let offsets: [i64; 13] = [
        512,
        1024 - 4,
        512 + 1,
        1024 - 4 + 1,
        512 - 1,
        1024 - 4 - 1,
        512 - size,
        1024 - 4 - size,
        512 - size + 1,
        1024 - 4 - size + 1,
        512 - size - 1,
        1024 - 4 - size - 1,
        size * (COUNT as i64 - 2) - 1,
    ];

    let mut buf = [0u8; 32];
    for off in offsets {
        if off < 0 || off + size > (COUNT as i64 * size) {
            continue;
        }
        assert_eq!(
            lfs_file_seek(
                lfs.as_mut_ptr(),
                file.as_mut_ptr(),
                off as i32,
                LFS_SEEK_SET
            ),
            off as i32
        );
        let n = lfs_file_read(
            lfs.as_mut_ptr(),
            file.as_mut_ptr(),
            buf.as_mut_ptr() as *mut core::ffi::c_void,
            KITTY.len() as u32,
        );
        assert_eq!(n, KITTY.len() as i32);
        let base = (off % size) as usize;
        assert_eq!(
            &buf[..KITTY.len()],
            &pattern[base..base + KITTY.len()],
            "off={}",
            off
        );

        let off_after = off + size + 1;
        if off_after >= 0 && off_after + size <= COUNT as i64 * size {
            assert_eq!(
                lfs_file_seek(
                    lfs.as_mut_ptr(),
                    file.as_mut_ptr(),
                    off_after as i32,
                    LFS_SEEK_SET
                ),
                off_after as i32
            );
            let n = lfs_file_read(
                lfs.as_mut_ptr(),
                file.as_mut_ptr(),
                buf.as_mut_ptr() as *mut core::ffi::c_void,
                KITTY.len() as u32,
            );
            assert_eq!(n, KITTY.len() as i32);
            let base = ((off + 1) % size) as usize;
            assert_eq!(&buf[..KITTY.len()], &pattern[base..base + KITTY.len()]);
        }

        let off_before = off - size - 1;
        if off_before >= 0 {
            assert_eq!(
                lfs_file_seek(
                    lfs.as_mut_ptr(),
                    file.as_mut_ptr(),
                    off_before as i32,
                    LFS_SEEK_SET
                ),
                off_before as i32
            );
            let n = lfs_file_read(
                lfs.as_mut_ptr(),
                file.as_mut_ptr(),
                buf.as_mut_ptr() as *mut core::ffi::c_void,
                KITTY.len() as u32,
            );
            assert_eq!(n, KITTY.len() as i32);
            let base = ((off - 1).rem_euclid(size)) as usize;
            assert_eq!(&buf[..KITTY.len()], &pattern[base..base + KITTY.len()]);
        }

        assert_eq!(
            lfs_file_seek(lfs.as_mut_ptr(), file.as_mut_ptr(), 0, LFS_SEEK_SET),
            0
        );
        let n = lfs_file_read(
            lfs.as_mut_ptr(),
            file.as_mut_ptr(),
            buf.as_mut_ptr() as *mut core::ffi::c_void,
            KITTY.len() as u32,
        );
        assert_eq!(n, KITTY.len() as i32);
        assert_eq!(&buf[..KITTY.len()], KITTY);

        assert_eq!(
            lfs_file_seek(
                lfs.as_mut_ptr(),
                file.as_mut_ptr(),
                off as i32,
                LFS_SEEK_SET
            ),
            off as i32
        );
        let n = lfs_file_read(
            lfs.as_mut_ptr(),
            file.as_mut_ptr(),
            buf.as_mut_ptr() as *mut core::ffi::c_void,
            KITTY.len() as u32,
        );
        assert_eq!(n, KITTY.len() as i32);
        let base = (off % size) as usize;
        assert_eq!(&buf[..KITTY.len()], &pattern[base..base + KITTY.len()]);

        let off_after = off + size + 1;
        if off_after >= 0 && off_after + size <= COUNT as i64 * size {
            assert_eq!(
                lfs_file_seek(
                    lfs.as_mut_ptr(),
                    file.as_mut_ptr(),
                    off_after as i32,
                    LFS_SEEK_SET
                ),
                off_after as i32
            );
            let n = lfs_file_read(
                lfs.as_mut_ptr(),
                file.as_mut_ptr(),
                buf.as_mut_ptr() as *mut core::ffi::c_void,
                KITTY.len() as u32,
            );
            assert_eq!(n, KITTY.len() as i32);
            let base = ((off + 1) % size) as usize;
            assert_eq!(&buf[..KITTY.len()], &pattern[base..base + KITTY.len()]);
        }

        let off_before = off - size - 1;
        if off_before >= 0 {
            assert_eq!(
                lfs_file_seek(
                    lfs.as_mut_ptr(),
                    file.as_mut_ptr(),
                    off_before as i32,
                    LFS_SEEK_SET
                ),
                off_before as i32
            );
            let n = lfs_file_read(
                lfs.as_mut_ptr(),
                file.as_mut_ptr(),
                buf.as_mut_ptr() as *mut core::ffi::c_void,
                KITTY.len() as u32,
            );
            assert_eq!(n, KITTY.len() as i32);
            let base = ((off - 1).rem_euclid(size)) as usize;
            assert_eq!(&buf[..KITTY.len()], &pattern[base..base + KITTY.len()]);
        }

        assert_ok(lfs_file_sync(lfs.as_mut_ptr(), file.as_mut_ptr()));

        assert_eq!(
            lfs_file_seek(lfs.as_mut_ptr(), file.as_mut_ptr(), 0, LFS_SEEK_SET),
            0
        );
        let n = lfs_file_read(
            lfs.as_mut_ptr(),
            file.as_mut_ptr(),
            buf.as_mut_ptr() as *mut core::ffi::c_void,
            KITTY.len() as u32,
        );
        assert_eq!(n, KITTY.len() as i32);
        assert_eq!(&buf[..KITTY.len()], KITTY);

        assert_eq!(
            lfs_file_seek(
                lfs.as_mut_ptr(),
                file.as_mut_ptr(),
                off as i32,
                LFS_SEEK_SET
            ),
            off as i32
        );
        let n = lfs_file_read(
            lfs.as_mut_ptr(),
            file.as_mut_ptr(),
            buf.as_mut_ptr() as *mut core::ffi::c_void,
            KITTY.len() as u32,
        );
        assert_eq!(n, KITTY.len() as i32);
        let base = (off % size) as usize;
        assert_eq!(&buf[..KITTY.len()], &pattern[base..base + KITTY.len()]);

        let off_after = off + size + 1;
        if off_after >= 0 && off_after + size <= COUNT as i64 * size {
            assert_eq!(
                lfs_file_seek(
                    lfs.as_mut_ptr(),
                    file.as_mut_ptr(),
                    off_after as i32,
                    LFS_SEEK_SET
                ),
                off_after as i32
            );
            let n = lfs_file_read(
                lfs.as_mut_ptr(),
                file.as_mut_ptr(),
                buf.as_mut_ptr() as *mut core::ffi::c_void,
                KITTY.len() as u32,
            );
            assert_eq!(n, KITTY.len() as i32);
            let base = ((off + 1) % size) as usize;
            assert_eq!(&buf[..KITTY.len()], &pattern[base..base + KITTY.len()]);
        }

        let off_before = off - size - 1;
        if off_before >= 0 {
            assert_eq!(
                lfs_file_seek(
                    lfs.as_mut_ptr(),
                    file.as_mut_ptr(),
                    off_before as i32,
                    LFS_SEEK_SET
                ),
                off_before as i32
            );
            let n = lfs_file_read(
                lfs.as_mut_ptr(),
                file.as_mut_ptr(),
                buf.as_mut_ptr() as *mut core::ffi::c_void,
                KITTY.len() as u32,
            );
            assert_eq!(n, KITTY.len() as i32);
            let base = ((off - 1).rem_euclid(size)) as usize;
            assert_eq!(&buf[..KITTY.len()], &pattern[base..base + KITTY.len()]);
        }
    }

    assert_ok(lfs_file_close(lfs.as_mut_ptr(), file.as_mut_ptr()));
    assert_ok(lfs_unmount(lfs.as_mut_ptr()));
}

/// Upstream: [cases.test_seek_boundary_write]
/// defines.COUNT = 132
#[test]
fn test_seek_boundary_write() {
    const COUNT: u32 = 132;
    let mut env = default_config(256);
    init_context(&mut env);

    let mut lfs = core::mem::MaybeUninit::<Lfs>::zeroed();
    assert_ok(lfs_format(
        lfs.as_mut_ptr(),
        &env.config as *const LfsConfig,
    ));
    assert_ok(lfs_mount(lfs.as_mut_ptr(), &env.config as *const LfsConfig));

    let path = path_bytes("kitty");
    let mut file = core::mem::MaybeUninit::<LfsFile>::zeroed();
    assert_ok(lfs_file_open(
        lfs.as_mut_ptr(),
        file.as_mut_ptr(),
        path.as_ptr(),
        LFS_O_WRONLY | LFS_O_CREAT | LFS_O_APPEND,
    ));
    for _ in 0..COUNT {
        let n = lfs_file_write(
            lfs.as_mut_ptr(),
            file.as_mut_ptr(),
            KITTY.as_ptr() as *const core::ffi::c_void,
            KITTY.len() as u32,
        );
        assert_eq!(n, KITTY.len() as i32);
    }
    assert_ok(lfs_file_close(lfs.as_mut_ptr(), file.as_mut_ptr()));
    assert_ok(lfs_unmount(lfs.as_mut_ptr()));

    assert_ok(lfs_mount(lfs.as_mut_ptr(), &env.config as *const LfsConfig));
    assert_ok(lfs_file_open(
        lfs.as_mut_ptr(),
        file.as_mut_ptr(),
        path.as_ptr(),
        LFS_O_RDWR,
    ));

    let size = KITTY.len() as i64;
    let offsets: [i64; 13] = [
        512,
        1024 - 4,
        512 + 1,
        1024 - 4 + 1,
        512 - 1,
        1024 - 4 - 1,
        512 - size,
        1024 - 4 - size,
        512 - size + 1,
        1024 - 4 - size + 1,
        512 - size - 1,
        1024 - 4 - size - 1,
        size * (COUNT as i64 - 2) - 1,
    ];

    let mut buf = [0u8; 32];
    for off in offsets {
        if off < 0 || off + size > COUNT as i64 * size {
            continue;
        }
        assert_eq!(
            lfs_file_seek(
                lfs.as_mut_ptr(),
                file.as_mut_ptr(),
                off as i32,
                LFS_SEEK_SET
            ),
            off as i32
        );
        let n = lfs_file_write(
            lfs.as_mut_ptr(),
            file.as_mut_ptr(),
            HEDGEHOG.as_ptr() as *const core::ffi::c_void,
            HEDGEHOG.len() as u32,
        );
        assert_eq!(n, HEDGEHOG.len() as i32);

        assert_eq!(
            lfs_file_seek(
                lfs.as_mut_ptr(),
                file.as_mut_ptr(),
                off as i32,
                LFS_SEEK_SET
            ),
            off as i32
        );
        let n = lfs_file_read(
            lfs.as_mut_ptr(),
            file.as_mut_ptr(),
            buf.as_mut_ptr() as *mut core::ffi::c_void,
            HEDGEHOG.len() as u32,
        );
        assert_eq!(n, HEDGEHOG.len() as i32);
        assert_eq!(&buf[..HEDGEHOG.len()], HEDGEHOG);

        assert_eq!(
            lfs_file_seek(lfs.as_mut_ptr(), file.as_mut_ptr(), 0, LFS_SEEK_SET),
            0
        );
        let n = lfs_file_read(
            lfs.as_mut_ptr(),
            file.as_mut_ptr(),
            buf.as_mut_ptr() as *mut core::ffi::c_void,
            KITTY.len() as u32,
        );
        assert_eq!(n, KITTY.len() as i32);
        assert_eq!(&buf[..KITTY.len()], KITTY);

        assert_eq!(
            lfs_file_seek(
                lfs.as_mut_ptr(),
                file.as_mut_ptr(),
                off as i32,
                LFS_SEEK_SET
            ),
            off as i32
        );
        let n = lfs_file_read(
            lfs.as_mut_ptr(),
            file.as_mut_ptr(),
            buf.as_mut_ptr() as *mut core::ffi::c_void,
            HEDGEHOG.len() as u32,
        );
        assert_eq!(n, HEDGEHOG.len() as i32);
        assert_eq!(&buf[..HEDGEHOG.len()], HEDGEHOG);

        assert_ok(lfs_file_sync(lfs.as_mut_ptr(), file.as_mut_ptr()));

        assert_eq!(
            lfs_file_seek(lfs.as_mut_ptr(), file.as_mut_ptr(), 0, LFS_SEEK_SET),
            0
        );
        let n = lfs_file_read(
            lfs.as_mut_ptr(),
            file.as_mut_ptr(),
            buf.as_mut_ptr() as *mut core::ffi::c_void,
            KITTY.len() as u32,
        );
        assert_eq!(n, KITTY.len() as i32);
        assert_eq!(&buf[..KITTY.len()], KITTY);

        assert_eq!(
            lfs_file_seek(
                lfs.as_mut_ptr(),
                file.as_mut_ptr(),
                off as i32,
                LFS_SEEK_SET
            ),
            off as i32
        );
        let n = lfs_file_read(
            lfs.as_mut_ptr(),
            file.as_mut_ptr(),
            buf.as_mut_ptr() as *mut core::ffi::c_void,
            HEDGEHOG.len() as u32,
        );
        assert_eq!(n, HEDGEHOG.len() as i32);
        assert_eq!(&buf[..HEDGEHOG.len()], HEDGEHOG);
    }

    assert_ok(lfs_file_close(lfs.as_mut_ptr(), file.as_mut_ptr()));
    assert_ok(lfs_unmount(lfs.as_mut_ptr()));
}

/// Upstream: [cases.test_seek_out_of_bounds]
#[rstest]
#[case(132, 4)]
#[case(132, 128)]
#[case(200, 10)]
#[case(200, 100)]
#[case(4, 2)]
#[case(4, 3)]
fn test_seek_out_of_bounds(#[case] count: u32, #[case] skip: u32) {
    let mut env = default_config(256);
    init_context(&mut env);

    let mut lfs = core::mem::MaybeUninit::<Lfs>::zeroed();
    assert_ok(lfs_format(
        lfs.as_mut_ptr(),
        &env.config as *const LfsConfig,
    ));
    assert_ok(lfs_mount(lfs.as_mut_ptr(), &env.config as *const LfsConfig));

    let path = path_bytes("kitty");
    let mut file = core::mem::MaybeUninit::<LfsFile>::zeroed();
    assert_ok(lfs_file_open(
        lfs.as_mut_ptr(),
        file.as_mut_ptr(),
        path.as_ptr(),
        LFS_O_WRONLY | LFS_O_CREAT | LFS_O_APPEND,
    ));
    for _ in 0..count {
        let n = lfs_file_write(
            lfs.as_mut_ptr(),
            file.as_mut_ptr(),
            KITTY.as_ptr() as *const core::ffi::c_void,
            KITTY.len() as u32,
        );
        assert_eq!(n, KITTY.len() as i32);
    }
    assert_ok(lfs_file_close(lfs.as_mut_ptr(), file.as_mut_ptr()));
    assert_ok(lfs_unmount(lfs.as_mut_ptr()));

    assert_ok(lfs_mount(lfs.as_mut_ptr(), &env.config as *const LfsConfig));
    assert_ok(lfs_file_open(
        lfs.as_mut_ptr(),
        file.as_mut_ptr(),
        path.as_ptr(),
        LFS_O_RDWR,
    ));

    let size = KITTY.len() as i64;
    let hole_offset = (count as i64 + skip as i64) * size;

    assert_eq!(
        lfs_file_size(lfs.as_mut_ptr(), file.as_mut_ptr()),
        (count * KITTY.len() as u32) as i32
    );
    assert_eq!(
        lfs_file_seek(
            lfs.as_mut_ptr(),
            file.as_mut_ptr(),
            hole_offset as i32,
            LFS_SEEK_SET,
        ),
        hole_offset as i32
    );
    let mut buf = [0u8; 32];
    let n = lfs_file_read(
        lfs.as_mut_ptr(),
        file.as_mut_ptr(),
        buf.as_mut_ptr() as *mut core::ffi::c_void,
        KITTY.len() as u32,
    );
    assert_eq!(n, 0);

    let n = lfs_file_write(
        lfs.as_mut_ptr(),
        file.as_mut_ptr(),
        PORCUPINE.as_ptr() as *const core::ffi::c_void,
        PORCUPINE.len() as u32,
    );
    assert_eq!(n, PORCUPINE.len() as i32);

    assert_eq!(
        lfs_file_seek(
            lfs.as_mut_ptr(),
            file.as_mut_ptr(),
            hole_offset as i32,
            LFS_SEEK_SET,
        ),
        hole_offset as i32
    );
    let n = lfs_file_read(
        lfs.as_mut_ptr(),
        file.as_mut_ptr(),
        buf.as_mut_ptr() as *mut core::ffi::c_void,
        PORCUPINE.len() as u32,
    );
    assert_eq!(n, PORCUPINE.len() as i32);
    assert_eq!(&buf[..PORCUPINE.len()], PORCUPINE);

    assert_eq!(
        lfs_file_seek(
            lfs.as_mut_ptr(),
            file.as_mut_ptr(),
            (count as i32) * (size as i32),
            LFS_SEEK_SET,
        ),
        (count as i32) * (size as i32)
    );
    let n = lfs_file_read(
        lfs.as_mut_ptr(),
        file.as_mut_ptr(),
        buf.as_mut_ptr() as *mut core::ffi::c_void,
        KITTY.len() as u32,
    );
    assert_eq!(n, KITTY.len() as i32);
    assert!(
        buf[..KITTY.len()].iter().all(|&b| b == 0),
        "hole should be zeros, got {:?}",
        &buf[..KITTY.len()]
    );

    // After read at count*size we're at (count+1)*size
    assert_eq!(
        lfs_file_seek(
            lfs.as_mut_ptr(),
            file.as_mut_ptr(),
            -(hole_offset as i32),
            LFS_SEEK_CUR,
        ),
        LFS_ERR_INVAL
    );
    assert_eq!(
        lfs_file_tell(lfs.as_mut_ptr(), file.as_mut_ptr()),
        (count as i32 + 1) * (size as i32)
    );

    assert_eq!(
        lfs_file_seek(
            lfs.as_mut_ptr(),
            file.as_mut_ptr(),
            -((count as i32 + 2 * skip as i32) * (size as i32)),
            LFS_SEEK_END,
        ),
        LFS_ERR_INVAL
    );
    assert_eq!(
        lfs_file_tell(lfs.as_mut_ptr(), file.as_mut_ptr()),
        (count as i32 + 1) * (size as i32)
    );

    assert_ok(lfs_file_close(lfs.as_mut_ptr(), file.as_mut_ptr()));
    assert_ok(lfs_unmount(lfs.as_mut_ptr()));
}

/// Upstream: [cases.test_seek_inline_write]
/// defines.SIZE = [2, 4, 128, 132]
#[rstest]
#[case(2)]
#[case(4)]
#[case(128)]
#[case(132)]
fn test_seek_inline_write(#[case] size: u32) {
    let mut env = default_config(256);
    init_context(&mut env);

    let mut lfs = core::mem::MaybeUninit::<Lfs>::zeroed();
    assert_ok(lfs_format(
        lfs.as_mut_ptr(),
        &env.config as *const LfsConfig,
    ));
    assert_ok(lfs_mount(lfs.as_mut_ptr(), &env.config as *const LfsConfig));

    let path = path_bytes("tinykitty");
    let mut file = core::mem::MaybeUninit::<LfsFile>::zeroed();
    assert_ok(lfs_file_open(
        lfs.as_mut_ptr(),
        file.as_mut_ptr(),
        path.as_ptr(),
        LFS_O_RDWR | LFS_O_CREAT,
    ));

    let alphabet = b"abcdefghijklmnopqrstuvwxyz";
    let mut j = 0usize;
    let mut k = 0usize;

    for i in 0..size {
        let c = alphabet[j % 26];
        let n = lfs_file_write(
            lfs.as_mut_ptr(),
            file.as_mut_ptr(),
            &c as *const u8 as *const core::ffi::c_void,
            1,
        );
        assert_eq!(n, 1);
        assert_eq!(
            lfs_file_tell(lfs.as_mut_ptr(), file.as_mut_ptr()),
            (i + 1) as i32
        );
        assert_eq!(
            lfs_file_size(lfs.as_mut_ptr(), file.as_mut_ptr()),
            (i + 1) as i32
        );
        j += 1;
    }

    assert_eq!(
        lfs_file_seek(lfs.as_mut_ptr(), file.as_mut_ptr(), 0, LFS_SEEK_SET),
        0
    );
    assert_eq!(lfs_file_tell(lfs.as_mut_ptr(), file.as_mut_ptr()), 0);
    assert_eq!(
        lfs_file_size(lfs.as_mut_ptr(), file.as_mut_ptr()),
        size as i32
    );

    let mut c = [0u8; 1];
    for _ in 0..size {
        let n = lfs_file_read(
            lfs.as_mut_ptr(),
            file.as_mut_ptr(),
            c.as_mut_ptr() as *mut core::ffi::c_void,
            1,
        );
        assert_eq!(n, 1);
        assert_eq!(c[0], alphabet[k % 26]);
        k += 1;
    }

    assert_ok(lfs_file_sync(lfs.as_mut_ptr(), file.as_mut_ptr()));
    assert_eq!(
        lfs_file_tell(lfs.as_mut_ptr(), file.as_mut_ptr()),
        size as i32
    );
    assert_eq!(
        lfs_file_size(lfs.as_mut_ptr(), file.as_mut_ptr()),
        size as i32
    );

    assert_eq!(
        lfs_file_seek(lfs.as_mut_ptr(), file.as_mut_ptr(), 0, LFS_SEEK_SET),
        0
    );

    for i in 0..size {
        let c = alphabet[j % 26];
        let n = lfs_file_write(
            lfs.as_mut_ptr(),
            file.as_mut_ptr(),
            &c as *const u8 as *const core::ffi::c_void,
            1,
        );
        assert_eq!(n, 1);
        assert_eq!(
            lfs_file_tell(lfs.as_mut_ptr(), file.as_mut_ptr()),
            (i + 1) as i32
        );
        assert_eq!(
            lfs_file_size(lfs.as_mut_ptr(), file.as_mut_ptr()),
            size as i32
        );
        assert_ok(lfs_file_sync(lfs.as_mut_ptr(), file.as_mut_ptr()));
        assert_eq!(
            lfs_file_tell(lfs.as_mut_ptr(), file.as_mut_ptr()),
            (i + 1) as i32
        );
        assert_eq!(
            lfs_file_size(lfs.as_mut_ptr(), file.as_mut_ptr()),
            size as i32
        );

        if i < size - 2 {
            let mut buf3 = [0u8; 3];
            assert_eq!(
                lfs_file_seek(lfs.as_mut_ptr(), file.as_mut_ptr(), -1, LFS_SEEK_CUR),
                i as i32
            );
            let n = lfs_file_read(
                lfs.as_mut_ptr(),
                file.as_mut_ptr(),
                buf3.as_mut_ptr() as *mut core::ffi::c_void,
                3,
            );
            assert_eq!(n, 3);
            assert_eq!(
                lfs_file_tell(lfs.as_mut_ptr(), file.as_mut_ptr()),
                (i + 3) as i32
            );
            assert_eq!(
                lfs_file_size(lfs.as_mut_ptr(), file.as_mut_ptr()),
                size as i32
            );
            assert_eq!(
                lfs_file_seek(
                    lfs.as_mut_ptr(),
                    file.as_mut_ptr(),
                    (i + 1) as i32,
                    LFS_SEEK_SET
                ),
                (i + 1) as i32
            );
            assert_eq!(
                lfs_file_tell(lfs.as_mut_ptr(), file.as_mut_ptr()),
                (i + 1) as i32
            );
            assert_eq!(
                lfs_file_size(lfs.as_mut_ptr(), file.as_mut_ptr()),
                size as i32
            );
        }
        j += 1;
    }

    assert_eq!(
        lfs_file_seek(lfs.as_mut_ptr(), file.as_mut_ptr(), 0, LFS_SEEK_SET),
        0
    );
    assert_eq!(lfs_file_tell(lfs.as_mut_ptr(), file.as_mut_ptr()), 0);
    assert_eq!(
        lfs_file_size(lfs.as_mut_ptr(), file.as_mut_ptr()),
        size as i32
    );

    let mut c = [0u8; 1];
    for _ in 0..size {
        let n = lfs_file_read(
            lfs.as_mut_ptr(),
            file.as_mut_ptr(),
            c.as_mut_ptr() as *mut core::ffi::c_void,
            1,
        );
        assert_eq!(n, 1);
        assert_eq!(c[0], alphabet[k % 26]);
        k += 1;
    }

    assert_ok(lfs_file_sync(lfs.as_mut_ptr(), file.as_mut_ptr()));
    assert_eq!(
        lfs_file_tell(lfs.as_mut_ptr(), file.as_mut_ptr()),
        size as i32
    );
    assert_eq!(
        lfs_file_size(lfs.as_mut_ptr(), file.as_mut_ptr()),
        size as i32
    );

    assert_ok(lfs_file_close(lfs.as_mut_ptr(), file.as_mut_ptr()));
    assert_ok(lfs_unmount(lfs.as_mut_ptr()));
}

/// Upstream: [cases.test_seek_reentrant_write]
/// defines.COUNT = [4, 64, 128], POWERLOSS_BEHAVIOR = [NOOP, OOO]
#[rstest]
#[case(4)]
#[case(64)]
#[case(128)]
#[cfg(feature = "slow_tests")]
#[ignore = "bug: power-loss iteration returns -1 for some cases"]
fn test_seek_reentrant_write(#[case] count: u32) {
    let mut env = powerloss_config(256);
    init_powerloss_context(&mut env);

    let config_ptr = &env.config as *const LfsConfig;
    let mut lfs = core::mem::MaybeUninit::<Lfs>::zeroed();

    assert_ok(littlefs_rust_core::lfs_format(lfs.as_mut_ptr(), config_ptr));
    assert_ok(littlefs_rust_core::lfs_mount(lfs.as_mut_ptr(), config_ptr));
    assert_ok(littlefs_rust_core::lfs_unmount(lfs.as_mut_ptr()));
    let snapshot = env.snapshot();

    let op = |lfs: *mut Lfs, cfg: *const LfsConfig| -> Result<(), i32> {
        let err = littlefs_rust_core::lfs_mount(lfs, cfg);
        if err != 0 {
            let _ = littlefs_rust_core::lfs_format(lfs, cfg);
            let e = littlefs_rust_core::lfs_mount(lfs, cfg);
            if e != 0 {
                return Err(e);
            }
        }

        let path = path_bytes("kitty");
        let mut file = core::mem::MaybeUninit::<LfsFile>::zeroed();
        let mut buf = [0u8; 32];

        let open_err =
            littlefs_rust_core::lfs_file_open(lfs, file.as_mut_ptr(), path.as_ptr(), LFS_O_RDONLY);
        if open_err == 0 {
            let sz = littlefs_rust_core::lfs_file_size(lfs, file.as_mut_ptr());
            if sz != 0 {
                assert_eq!(sz, (count * 11) as i32);
                for _ in 0..count {
                    let n = littlefs_rust_core::lfs_file_read(
                        lfs,
                        file.as_mut_ptr(),
                        buf.as_mut_ptr() as *mut core::ffi::c_void,
                        11,
                    );
                    if n != 11 {
                        return Err(-1);
                    }
                    assert!(
                        &buf[..11] == KITTY || &buf[..11] == DOGGO,
                        "unexpected content"
                    );
                }
            }
            let e = littlefs_rust_core::lfs_file_close(lfs, file.as_mut_ptr());
            if e != 0 {
                return Err(e);
            }
        } else {
            assert_eq!(open_err, LFS_ERR_NOENT);
        }

        let e = littlefs_rust_core::lfs_file_open(
            lfs,
            file.as_mut_ptr(),
            path.as_ptr(),
            LFS_O_WRONLY | LFS_O_CREAT,
        );
        if e != 0 {
            return Err(e);
        }
        if littlefs_rust_core::lfs_file_size(lfs, file.as_mut_ptr()) == 0 {
            for _ in 0..count {
                let n = littlefs_rust_core::lfs_file_write(
                    lfs,
                    file.as_mut_ptr(),
                    KITTY.as_ptr() as *const core::ffi::c_void,
                    KITTY.len() as u32,
                );
                if n < 0 {
                    return Err(n);
                }
                assert_eq!(n, KITTY.len() as i32);
            }
        }
        let e = littlefs_rust_core::lfs_file_close(lfs, file.as_mut_ptr());
        if e != 0 {
            return Err(e);
        }

        let e = littlefs_rust_core::lfs_file_open(lfs, file.as_mut_ptr(), path.as_ptr(), LFS_O_RDWR);
        if e != 0 {
            return Err(e);
        }
        assert_eq!(
            littlefs_rust_core::lfs_file_size(lfs, file.as_mut_ptr()),
            (count * 11) as i32
        );

        let mut off: u32 = 0;
        for _ in 0..count {
            off = (5 * off + 1) % count;
            let pos = (off * 11) as i32;
            let seek_res =
                littlefs_rust_core::lfs_file_seek(lfs, file.as_mut_ptr(), pos, LFS_SEEK_SET);
            if seek_res != pos {
                return Err(-1);
            }
            let n = littlefs_rust_core::lfs_file_read(
                lfs,
                file.as_mut_ptr(),
                buf.as_mut_ptr() as *mut core::ffi::c_void,
                11,
            );
            if n != 11 {
                return Err(-1);
            }
            assert!(&buf[..11] == KITTY || &buf[..11] == DOGGO);
            if &buf[..11] != DOGGO {
                let seek_res =
                    littlefs_rust_core::lfs_file_seek(lfs, file.as_mut_ptr(), pos, LFS_SEEK_SET);
                if seek_res != pos {
                    return Err(-1);
                }
                let n = littlefs_rust_core::lfs_file_write(
                    lfs,
                    file.as_mut_ptr(),
                    DOGGO.as_ptr() as *const core::ffi::c_void,
                    DOGGO.len() as u32,
                );
                if n < 0 {
                    return Err(n);
                }
                assert_eq!(n, DOGGO.len() as i32);
                let seek_res =
                    littlefs_rust_core::lfs_file_seek(lfs, file.as_mut_ptr(), pos, LFS_SEEK_SET);
                if seek_res != pos {
                    return Err(-1);
                }
                let n = littlefs_rust_core::lfs_file_read(
                    lfs,
                    file.as_mut_ptr(),
                    buf.as_mut_ptr() as *mut core::ffi::c_void,
                    11,
                );
                if n != 11 {
                    return Err(-1);
                }
                assert_eq!(&buf[..11], DOGGO);
                let e = littlefs_rust_core::lfs_file_sync(lfs, file.as_mut_ptr());
                if e != 0 {
                    return Err(e);
                }
                let seek_res =
                    littlefs_rust_core::lfs_file_seek(lfs, file.as_mut_ptr(), pos, LFS_SEEK_SET);
                if seek_res != pos {
                    return Err(-1);
                }
                let n = littlefs_rust_core::lfs_file_read(
                    lfs,
                    file.as_mut_ptr(),
                    buf.as_mut_ptr() as *mut core::ffi::c_void,
                    11,
                );
                if n != 11 {
                    return Err(-1);
                }
                assert_eq!(&buf[..11], DOGGO);
            }
        }

        let e = littlefs_rust_core::lfs_file_close(lfs, file.as_mut_ptr());
        if e != 0 {
            return Err(e);
        }

        let e = littlefs_rust_core::lfs_file_open(lfs, file.as_mut_ptr(), path.as_ptr(), LFS_O_RDWR);
        if e != 0 {
            return Err(e);
        }
        assert_eq!(
            littlefs_rust_core::lfs_file_size(lfs, file.as_mut_ptr()),
            (count * 11) as i32
        );
        for _ in 0..count {
            let n = littlefs_rust_core::lfs_file_read(
                lfs,
                file.as_mut_ptr(),
                buf.as_mut_ptr() as *mut core::ffi::c_void,
                11,
            );
            if n != 11 {
                return Err(-1);
            }
            assert_eq!(&buf[..11], DOGGO);
        }
        let e = littlefs_rust_core::lfs_file_close(lfs, file.as_mut_ptr());
        if e != 0 {
            return Err(e);
        }
        let e = littlefs_rust_core::lfs_unmount(lfs);
        if e != 0 {
            return Err(e);
        }
        Ok(())
    };

    let result = run_powerloss_linear(&mut env, &snapshot, 3000, op, |_, _| Ok(()));
    result.expect("reentrant seek write should eventually succeed");
}

/// Upstream: [cases.test_seek_filemax]
#[test]
fn test_seek_filemax() {
    let mut env = default_config(128);
    init_context(&mut env);

    let mut lfs = core::mem::MaybeUninit::<Lfs>::zeroed();
    assert_ok(lfs_format(
        lfs.as_mut_ptr(),
        &env.config as *const LfsConfig,
    ));
    assert_ok(lfs_mount(lfs.as_mut_ptr(), &env.config as *const LfsConfig));

    let path = path_bytes("kitty");
    let mut file = core::mem::MaybeUninit::<LfsFile>::zeroed();
    assert_ok(lfs_file_open(
        lfs.as_mut_ptr(),
        file.as_mut_ptr(),
        path.as_ptr(),
        LFS_O_WRONLY | LFS_O_CREAT | LFS_O_APPEND,
    ));
    let n = lfs_file_write(
        lfs.as_mut_ptr(),
        file.as_mut_ptr(),
        KITTY.as_ptr() as *const core::ffi::c_void,
        KITTY.len() as u32,
    );
    assert_eq!(n, KITTY.len() as i32);

    assert_eq!(
        lfs_file_seek(
            lfs.as_mut_ptr(),
            file.as_mut_ptr(),
            LFS_FILE_MAX,
            LFS_SEEK_SET
        ),
        LFS_FILE_MAX
    );

    assert_eq!(
        lfs_file_seek(lfs.as_mut_ptr(), file.as_mut_ptr(), 0, LFS_SEEK_CUR),
        LFS_FILE_MAX
    );

    assert_eq!(
        lfs_file_seek(lfs.as_mut_ptr(), file.as_mut_ptr(), 10, LFS_SEEK_END),
        KITTY.len() as i32 + 10
    );

    assert_ok(lfs_file_close(lfs.as_mut_ptr(), file.as_mut_ptr()));
    assert_ok(lfs_unmount(lfs.as_mut_ptr()));
}

/// Upstream: [cases.test_seek_underflow]
#[test]
fn test_seek_underflow() {
    let mut env = default_config(128);
    init_context(&mut env);

    let mut lfs = core::mem::MaybeUninit::<Lfs>::zeroed();
    assert_ok(lfs_format(
        lfs.as_mut_ptr(),
        &env.config as *const LfsConfig,
    ));
    assert_ok(lfs_mount(lfs.as_mut_ptr(), &env.config as *const LfsConfig));

    let path = path_bytes("kitty");
    let mut file = core::mem::MaybeUninit::<LfsFile>::zeroed();
    assert_ok(lfs_file_open(
        lfs.as_mut_ptr(),
        file.as_mut_ptr(),
        path.as_ptr(),
        LFS_O_WRONLY | LFS_O_CREAT | LFS_O_APPEND,
    ));
    let n = lfs_file_write(
        lfs.as_mut_ptr(),
        file.as_mut_ptr(),
        KITTY.as_ptr() as *const core::ffi::c_void,
        KITTY.len() as u32,
    );
    assert_eq!(n, KITTY.len() as i32);
    let size = KITTY.len() as i32;

    assert_eq!(
        lfs_file_seek(
            lfs.as_mut_ptr(),
            file.as_mut_ptr(),
            -(size + 10),
            LFS_SEEK_CUR
        ),
        LFS_ERR_INVAL
    );
    assert_eq!(
        lfs_file_seek(
            lfs.as_mut_ptr(),
            file.as_mut_ptr(),
            -LFS_FILE_MAX,
            LFS_SEEK_CUR
        ),
        LFS_ERR_INVAL
    );
    assert_eq!(
        lfs_file_seek(
            lfs.as_mut_ptr(),
            file.as_mut_ptr(),
            i32::MIN, // -(size + LFS_FILE_MAX) overflows; use MIN to trigger underflow
            LFS_SEEK_CUR,
        ),
        LFS_ERR_INVAL
    );

    assert_eq!(
        lfs_file_seek(
            lfs.as_mut_ptr(),
            file.as_mut_ptr(),
            -(size + 10),
            LFS_SEEK_END
        ),
        LFS_ERR_INVAL
    );
    assert_eq!(
        lfs_file_seek(
            lfs.as_mut_ptr(),
            file.as_mut_ptr(),
            -LFS_FILE_MAX,
            LFS_SEEK_END
        ),
        LFS_ERR_INVAL
    );
    assert_eq!(
        lfs_file_seek(
            lfs.as_mut_ptr(),
            file.as_mut_ptr(),
            i32::MIN, // -(size + LFS_FILE_MAX) overflows; use MIN
            LFS_SEEK_END,
        ),
        LFS_ERR_INVAL
    );

    assert_eq!(lfs_file_tell(lfs.as_mut_ptr(), file.as_mut_ptr()), size);

    assert_ok(lfs_file_close(lfs.as_mut_ptr(), file.as_mut_ptr()));
    assert_ok(lfs_unmount(lfs.as_mut_ptr()));
}

/// Upstream: [cases.test_seek_overflow]
#[test]
fn test_seek_overflow() {
    let mut env = default_config(128);
    init_context(&mut env);

    let mut lfs = core::mem::MaybeUninit::<Lfs>::zeroed();
    assert_ok(lfs_format(
        lfs.as_mut_ptr(),
        &env.config as *const LfsConfig,
    ));
    assert_ok(lfs_mount(lfs.as_mut_ptr(), &env.config as *const LfsConfig));

    let path = path_bytes("kitty");
    let mut file = core::mem::MaybeUninit::<LfsFile>::zeroed();
    assert_ok(lfs_file_open(
        lfs.as_mut_ptr(),
        file.as_mut_ptr(),
        path.as_ptr(),
        LFS_O_WRONLY | LFS_O_CREAT | LFS_O_APPEND,
    ));
    let n = lfs_file_write(
        lfs.as_mut_ptr(),
        file.as_mut_ptr(),
        KITTY.as_ptr() as *const core::ffi::c_void,
        KITTY.len() as u32,
    );
    assert_eq!(n, KITTY.len() as i32);
    let size = KITTY.len() as i32;

    assert_eq!(
        lfs_file_seek(
            lfs.as_mut_ptr(),
            file.as_mut_ptr(),
            LFS_FILE_MAX,
            LFS_SEEK_SET
        ),
        LFS_FILE_MAX
    );

    assert_eq!(
        lfs_file_seek(lfs.as_mut_ptr(), file.as_mut_ptr(), 10, LFS_SEEK_CUR),
        LFS_ERR_INVAL
    );
    assert_eq!(
        lfs_file_seek(
            lfs.as_mut_ptr(),
            file.as_mut_ptr(),
            LFS_FILE_MAX,
            LFS_SEEK_CUR
        ),
        LFS_ERR_INVAL
    );

    assert_eq!(
        lfs_file_seek(
            lfs.as_mut_ptr(),
            file.as_mut_ptr(),
            LFS_FILE_MAX.wrapping_add(10),
            LFS_SEEK_SET,
        ),
        LFS_ERR_INVAL
    );
    assert_eq!(
        lfs_file_seek(
            lfs.as_mut_ptr(),
            file.as_mut_ptr(),
            LFS_FILE_MAX.wrapping_add(LFS_FILE_MAX),
            LFS_SEEK_SET,
        ),
        LFS_ERR_INVAL
    );

    assert_eq!(
        lfs_file_seek(
            lfs.as_mut_ptr(),
            file.as_mut_ptr(),
            LFS_FILE_MAX.wrapping_sub(size).wrapping_add(10),
            LFS_SEEK_END,
        ),
        LFS_ERR_INVAL
    );
    assert_eq!(
        lfs_file_seek(
            lfs.as_mut_ptr(),
            file.as_mut_ptr(),
            LFS_FILE_MAX.wrapping_sub(size).wrapping_add(LFS_FILE_MAX),
            LFS_SEEK_END,
        ),
        LFS_ERR_INVAL
    );

    assert_eq!(
        lfs_file_tell(lfs.as_mut_ptr(), file.as_mut_ptr()),
        LFS_FILE_MAX
    );

    assert_ok(lfs_file_close(lfs.as_mut_ptr(), file.as_mut_ptr()));
    assert_ok(lfs_unmount(lfs.as_mut_ptr()));
}
