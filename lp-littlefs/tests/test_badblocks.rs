//! Upstream: tests/test_badblocks.toml
//!
//! Bad-block handling: single, region, alternating corruption, and superblock corruption.
//! All cases require block_cycles == -1 (no FS-level wear leveling).

#![allow(clippy::needless_range_loop)]

mod common;

use common::{
    assert_ok, config_with_wear_leveling_full, init_wear_leveling_context, BadBlockBehavior,
    LFS_O_CREAT, LFS_O_RDONLY, LFS_O_WRONLY,
};
use lp_littlefs::{
    lfs_file_close, lfs_file_open, lfs_file_read, lfs_file_write, lfs_format, lfs_mkdir, lfs_mount,
    lfs_stat, lfs_unmount, Lfs, LfsConfig, LfsFile, LfsInfo, LFS_ERR_CORRUPT, LFS_ERR_NOSPC,
};
use rstest::rstest;

const LFS_TYPE_DIR: u8 = 0x02;
const NAMEMULT: usize = 64;
const FILEMULT: usize = 1;

/// Upstream: [cases.test_badblocks_single]
/// defines.ERASE_COUNT = 256
/// defines.ERASE_CYCLES = 0xffffffff
/// defines.ERASE_VALUE = [0x00, 0xff, -1]
/// defines.BADBLOCK_BEHAVIOR = [PROGERROR, ERASEERROR, READERROR, PROGNOOP, ERASENOOP]
/// defines.NAMEMULT = 64
/// defines.FILEMULT = 1
///
/// For each block b in 2..BLOCK_COUNT: mark block b as worn (0xffffffff) and
/// block b-1 as fresh (0). Format, mount, create 9 dirs with files, unmount,
/// remount, stat/read all dirs and files.
#[rstest]
fn test_badblocks_single(
    #[values(0x00, 0xff, -1)] erase_value: i32,
    #[values(
        BadBlockBehavior::ProgError,
        BadBlockBehavior::EraseError,
        BadBlockBehavior::ReadError,
        BadBlockBehavior::ProgNoop,
        BadBlockBehavior::EraseNoop
    )]
    behavior: BadBlockBehavior,
) {
    let block_count: u32 = 256;

    for badblock in 2..block_count {
        let mut env =
            config_with_wear_leveling_full(block_count, 0xffffffff, behavior, erase_value);
        init_wear_leveling_context(&mut env);

        // C: lfs_emubd_setwear(cfg, badblock-1, 0)
        env.bd.set_wear(badblock - 1, 0);
        // C: lfs_emubd_setwear(cfg, badblock, 0xffffffff)
        env.bd.set_wear(badblock, 0xffffffff);

        let mut lfs = core::mem::MaybeUninit::<Lfs>::zeroed();
        assert_ok(lfs_format(
            lfs.as_mut_ptr(),
            &env.config as *const LfsConfig,
        ));

        assert_ok(lfs_mount(lfs.as_mut_ptr(), &env.config as *const LfsConfig));

        for i in 1..10 {
            let mut buffer = [0u8; 1024];
            for j in 0..NAMEMULT {
                buffer[j] = b'0' + i as u8;
            }
            buffer[NAMEMULT] = 0;

            // mkdir
            assert_ok(lfs_mkdir(lfs.as_mut_ptr(), buffer.as_ptr()));

            // Build file path: "dirname/dirname"
            buffer[NAMEMULT] = b'/';
            for j in 0..NAMEMULT {
                buffer[j + NAMEMULT + 1] = b'0' + i as u8;
            }
            buffer[2 * NAMEMULT + 1] = 0;

            let mut file = core::mem::MaybeUninit::<LfsFile>::zeroed();
            assert_ok(lfs_file_open(
                lfs.as_mut_ptr(),
                file.as_mut_ptr(),
                buffer.as_ptr(),
                LFS_O_WRONLY | LFS_O_CREAT,
            ));

            let size = NAMEMULT as u32;
            for _j in 0..(i * FILEMULT) {
                let n = lfs_file_write(
                    lfs.as_mut_ptr(),
                    file.as_mut_ptr(),
                    buffer.as_ptr() as *const core::ffi::c_void,
                    size,
                );
                assert_eq!(n, size as i32);
            }

            assert_ok(lfs_file_close(lfs.as_mut_ptr(), file.as_mut_ptr()));
        }
        assert_ok(lfs_unmount(lfs.as_mut_ptr()));

        // Remount and verify
        assert_ok(lfs_mount(lfs.as_mut_ptr(), &env.config as *const LfsConfig));

        for i in 1..10 {
            let mut buffer = [0u8; 1024];
            for j in 0..NAMEMULT {
                buffer[j] = b'0' + i as u8;
            }
            buffer[NAMEMULT] = 0;

            let mut info = core::mem::MaybeUninit::<LfsInfo>::zeroed();
            assert_ok(lfs_stat(
                lfs.as_mut_ptr(),
                buffer.as_ptr(),
                info.as_mut_ptr(),
            ));
            let info_ref = unsafe { &*info.as_ptr() };
            assert_eq!(info_ref.type_, LFS_TYPE_DIR);

            buffer[NAMEMULT] = b'/';
            for j in 0..NAMEMULT {
                buffer[j + NAMEMULT + 1] = b'0' + i as u8;
            }
            buffer[2 * NAMEMULT + 1] = 0;

            let mut file = core::mem::MaybeUninit::<LfsFile>::zeroed();
            assert_ok(lfs_file_open(
                lfs.as_mut_ptr(),
                file.as_mut_ptr(),
                buffer.as_ptr(),
                LFS_O_RDONLY,
            ));

            let size = NAMEMULT as u32;
            for _j in 0..(i * FILEMULT) {
                let mut rbuffer = [0u8; 1024];
                let n = lfs_file_read(
                    lfs.as_mut_ptr(),
                    file.as_mut_ptr(),
                    rbuffer.as_mut_ptr() as *mut core::ffi::c_void,
                    size,
                );
                assert_eq!(n, size as i32);
                assert_eq!(&rbuffer[..size as usize], &buffer[..size as usize]);
            }

            assert_ok(lfs_file_close(lfs.as_mut_ptr(), file.as_mut_ptr()));
        }
        assert_ok(lfs_unmount(lfs.as_mut_ptr()));
    }
}

/// Upstream: [cases.test_badblocks_region_corruption]
/// defines.ERASE_COUNT = 256
/// defines.ERASE_CYCLES = 0xffffffff
/// defines.ERASE_VALUE = [0x00, 0xff, -1]
/// defines.BADBLOCK_BEHAVIOR = [PROGERROR, ERASEERROR, READERROR, PROGNOOP, ERASENOOP]
/// defines.NAMEMULT = 64
/// defines.FILEMULT = 1
///
/// Mark first half of blocks (starting at 2) as worn. Format, create
/// 9 dirs+files, unmount, remount, verify.
#[rstest]
fn test_badblocks_region_corruption(
    #[values(0x00, 0xff, -1)] erase_value: i32,
    #[values(
        BadBlockBehavior::ProgError,
        BadBlockBehavior::EraseError,
        BadBlockBehavior::ReadError,
        BadBlockBehavior::ProgNoop,
        BadBlockBehavior::EraseNoop
    )]
    behavior: BadBlockBehavior,
) {
    let block_count: u32 = 256;

    let mut env = config_with_wear_leveling_full(block_count, 0xffffffff, behavior, erase_value);
    init_wear_leveling_context(&mut env);

    // C: for (lfs_block_t i = 0; i < (BLOCK_COUNT-2)/2; i++) {
    //        lfs_emubd_setwear(cfg, i+2, 0xffffffff)
    for i in 0..((block_count - 2) / 2) {
        env.bd.set_wear(i + 2, 0xffffffff);
    }

    let mut lfs = core::mem::MaybeUninit::<Lfs>::zeroed();
    assert_ok(lfs_format(
        lfs.as_mut_ptr(),
        &env.config as *const LfsConfig,
    ));

    assert_ok(lfs_mount(lfs.as_mut_ptr(), &env.config as *const LfsConfig));
    badblocks_create_dirs_and_files(lfs.as_mut_ptr());
    assert_ok(lfs_unmount(lfs.as_mut_ptr()));

    assert_ok(lfs_mount(lfs.as_mut_ptr(), &env.config as *const LfsConfig));
    badblocks_verify_dirs_and_files(lfs.as_mut_ptr());
    assert_ok(lfs_unmount(lfs.as_mut_ptr()));
}

/// Upstream: [cases.test_badblocks_alternating_corruption]
/// defines.ERASE_COUNT = 256
/// defines.ERASE_CYCLES = 0xffffffff
/// defines.ERASE_VALUE = [0x00, 0xff, -1]
/// defines.BADBLOCK_BEHAVIOR = [PROGERROR, ERASEERROR, READERROR, PROGNOOP, ERASENOOP]
/// defines.NAMEMULT = 64
/// defines.FILEMULT = 1
///
/// Mark every other block starting at 2 as worn. Format, create
/// 9 dirs+files, unmount, remount, verify.
#[rstest]
fn test_badblocks_alternating_corruption(
    #[values(0x00, 0xff, -1)] erase_value: i32,
    #[values(
        BadBlockBehavior::ProgError,
        BadBlockBehavior::EraseError,
        BadBlockBehavior::ReadError,
        BadBlockBehavior::ProgNoop,
        BadBlockBehavior::EraseNoop
    )]
    behavior: BadBlockBehavior,
) {
    let block_count: u32 = 256;

    let mut env = config_with_wear_leveling_full(block_count, 0xffffffff, behavior, erase_value);
    init_wear_leveling_context(&mut env);

    // C: for (lfs_block_t i = 0; i < (BLOCK_COUNT-2)/2; i++) {
    //        lfs_emubd_setwear(cfg, (2*i) + 2, 0xffffffff)
    for i in 0..((block_count - 2) / 2) {
        env.bd.set_wear((2 * i) + 2, 0xffffffff);
    }

    let mut lfs = core::mem::MaybeUninit::<Lfs>::zeroed();
    assert_ok(lfs_format(
        lfs.as_mut_ptr(),
        &env.config as *const LfsConfig,
    ));

    assert_ok(lfs_mount(lfs.as_mut_ptr(), &env.config as *const LfsConfig));
    badblocks_create_dirs_and_files(lfs.as_mut_ptr());
    assert_ok(lfs_unmount(lfs.as_mut_ptr()));

    assert_ok(lfs_mount(lfs.as_mut_ptr(), &env.config as *const LfsConfig));
    badblocks_verify_dirs_and_files(lfs.as_mut_ptr());
    assert_ok(lfs_unmount(lfs.as_mut_ptr()));
}

/// Upstream: [cases.test_badblocks_superblocks]
/// defines.ERASE_CYCLES = 0xffffffff
/// defines.ERASE_VALUE = [0x00, 0xff, -1]
/// defines.BADBLOCK_BEHAVIOR = [PROGERROR, ERASEERROR, READERROR, PROGNOOP, ERASENOOP]
///
/// Mark blocks 0 and 1 (superblocks) as worn.
/// Expect lfs_format to fail with LFS_ERR_NOSPC.
/// Expect lfs_mount to fail with LFS_ERR_CORRUPT.
#[rstest]
fn test_badblocks_superblocks(
    #[values(0x00, 0xff, -1)] erase_value: i32,
    #[values(
        BadBlockBehavior::ProgError,
        BadBlockBehavior::EraseError,
        BadBlockBehavior::ReadError,
        BadBlockBehavior::ProgNoop,
        BadBlockBehavior::EraseNoop
    )]
    behavior: BadBlockBehavior,
) {
    let block_count: u32 = 128;

    let mut env = config_with_wear_leveling_full(block_count, 0xffffffff, behavior, erase_value);
    init_wear_leveling_context(&mut env);

    env.bd.set_wear(0, 0xffffffff);
    env.bd.set_wear(1, 0xffffffff);

    let mut lfs = core::mem::MaybeUninit::<Lfs>::zeroed();
    let err = lfs_format(lfs.as_mut_ptr(), &env.config as *const LfsConfig);
    assert_eq!(
        err, LFS_ERR_NOSPC,
        "format should fail with NOSPC, got {err}"
    );

    let err = lfs_mount(lfs.as_mut_ptr(), &env.config as *const LfsConfig);
    assert_eq!(
        err, LFS_ERR_CORRUPT,
        "mount should fail with CORRUPT, got {err}"
    );
}

// ── Helpers shared by region/alternating tests ──────────────────────────────

fn badblocks_create_dirs_and_files(lfs: *mut Lfs) {
    for i in 1..10 {
        let mut buffer = [0u8; 1024];
        for j in 0..NAMEMULT {
            buffer[j] = b'0' + i as u8;
        }
        buffer[NAMEMULT] = 0;

        assert_ok(lfs_mkdir(lfs, buffer.as_ptr()));

        buffer[NAMEMULT] = b'/';
        for j in 0..NAMEMULT {
            buffer[j + NAMEMULT + 1] = b'0' + i as u8;
        }
        buffer[2 * NAMEMULT + 1] = 0;

        let mut file = core::mem::MaybeUninit::<LfsFile>::zeroed();
        assert_ok(lfs_file_open(
            lfs,
            file.as_mut_ptr(),
            buffer.as_ptr(),
            LFS_O_WRONLY | LFS_O_CREAT,
        ));

        let size = NAMEMULT as u32;
        for _j in 0..(i * FILEMULT) {
            let n = lfs_file_write(
                lfs,
                file.as_mut_ptr(),
                buffer.as_ptr() as *const core::ffi::c_void,
                size,
            );
            assert_eq!(n, size as i32);
        }

        assert_ok(lfs_file_close(lfs, file.as_mut_ptr()));
    }
}

fn badblocks_verify_dirs_and_files(lfs: *mut Lfs) {
    for i in 1..10 {
        let mut buffer = [0u8; 1024];
        for j in 0..NAMEMULT {
            buffer[j] = b'0' + i as u8;
        }
        buffer[NAMEMULT] = 0;

        let mut info = core::mem::MaybeUninit::<LfsInfo>::zeroed();
        assert_ok(lfs_stat(lfs, buffer.as_ptr(), info.as_mut_ptr()));
        let info_ref = unsafe { &*info.as_ptr() };
        assert_eq!(info_ref.type_, LFS_TYPE_DIR);

        buffer[NAMEMULT] = b'/';
        for j in 0..NAMEMULT {
            buffer[j + NAMEMULT + 1] = b'0' + i as u8;
        }
        buffer[2 * NAMEMULT + 1] = 0;

        let mut file = core::mem::MaybeUninit::<LfsFile>::zeroed();
        assert_ok(lfs_file_open(
            lfs,
            file.as_mut_ptr(),
            buffer.as_ptr(),
            LFS_O_RDONLY,
        ));

        let size = NAMEMULT as u32;
        for _j in 0..(i * FILEMULT) {
            let mut rbuffer = [0u8; 1024];
            let n = lfs_file_read(
                lfs,
                file.as_mut_ptr(),
                rbuffer.as_mut_ptr() as *mut core::ffi::c_void,
                size,
            );
            assert_eq!(n, size as i32);
            assert_eq!(&rbuffer[..size as usize], &buffer[..size as usize]);
        }

        assert_ok(lfs_file_close(lfs, file.as_mut_ptr()));
    }
}
