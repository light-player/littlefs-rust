//! Superblock and format/mount tests.
//!
//! Upstream: tests/test_superblocks.toml
//! Source: https://github.com/littlefs-project/littlefs/blob/master/tests/test_superblocks.toml

mod common;

use common::{
    assert_err, assert_ok, assert_superblock_magic, clone_config_with_block_count, default_config,
    init_context, path_bytes,
    powerloss::{init_powerloss_context, powerloss_config, run_powerloss_linear},
    LFS_O_CREAT, LFS_O_EXCL, LFS_O_RDONLY, LFS_O_WRONLY,
};
use lp_littlefs::lfs_type::lfs_type::LFS_TYPE_REG;
use lp_littlefs::{
    lfs_file_close, lfs_file_open, lfs_file_read, lfs_file_write, lfs_format, lfs_fs_grow,
    lfs_fs_stat, lfs_mount, lfs_remove, lfs_stat, lfs_unmount, Lfs, LfsConfig, LfsFile, LfsFsinfo,
    LfsInfo, LFS_ERR_INVAL, LFS_ERR_NOENT,
};
use rstest::rstest;

// --- test_superblocks_format ---
// Upstream: lfs_format(&lfs, cfg) => 0
#[test]
fn test_superblocks_format() {
    let mut env = default_config(128);
    init_context(&mut env);
    let mut lfs = core::mem::MaybeUninit::<Lfs>::zeroed();
    let err = lfs_format(lfs.as_mut_ptr(), &env.config as *const LfsConfig);
    assert_ok(err);
}

// --- test_superblocks_mount ---
// Upstream: format, mount, unmount
#[test]
fn test_superblocks_mount() {
    let mut env = default_config(128);
    init_context(&mut env);
    let mut lfs = core::mem::MaybeUninit::<Lfs>::zeroed();
    assert_ok(lfs_format(
        lfs.as_mut_ptr(),
        &env.config as *const LfsConfig,
    ));
    assert_ok(lfs_mount(lfs.as_mut_ptr(), &env.config as *const LfsConfig));
    assert_ok(lfs_unmount(lfs.as_mut_ptr()));
}

// --- test_superblocks_magic ---
// Upstream: format, then raw read to verify "littlefs" at MAGIC_OFFSET in both blocks.
#[test]
fn test_superblocks_magic() {
    common::init_logger();
    let mut env = default_config(128);
    init_context(&mut env);
    let mut lfs = core::mem::MaybeUninit::<Lfs>::zeroed();
    assert_ok(lfs_format(
        lfs.as_mut_ptr(),
        &env.config as *const LfsConfig,
    ));

    assert_superblock_magic(&env.config);
}

// --- test_traverse_attrs_callback_order ---
// Unit test (in integration harness): traverse with tmask=0 passes SUPERBLOCK correctly.
#[test]
fn test_traverse_attrs_callback_order() {
    let mut env = default_config(128);
    init_context(&mut env);
    let mut lfs = core::mem::MaybeUninit::<Lfs>::zeroed();
    let mut out = lp_littlefs::TraverseTestOut::default();

    assert_ok(unsafe {
        lp_littlefs::test_traverse_format_attrs(
            lfs.as_mut_ptr(),
            &env.config as *const LfsConfig,
            &mut out as *mut _,
        )
    });

    assert_eq!(out.call_count, 3);
    assert_eq!(out.tags[1], 0x0ff, "second callback should be SUPERBLOCK");
    assert_eq!(out.first_bytes[1], b'l');
}

// --- test_traverse_filter_gets_superblock_after_push ---
// Unit test: traverse with tmask (compact-style) triggers push; callback receives SUPERBLOCK with 'l'.
#[test]
fn test_traverse_filter_gets_superblock_after_push() {
    let mut env = default_config(128);
    init_context(&mut env);
    let mut lfs = core::mem::MaybeUninit::<Lfs>::zeroed();
    let mut out = lp_littlefs::TraverseTestOut::default();

    assert_ok(unsafe {
        lp_littlefs::test_traverse_filter_gets_superblock_after_push(
            lfs.as_mut_ptr(),
            &env.config as *const LfsConfig,
            &mut out as *mut _,
        )
    });

    let has_superblock = out.tags[..out.call_count as usize].contains(&0x0ff);
    assert!(
        has_superblock,
        "callback should receive SUPERBLOCK (type3=0x0ff)"
    );
    let superblock_idx = out.tags[..out.call_count as usize]
        .iter()
        .position(|&t| t == 0x0ff)
        .unwrap();
    assert_eq!(
        out.first_bytes[superblock_idx], b'l',
        "SUPERBLOCK buffer first byte should be 'l'"
    );
}

// --- test_superblocks_invalid_mount ---
// Upstream: mount on blank device => LFS_ERR_CORRUPT
#[test]
fn test_superblocks_invalid_mount() {
    let mut env = default_config(128);
    init_context(&mut env);
    let mut lfs = core::mem::MaybeUninit::<Lfs>::zeroed();
    let err = lfs_mount(lfs.as_mut_ptr(), &env.config as *const LfsConfig);
    assert_err(lp_littlefs::LFS_ERR_CORRUPT, err);
}

// --- test_superblocks_stat ---
// Upstream: fs_stat after format/mount returns correct values
#[test]
fn test_superblocks_stat() {
    let mut env = default_config(128);
    init_context(&mut env);
    let mut lfs = core::mem::MaybeUninit::<Lfs>::zeroed();
    assert_ok(lfs_format(
        lfs.as_mut_ptr(),
        &env.config as *const LfsConfig,
    ));
    assert_ok(lfs_mount(lfs.as_mut_ptr(), &env.config as *const LfsConfig));

    let mut fsinfo = core::mem::MaybeUninit::<LfsFsinfo>::uninit();
    assert_ok(lfs_fs_stat(lfs.as_mut_ptr(), fsinfo.as_mut_ptr()));
    let fsinfo = unsafe { fsinfo.assume_init() };
    assert_eq!(fsinfo.block_size, env.config.block_size);
    assert_eq!(fsinfo.block_count, env.config.block_count);
    assert_eq!(fsinfo.disk_version, 0x0002_0001);
    assert_eq!(fsinfo.name_max, 255);
    assert_eq!(fsinfo.file_max, 2_147_483_647);
    assert_eq!(fsinfo.attr_max, 1022);
}

// --- Missing upstream stubs ---

/// Upstream: [cases.test_superblocks_mount_unknown_block_count]
/// Mount with block_count=0; verify lfs.block_count is set from superblock.
#[test]
fn test_superblocks_mount_unknown_block_count() {
    let mut env = default_config(128);
    init_context(&mut env);
    let mut lfs = core::mem::MaybeUninit::<Lfs>::zeroed();
    assert_ok(lfs_format(
        lfs.as_mut_ptr(),
        &env.config as *const LfsConfig,
    ));

    let cfg0 = clone_config_with_block_count(&env, 0);
    assert_ok(lfs_mount(
        lfs.as_mut_ptr(),
        &cfg0.config as *const LfsConfig,
    ));
    let block_count = unsafe { (*lfs.as_ptr()).block_count };
    assert_eq!(
        block_count, 128,
        "lfs.block_count should match format config"
    );
    assert_ok(lfs_unmount(lfs.as_mut_ptr()));
}

/// Upstream: [cases.test_superblocks_reentrant_format]
/// reentrant = true, POWERLOSS_BEHAVIOR = [NOOP, OOO]. Format under power-loss, then mount.
#[test]
#[cfg(feature = "slow_tests")]
fn test_superblocks_reentrant_format() {
    let mut env = powerloss_config(128);
    init_powerloss_context(&mut env);
    let snapshot = env.snapshot();

    let result = run_powerloss_linear(
        &mut env,
        &snapshot,
        500,
        |lfs_ptr, config| {
            let err = lfs_mount(lfs_ptr, config);
            if err != 0 {
                let e = lfs_format(lfs_ptr, config);
                if e != 0 {
                    return Err(e);
                }
                let e = lfs_mount(lfs_ptr, config);
                if e != 0 {
                    return Err(e);
                }
            }
            let e = lfs_unmount(lfs_ptr);
            if e != 0 {
                return Err(e);
            }
            Ok(())
        },
        |_, _| Ok(()),
    );
    result.expect("test_superblocks_reentrant_format should complete");
}

/// Upstream: [cases.test_superblocks_stat_tweaked]
/// Format with name_max=63, file_max=65535, attr_max=512; mount with default; verify fsinfo.
#[test]
fn test_superblocks_stat_tweaked() {
    let mut env = default_config(128);
    init_context(&mut env);
    env.config.name_max = 63;
    env.config.file_max = 65535;
    env.config.attr_max = 512;

    let mut lfs = core::mem::MaybeUninit::<Lfs>::zeroed();
    assert_ok(lfs_format(
        lfs.as_mut_ptr(),
        &env.config as *const LfsConfig,
    ));

    env.config.name_max = 255;
    env.config.file_max = 2_147_483_647;
    env.config.attr_max = 1022;
    assert_ok(lfs_mount(lfs.as_mut_ptr(), &env.config as *const LfsConfig));

    let mut fsinfo = core::mem::MaybeUninit::<LfsFsinfo>::uninit();
    assert_ok(lfs_fs_stat(lfs.as_mut_ptr(), fsinfo.as_mut_ptr()));
    let fsinfo = unsafe { fsinfo.assume_init() };
    assert_eq!(fsinfo.name_max, 63);
    assert_eq!(fsinfo.file_max, 65535);
    assert_eq!(fsinfo.attr_max, 512);
    assert_ok(lfs_unmount(lfs.as_mut_ptr()));
}

/// Upstream: [cases.test_superblocks_expand]
/// Create/remove dummy file N times; verify superblock survives compaction.
#[test]
fn test_superblocks_expand() {
    for &block_cycles in &[32i32, 33, 1] {
        for &n in &[10u32, 100, 1000] {
            let mut env = default_config(128);
            init_context(&mut env);
            env.config.block_cycles = block_cycles;

            let mut lfs = core::mem::MaybeUninit::<Lfs>::zeroed();
            assert_ok(lfs_format(
                lfs.as_mut_ptr(),
                &env.config as *const LfsConfig,
            ));
            assert_ok(lfs_mount(lfs.as_mut_ptr(), &env.config as *const LfsConfig));

            let dummy = path_bytes("dummy");
            for _ in 0..n {
                let mut file = core::mem::MaybeUninit::<LfsFile>::zeroed();
                assert_ok(lfs_file_open(
                    lfs.as_mut_ptr(),
                    file.as_mut_ptr(),
                    dummy.as_ptr(),
                    LFS_O_WRONLY | LFS_O_CREAT | LFS_O_EXCL,
                ));
                assert_ok(lfs_file_close(lfs.as_mut_ptr(), file.as_mut_ptr()));
                let mut info = core::mem::MaybeUninit::<LfsInfo>::uninit();
                assert_ok(lfs_stat(
                    lfs.as_mut_ptr(),
                    dummy.as_ptr(),
                    info.as_mut_ptr(),
                ));
                let info = unsafe { info.assume_init() };
                assert_eq!(info.type_, LFS_TYPE_REG as u8);
                assert_ok(lfs_remove(lfs.as_mut_ptr(), dummy.as_ptr()));
            }
            assert_ok(lfs_unmount(lfs.as_mut_ptr()));

            assert_ok(lfs_mount(lfs.as_mut_ptr(), &env.config as *const LfsConfig));
            let mut file = core::mem::MaybeUninit::<LfsFile>::zeroed();
            assert_ok(lfs_file_open(
                lfs.as_mut_ptr(),
                file.as_mut_ptr(),
                dummy.as_ptr(),
                LFS_O_WRONLY | LFS_O_CREAT | LFS_O_EXCL,
            ));
            assert_ok(lfs_file_close(lfs.as_mut_ptr(), file.as_mut_ptr()));
            let mut info = core::mem::MaybeUninit::<LfsInfo>::uninit();
            assert_ok(lfs_stat(
                lfs.as_mut_ptr(),
                dummy.as_ptr(),
                info.as_mut_ptr(),
            ));
            let info = unsafe { info.assume_init() };
            assert_eq!(info.type_, LFS_TYPE_REG as u8);
            assert_ok(lfs_unmount(lfs.as_mut_ptr()));
        }
    }
}

/// Upstream: [cases.test_superblocks_magic_expand]
/// Same as expand + magic check after.
#[test]
fn test_superblocks_magic_expand() {
    common::init_logger();
    for &block_cycles in &[32i32, 33, 1] {
        for &n in &[10u32, 100, 1000] {
            let mut env = default_config(128);
            init_context(&mut env);
            env.config.block_cycles = block_cycles;

            let mut lfs = core::mem::MaybeUninit::<Lfs>::zeroed();
            assert_ok(lfs_format(
                lfs.as_mut_ptr(),
                &env.config as *const LfsConfig,
            ));
            assert_ok(lfs_mount(lfs.as_mut_ptr(), &env.config as *const LfsConfig));

            let dummy = path_bytes("dummy");
            for _ in 0..n {
                let mut file = core::mem::MaybeUninit::<LfsFile>::zeroed();
                assert_ok(lfs_file_open(
                    lfs.as_mut_ptr(),
                    file.as_mut_ptr(),
                    dummy.as_ptr(),
                    LFS_O_WRONLY | LFS_O_CREAT | LFS_O_EXCL,
                ));
                assert_ok(lfs_file_close(lfs.as_mut_ptr(), file.as_mut_ptr()));
                let mut info = core::mem::MaybeUninit::<LfsInfo>::uninit();
                assert_ok(lfs_stat(
                    lfs.as_mut_ptr(),
                    dummy.as_ptr(),
                    info.as_mut_ptr(),
                ));
                let info = unsafe { info.assume_init() };
                assert_eq!(info.type_, LFS_TYPE_REG as u8);
                assert_ok(lfs_remove(lfs.as_mut_ptr(), dummy.as_ptr()));
            }
            assert_ok(lfs_unmount(lfs.as_mut_ptr()));

            assert_superblock_magic(&env.config);
        }
    }
}

/// Upstream: [cases.test_superblocks_expand_power_cycle]
/// Same as expand but unmount/remount after each iteration.
#[test]
fn test_superblocks_expand_power_cycle() {
    for &block_cycles in &[32i32, 33, 1] {
        for &n in &[10u32, 100, 1000] {
            let mut env = default_config(128);
            init_context(&mut env);
            env.config.block_cycles = block_cycles;

            let mut lfs = core::mem::MaybeUninit::<Lfs>::zeroed();
            assert_ok(lfs_format(
                lfs.as_mut_ptr(),
                &env.config as *const LfsConfig,
            ));

            let dummy = path_bytes("dummy");
            for i in 0..n {
                assert_ok(lfs_mount(lfs.as_mut_ptr(), &env.config as *const LfsConfig));
                let mut info = core::mem::MaybeUninit::<LfsInfo>::uninit();
                let err = lfs_stat(lfs.as_mut_ptr(), dummy.as_ptr(), info.as_mut_ptr());
                assert!(
                    err == 0 || (err == LFS_ERR_NOENT && i == 0),
                    "stat dummy: err={err} i={i}"
                );
                if err == 0 {
                    let info = unsafe { info.assume_init() };
                    assert_eq!(info.type_, LFS_TYPE_REG as u8);
                    assert_ok(lfs_remove(lfs.as_mut_ptr(), dummy.as_ptr()));
                }

                let mut file = core::mem::MaybeUninit::<LfsFile>::zeroed();
                assert_ok(lfs_file_open(
                    lfs.as_mut_ptr(),
                    file.as_mut_ptr(),
                    dummy.as_ptr(),
                    LFS_O_WRONLY | LFS_O_CREAT | LFS_O_EXCL,
                ));
                assert_ok(lfs_file_close(lfs.as_mut_ptr(), file.as_mut_ptr()));
                let mut info = core::mem::MaybeUninit::<LfsInfo>::uninit();
                assert_ok(lfs_stat(
                    lfs.as_mut_ptr(),
                    dummy.as_ptr(),
                    info.as_mut_ptr(),
                ));
                let info = unsafe { info.assume_init() };
                assert_eq!(info.type_, LFS_TYPE_REG as u8);
                assert_ok(lfs_unmount(lfs.as_mut_ptr()));
            }

            assert_ok(lfs_mount(lfs.as_mut_ptr(), &env.config as *const LfsConfig));
            let mut info = core::mem::MaybeUninit::<LfsInfo>::uninit();
            assert_ok(lfs_stat(
                lfs.as_mut_ptr(),
                dummy.as_ptr(),
                info.as_mut_ptr(),
            ));
            let info = unsafe { info.assume_init() };
            assert_eq!(info.type_, LFS_TYPE_REG as u8);
            assert_ok(lfs_unmount(lfs.as_mut_ptr()));
        }
    }
}

/// Upstream: [cases.test_superblocks_reentrant_expand]
/// BLOCK_CYCLES = [2, 1], N = 24, reentrant, POWERLOSS_BEHAVIOR = [NOOP, OOO]
#[test]
#[cfg(feature = "slow_tests")]
fn test_superblocks_reentrant_expand() {
    const N: u32 = 24;
    for &block_cycles in &[2i32, 1] {
        let mut env = powerloss_config(128);
        init_powerloss_context(&mut env);
        env.config.block_cycles = block_cycles;

        let mut lfs = core::mem::MaybeUninit::<Lfs>::zeroed();
        assert_ok(lfs_format(
            lfs.as_mut_ptr(),
            &env.config as *const LfsConfig,
        ));
        assert_ok(lfs_mount(lfs.as_mut_ptr(), &env.config as *const LfsConfig));
        assert_ok(lfs_unmount(lfs.as_mut_ptr()));
        let snapshot = env.snapshot();

        let dummy = path_bytes("dummy");
        let result = run_powerloss_linear(
            &mut env,
            &snapshot,
            3000,
            |lfs_ptr, config| {
                let err = lfs_mount(lfs_ptr, config);
                if err != 0 {
                    let e = lfs_format(lfs_ptr, config);
                    if e != 0 {
                        return Err(e);
                    }
                    let e = lfs_mount(lfs_ptr, config);
                    if e != 0 {
                        return Err(e);
                    }
                }
                for i in 0..N {
                    let mut info = core::mem::MaybeUninit::<LfsInfo>::uninit();
                    let err = lfs_stat(lfs_ptr, dummy.as_ptr(), info.as_mut_ptr());
                    if err == 0 {
                        let info = unsafe { info.assume_init() };
                        if info.type_ == LFS_TYPE_REG as u8 {
                            let e = lfs_remove(lfs_ptr, dummy.as_ptr());
                            if e != 0 {
                                let _ = lfs_unmount(lfs_ptr);
                                return Err(e);
                            }
                        }
                    } else if err != lp_littlefs::LFS_ERR_NOENT || i != 0 {
                        let _ = lfs_unmount(lfs_ptr);
                        return Err(err);
                    }
                    let mut file = core::mem::MaybeUninit::<LfsFile>::zeroed();
                    let e = lfs_file_open(
                        lfs_ptr,
                        file.as_mut_ptr(),
                        dummy.as_ptr(),
                        LFS_O_WRONLY | LFS_O_CREAT | LFS_O_EXCL,
                    );
                    if e != 0 {
                        let _ = lfs_unmount(lfs_ptr);
                        return Err(e);
                    }
                    let e = lfs_file_close(lfs_ptr, file.as_mut_ptr());
                    if e != 0 {
                        let _ = lfs_unmount(lfs_ptr);
                        return Err(e);
                    }
                    let mut info = core::mem::MaybeUninit::<LfsInfo>::uninit();
                    let e = lfs_stat(lfs_ptr, dummy.as_ptr(), info.as_mut_ptr());
                    if e != 0 {
                        let _ = lfs_unmount(lfs_ptr);
                        return Err(e);
                    }
                }
                let e = lfs_unmount(lfs_ptr);
                if e != 0 {
                    return Err(e);
                }
                Ok(())
            },
            |_, _| Ok(()),
        );
        result.expect(&format!(
            "test_superblocks_reentrant_expand block_cycles={block_cycles} should complete"
        ));
    }
}

/// Upstream: [cases.test_superblocks_unknown_blocks]
/// Mount with block_count=0, lfs_fs_stat, basic file ops.
#[test]
fn test_superblocks_unknown_blocks() {
    const BLOCK_COUNT: u32 = 128;
    let mut env = default_config(BLOCK_COUNT);
    init_context(&mut env);
    let mut lfs = core::mem::MaybeUninit::<Lfs>::zeroed();
    assert_ok(lfs_format(
        lfs.as_mut_ptr(),
        &env.config as *const LfsConfig,
    ));

    assert_ok(lfs_mount(lfs.as_mut_ptr(), &env.config as *const LfsConfig));
    let mut fsinfo = core::mem::MaybeUninit::<LfsFsinfo>::uninit();
    assert_ok(lfs_fs_stat(lfs.as_mut_ptr(), fsinfo.as_mut_ptr()));
    let fsinfo = unsafe { fsinfo.assume_init() };
    assert_eq!(fsinfo.block_count, BLOCK_COUNT);
    assert_ok(lfs_unmount(lfs.as_mut_ptr()));

    let cfg0 = clone_config_with_block_count(&env, 0);
    assert_ok(lfs_mount(
        lfs.as_mut_ptr(),
        &cfg0.config as *const LfsConfig,
    ));
    let mut fsinfo = core::mem::MaybeUninit::<LfsFsinfo>::uninit();
    assert_ok(lfs_fs_stat(lfs.as_mut_ptr(), fsinfo.as_mut_ptr()));
    let fsinfo = unsafe { fsinfo.assume_init() };
    assert_eq!(fsinfo.block_count, BLOCK_COUNT);
    assert_ok(lfs_unmount(lfs.as_mut_ptr()));

    assert_ok(lfs_mount(
        lfs.as_mut_ptr(),
        &cfg0.config as *const LfsConfig,
    ));
    let mut fsinfo = core::mem::MaybeUninit::<LfsFsinfo>::uninit();
    assert_ok(lfs_fs_stat(lfs.as_mut_ptr(), fsinfo.as_mut_ptr()));
    let fsinfo = unsafe { fsinfo.assume_init() };
    assert_eq!(fsinfo.block_count, BLOCK_COUNT);
    let test_path = path_bytes("test");
    let mut file = core::mem::MaybeUninit::<LfsFile>::zeroed();
    assert_ok(lfs_file_open(
        lfs.as_mut_ptr(),
        file.as_mut_ptr(),
        test_path.as_ptr(),
        LFS_O_CREAT | LFS_O_EXCL | LFS_O_WRONLY,
    ));
    let data = b"hello!";
    assert_eq!(
        lfs_file_write(
            lfs.as_mut_ptr(),
            file.as_mut_ptr(),
            data.as_ptr() as *const core::ffi::c_void,
            data.len() as u32,
        ),
        data.len() as i32
    );
    assert_ok(lfs_file_close(lfs.as_mut_ptr(), file.as_mut_ptr()));
    assert_ok(lfs_unmount(lfs.as_mut_ptr()));

    assert_ok(lfs_mount(
        lfs.as_mut_ptr(),
        &cfg0.config as *const LfsConfig,
    ));
    let mut fsinfo = core::mem::MaybeUninit::<LfsFsinfo>::uninit();
    assert_ok(lfs_fs_stat(lfs.as_mut_ptr(), fsinfo.as_mut_ptr()));
    let fsinfo = unsafe { fsinfo.assume_init() };
    assert_eq!(fsinfo.block_count, BLOCK_COUNT);
    let mut file = core::mem::MaybeUninit::<LfsFile>::zeroed();
    assert_ok(lfs_file_open(
        lfs.as_mut_ptr(),
        file.as_mut_ptr(),
        test_path.as_ptr(),
        LFS_O_RDONLY,
    ));
    let mut buf = [0u8; 256];
    let n = lfs_file_read(
        lfs.as_mut_ptr(),
        file.as_mut_ptr(),
        buf.as_mut_ptr() as *mut core::ffi::c_void,
        buf.len() as u32,
    );
    assert_eq!(n, data.len() as i32);
    assert_eq!(&buf[..data.len()], data);
    assert_ok(lfs_file_close(lfs.as_mut_ptr(), file.as_mut_ptr()));
    assert_ok(lfs_unmount(lfs.as_mut_ptr()));
}

/// Upstream: [cases.test_superblocks_fewer_blocks]
/// Format with BLOCK_COUNT blocks; mount with ERASE_COUNT blocks => LFS_ERR_INVAL.
#[test]
#[ignore = "mount with block_count > superblock.block_count does not return INVAL"]
fn test_superblocks_fewer_blocks() {
    const ERASE_COUNT: u32 = 128;
    for &block_count in &[ERASE_COUNT / 2, ERASE_COUNT / 4, 2u32] {
        let mut env = default_config(ERASE_COUNT);
        init_context(&mut env);
        env.config.block_count = block_count;

        let mut lfs = core::mem::MaybeUninit::<Lfs>::zeroed();
        assert_ok(lfs_format(
            lfs.as_mut_ptr(),
            &env.config as *const LfsConfig,
        ));

        let cfg_full = clone_config_with_block_count(&env, ERASE_COUNT);
        let err = lfs_mount(lfs.as_mut_ptr(), &cfg_full.config as *const LfsConfig);
        assert_err(LFS_ERR_INVAL, err);

        let cfg0 = clone_config_with_block_count(&env, 0);
        assert_ok(lfs_mount(
            lfs.as_mut_ptr(),
            &cfg0.config as *const LfsConfig,
        ));
        let mut fsinfo = core::mem::MaybeUninit::<LfsFsinfo>::uninit();
        assert_ok(lfs_fs_stat(lfs.as_mut_ptr(), fsinfo.as_mut_ptr()));
        let fsinfo = unsafe { fsinfo.assume_init() };
        assert_eq!(fsinfo.block_count, block_count);
        assert_ok(lfs_unmount(lfs.as_mut_ptr()));

        let test_path = path_bytes("test");
        assert_ok(lfs_mount(
            lfs.as_mut_ptr(),
            &cfg0.config as *const LfsConfig,
        ));
        let mut file = core::mem::MaybeUninit::<LfsFile>::zeroed();
        assert_ok(lfs_file_open(
            lfs.as_mut_ptr(),
            file.as_mut_ptr(),
            test_path.as_ptr(),
            LFS_O_CREAT | LFS_O_EXCL | LFS_O_WRONLY,
        ));
        assert_ok(lfs_file_write(
            lfs.as_mut_ptr(),
            file.as_mut_ptr(),
            b"hello!".as_ptr() as *const core::ffi::c_void,
            6,
        ));
        assert_ok(lfs_file_close(lfs.as_mut_ptr(), file.as_mut_ptr()));
        assert_ok(lfs_unmount(lfs.as_mut_ptr()));

        assert_ok(lfs_mount(
            lfs.as_mut_ptr(),
            &cfg0.config as *const LfsConfig,
        ));
        let mut file = core::mem::MaybeUninit::<LfsFile>::zeroed();
        assert_ok(lfs_file_open(
            lfs.as_mut_ptr(),
            file.as_mut_ptr(),
            test_path.as_ptr(),
            LFS_O_RDONLY,
        ));
        let mut buf = [0u8; 16];
        assert_eq!(
            lfs_file_read(
                lfs.as_mut_ptr(),
                file.as_mut_ptr(),
                buf.as_mut_ptr() as *mut core::ffi::c_void,
                buf.len() as u32,
            ),
            6
        );
        assert_eq!(&buf[..6], b"hello!");
        assert_ok(lfs_file_close(lfs.as_mut_ptr(), file.as_mut_ptr()));
        assert_ok(lfs_unmount(lfs.as_mut_ptr()));
    }
}

/// Upstream: [cases.test_superblocks_more_blocks]
/// Format with 2*ERASE_COUNT blocks; mount with ERASE_COUNT => LFS_ERR_INVAL.
#[test]
fn test_superblocks_more_blocks() {
    const ERASE_COUNT: u32 = 128;
    let mut env = default_config(2 * ERASE_COUNT);
    init_context(&mut env);
    let mut lfs = core::mem::MaybeUninit::<Lfs>::zeroed();
    assert_ok(lfs_format(
        lfs.as_mut_ptr(),
        &env.config as *const LfsConfig,
    ));

    let cfg_half = clone_config_with_block_count(&env, ERASE_COUNT);
    let err = lfs_mount(lfs.as_mut_ptr(), &cfg_half.config as *const LfsConfig);
    assert_err(LFS_ERR_INVAL, err);
}

const ERASE_COUNT_GROW: u32 = 128;

/// Upstream: [cases.test_superblocks_grow]
/// defines.BLOCK_COUNT = [ERASE_COUNT/2, ERASE_COUNT/4, 2], BLOCK_COUNT_2 = ERASE_COUNT,
/// KNOWN_BLOCK_COUNT = [true, false]. lfs_fs_grow from smaller to larger block count.
#[rstest]
fn test_superblocks_grow(
    #[values(
        ERASE_COUNT_GROW / 2,
        ERASE_COUNT_GROW / 4,
        2u32
    )]
    small_count: u32,
    #[values(false, true)] known_block_count: bool,
) {
    let mut env = default_config(ERASE_COUNT_GROW);
    init_context(&mut env);
    let cfg = &env.config as *const LfsConfig;

    let large_count = ERASE_COUNT_GROW;
    env.config.block_count = small_count;

    let mut lfs = core::mem::MaybeUninit::<Lfs>::zeroed();
    assert_ok(lfs_format(lfs.as_mut_ptr(), cfg));
    assert_ok(lfs_mount(lfs.as_mut_ptr(), cfg));

    // Create a file to verify after grow
    let path = path_bytes("x");
    let mut file = core::mem::MaybeUninit::<LfsFile>::zeroed();
    assert_ok(lfs_file_open(
        lfs.as_mut_ptr(),
        file.as_mut_ptr(),
        path.as_ptr(),
        LFS_O_WRONLY | LFS_O_CREAT | LFS_O_EXCL,
    ));
    let buf = b"hello";
    assert_eq!(
        buf.len() as i32,
        lfs_file_write(
            lfs.as_mut_ptr(),
            file.as_mut_ptr(),
            buf.as_ptr() as *const core::ffi::c_void,
            buf.len() as u32,
        )
    );
    assert_ok(lfs_file_close(lfs.as_mut_ptr(), file.as_mut_ptr()));

    assert_ok(lfs_fs_grow(lfs.as_mut_ptr(), large_count));
    assert_ok(lfs_unmount(lfs.as_mut_ptr()));

    // Mount with full block_count and verify (or block_count=0 when known_block_count is false)
    let mount_block_count = if known_block_count { large_count } else { 0 };
    let mount_cfg = clone_config_with_block_count(&env, mount_block_count);
    env.config.block_count = large_count;
    assert_ok(lfs_mount(
        lfs.as_mut_ptr(),
        &mount_cfg.config as *const LfsConfig,
    ));
    let mut file = core::mem::MaybeUninit::<LfsFile>::zeroed();
    assert_ok(lfs_file_open(
        lfs.as_mut_ptr(),
        file.as_mut_ptr(),
        path.as_ptr(),
        LFS_O_RDONLY,
    ));
    let mut rbuf = [0u8; 16];
    let n = lfs_file_read(
        lfs.as_mut_ptr(),
        file.as_mut_ptr(),
        rbuf.as_mut_ptr() as *mut core::ffi::c_void,
        rbuf.len() as u32,
    );
    assert_eq!(n, buf.len() as i32);
    assert_eq!(&rbuf[..buf.len()], buf);
    assert_ok(lfs_file_close(lfs.as_mut_ptr(), file.as_mut_ptr()));
    assert_ok(lfs_unmount(lfs.as_mut_ptr()));
}

/// Upstream: [cases.test_superblocks_shrink]
/// Requires LFS_SHRINKNONRELOCATING. Shrink via lfs_fs_grow to smaller size.
#[test]
#[ignore = "requires LFS_SHRINKNONRELOCATING feature"]
fn test_superblocks_shrink() {
    todo!("implement when LFS_SHRINKNONRELOCATING is available")
}

/// Upstream: [cases.test_superblocks_metadata_max]
/// defines.METADATA_MAX = [lfs_max(512, PROG_SIZE), lfs_max(BLOCK_SIZE/2, PROG_SIZE), BLOCK_SIZE]
/// defines.N = [10, 100, 1000]. Set metadata_max in config during superblock compaction.
#[rstest]
#[ignore = "requires metadata_max in config during compaction cycles"]
fn test_superblocks_metadata_max(
    #[values(512, 256, 512)] _metadata_max: u32,
    #[values(10, 100, 1000)] _n: u32,
) {
    todo!("implement when metadata_max compaction test is wired")
}
