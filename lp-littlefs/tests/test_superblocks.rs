//! Superblock and format/mount tests.
//!
//! Upstream: tests/test_superblocks.toml
//! Source: https://github.com/littlefs-project/littlefs/blob/master/tests/test_superblocks.toml

mod common;

use common::{assert_err, assert_ok, assert_superblock_magic, default_config, init_context};
use lp_littlefs::{lfs_format, lfs_fs_stat, lfs_mount, lfs_unmount, Lfs, LfsConfig, LfsFsinfo};

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
#[test]
#[ignore = "stub"]
fn test_superblocks_mount_unknown_block_count() {
    todo!()
}

/// Upstream: [cases.test_superblocks_reentrant_format]
#[test]
#[ignore = "stub"]
fn test_superblocks_reentrant_format() {
    todo!()
}

/// Upstream: [cases.test_superblocks_stat_tweaked]
#[test]
#[ignore = "stub"]
fn test_superblocks_stat_tweaked() {
    todo!()
}

/// Upstream: [cases.test_superblocks_expand]
#[test]
#[ignore = "stub"]
fn test_superblocks_expand() {
    todo!()
}

/// Upstream: [cases.test_superblocks_magic_expand]
#[test]
#[ignore = "stub"]
fn test_superblocks_magic_expand() {
    todo!()
}

/// Upstream: [cases.test_superblocks_expand_power_cycle]
#[test]
#[ignore = "stub"]
fn test_superblocks_expand_power_cycle() {
    todo!()
}

/// Upstream: [cases.test_superblocks_reentrant_expand]
#[test]
#[ignore = "stub"]
fn test_superblocks_reentrant_expand() {
    todo!()
}

/// Upstream: [cases.test_superblocks_unknown_blocks]
#[test]
#[ignore = "stub"]
fn test_superblocks_unknown_blocks() {
    todo!()
}

/// Upstream: [cases.test_superblocks_fewer_blocks]
#[test]
#[ignore = "stub"]
fn test_superblocks_fewer_blocks() {
    todo!()
}

/// Upstream: [cases.test_superblocks_more_blocks]
#[test]
#[ignore = "stub"]
fn test_superblocks_more_blocks() {
    todo!()
}

/// Upstream: [cases.test_superblocks_grow]
#[test]
#[ignore = "stub"]
fn test_superblocks_grow() {
    todo!()
}

/// Upstream: [cases.test_superblocks_shrink]
#[test]
#[ignore = "stub"]
fn test_superblocks_shrink() {
    todo!()
}

/// Upstream: [cases.test_superblocks_metadata_max]
#[test]
#[ignore = "stub"]
fn test_superblocks_metadata_max() {
    todo!()
}
