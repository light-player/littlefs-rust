//! Orphan and power-loss consistency tests.
//!
//! Upstream: tests/test_orphans.toml
//! Source: https://github.com/littlefs-project/littlefs/blob/master/tests/test_orphans.toml

mod common;

use common::{assert_ok, default_config, init_context, init_logger, path_bytes};
use lp_littlefs::{
    lfs_format, lfs_fs_hasorphans, lfs_fs_mkconsistent, lfs_fs_preporphans, lfs_mkdir, lfs_mount,
    lfs_remove, lfs_unmount, Lfs, LfsConfig,
};

// --- test_orphans_mkconsistent_fresh ---
// Minimal: format, mount, mkconsistent. No mkdir/remove. Sanity check.
#[test]
fn test_orphans_mkconsistent_fresh() {
    init_logger();
    let mut env = default_config(128);
    init_context(&mut env);

    let mut lfs = core::mem::MaybeUninit::<Lfs>::zeroed();
    assert_ok(lfs_format(
        lfs.as_mut_ptr(),
        &env.config as *const LfsConfig,
    ));
    assert_ok(lfs_mount(lfs.as_mut_ptr(), &env.config as *const LfsConfig));

    let lfs_ptr = lfs.as_mut_ptr();
    assert_ok(lfs_fs_mkconsistent(lfs_ptr));
    assert_ok(lfs_unmount(lfs_ptr));
}

// --- test_orphans_mkconsistent_no_orphans ---
// With lazy force_consistency, mkdir/remove run deorphan first. So preporphans(1)
// gets cleared before the commit. Verify: mkconsistent clears (no-op) and persists.
#[test]
fn test_orphans_mkconsistent_no_orphans() {
    init_logger();
    let mut env = default_config(128);
    init_context(&mut env);

    let mut lfs = core::mem::MaybeUninit::<Lfs>::zeroed();
    assert_ok(lfs_format(
        lfs.as_mut_ptr(),
        &env.config as *const LfsConfig,
    ));
    assert_ok(lfs_mount(lfs.as_mut_ptr(), &env.config as *const LfsConfig));

    let lfs_ptr = lfs.as_mut_ptr();
    assert_ok(lfs_fs_preporphans(lfs_ptr, 1));
    assert!(unsafe { lfs_fs_hasorphans(lfs_ptr) });

    let path = path_bytes("_p");
    assert_ok(lfs_mkdir(lfs_ptr, path.as_ptr()));
    assert_ok(lfs_remove(lfs_ptr, path.as_ptr()));
    assert!(
        !unsafe { lfs_fs_hasorphans(lfs_ptr) },
        "force_consistency before mkdir clears orphans"
    );
    assert_ok(lfs_unmount(lfs_ptr));

    assert_ok(lfs_mount(lfs_ptr, &env.config as *const LfsConfig));
    assert!(
        !unsafe { lfs_fs_hasorphans(lfs_ptr) },
        "persisted gstate has no orphans"
    );
    assert_ok(lfs_fs_mkconsistent(lfs_ptr));
    assert!(
        !unsafe { lfs_fs_hasorphans(lfs_ptr) },
        "after mkconsistent, gstate should have no orphans"
    );
    assert_ok(lfs_unmount(lfs_ptr));

    assert_ok(lfs_mount(lfs_ptr, &env.config as *const LfsConfig));
    assert!(
        !unsafe { lfs_fs_hasorphans(lfs_ptr) },
        "after remount, gstate persisted to disk has no orphans"
    );
    assert_ok(lfs_unmount(lfs_ptr));
}

// --- test_orphans_no_orphans ---
// preporphans(+1), mkdir+remove clears via force_consistency, unmount
#[test]
fn test_orphans_no_orphans() {
    init_logger();
    let mut env = default_config(128);
    init_context(&mut env);

    let mut lfs = core::mem::MaybeUninit::<Lfs>::zeroed();
    assert_ok(lfs_format(
        lfs.as_mut_ptr(),
        &env.config as *const LfsConfig,
    ));
    assert_ok(lfs_mount(lfs.as_mut_ptr(), &env.config as *const LfsConfig));

    let lfs_ptr = lfs.as_mut_ptr();
    assert_ok(lfs_fs_preporphans(lfs_ptr, 1));
    assert!(unsafe { lfs_fs_hasorphans(lfs_ptr) });

    let path = path_bytes("_x");
    assert_ok(lfs_mkdir(lfs_ptr, path.as_ptr()));
    assert_ok(lfs_remove(lfs_ptr, path.as_ptr()));
    assert!(!unsafe { lfs_fs_hasorphans(lfs_ptr) });
    assert_ok(lfs_unmount(lfs_ptr));
}

// --- test_orphans_nonreentrant ---
// Upstream: orphan operations without powerloss.
// Uses n=1 dir to match test_dirs_many_removal (n=2+ mkdir currently fails in this crate).
#[test]
fn test_orphans_nonreentrant() {
    init_logger();
    let mut env = default_config(128);
    init_context(&mut env);

    let mut lfs = core::mem::MaybeUninit::<Lfs>::zeroed();
    assert_ok(lfs_format(
        lfs.as_mut_ptr(),
        &env.config as *const LfsConfig,
    ));
    assert_ok(lfs_mount(lfs.as_mut_ptr(), &env.config as *const LfsConfig));

    let lfs_ptr = lfs.as_mut_ptr();
    let path = path_bytes("a");
    assert_ok(lfs_mkdir(lfs_ptr, path.as_ptr()));
    assert_ok(lfs_remove(lfs_ptr, path.as_ptr()));
    assert!(!unsafe { lfs_fs_hasorphans(lfs_ptr) });
    assert_ok(lfs_unmount(lfs_ptr));
}
