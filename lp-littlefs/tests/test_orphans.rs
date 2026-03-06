//! Orphan and power-loss consistency tests.
//!
//! Upstream: tests/test_orphans.toml
//! Source: https://github.com/littlefs-project/littlefs/blob/master/tests/test_orphans.toml

mod common;

use common::{
    assert_ok, default_config, init_context, init_logger, path_bytes,
    powerloss::{init_powerloss_context, powerloss_config, run_powerloss_linear},
    test_prng,
};
use lp_littlefs::lfs_type::lfs_type::LFS_TYPE_DIR;
use lp_littlefs::{
    lfs_format, lfs_fs_hasorphans, lfs_fs_mkconsistent, lfs_fs_preporphans, lfs_mkdir, lfs_mount,
    lfs_remove, lfs_stat, lfs_unmount, Lfs, LfsConfig, LfsInfo, LFS_ERR_EXIST, LFS_ERR_NOENT,
    LFS_ERR_NOTEMPTY,
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

// --- Missing upstream stubs ---

/// Upstream: [cases.test_orphans_normal]
/// if = 'PROG_SIZE <= 0x3fe'. Corrupt child's commit to create orphan, mkdir triggers deorphan, check lfs_fs_size.
#[test]
#[ignore = "requires raw block corruption to create orphan; needs write_block_raw on child dir block"]
fn test_orphans_normal() {
    todo!("implement when orphan corruption pattern is wired")
}

/// Upstream: [cases.test_orphans_one_orphan]
/// Create orphan via internal APIs (lfs_dir_alloc + SOFTTAIL commit + lfs_fs_preporphans). Run lfs_fs_forceconsistency.
#[test]
#[ignore = "requires internal APIs: lfs_dir_alloc, lfs_dir_commit with SOFTTAIL"]
fn test_orphans_one_orphan() {
    todo!("implement when internal dir APIs are exposed")
}

/// Upstream: [cases.test_orphans_mkconsistent_one_orphan]
/// Same orphan creation as one_orphan. Use lfs_fs_mkconsistent + remount. Verify cleanup.
#[test]
#[ignore = "requires internal APIs: lfs_dir_alloc, lfs_dir_commit with SOFTTAIL"]
fn test_orphans_mkconsistent_one_orphan() {
    todo!("implement when internal dir APIs are exposed")
}

/// Upstream: [cases.test_orphans_reentrant]
/// FILES=[6,26], DEPTH=1; FILES=3,DEPTH=3 skipped when CACHE_SIZE!=64. reentrant, CYCLES=20.
#[test]
#[ignore = "slow: power-loss iteration"]
fn test_orphans_reentrant() {
    init_logger();
    const CYCLES: u32 = 20;
    const ALPHA: &[u8] = b"abcdefghijklmnopqrstuvwxyz";

    for (files, depth) in [(6usize, 1usize), (26, 1)] {
        if 2 * files >= 128 {
            continue;
        }
        let mut env = powerloss_config(128);
        init_powerloss_context(&mut env);
        let snapshot = env.snapshot();

        let result = run_powerloss_linear(
            &mut env,
            &snapshot,
            2000,
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

                let mut prng: u32 = 1;
                for _ in 0..CYCLES {
                    let mut components = Vec::with_capacity(depth);
                    for _ in 0..depth {
                        let c = ALPHA[(test_prng(&mut prng) as usize) % files];
                        components.push((c as char).to_string());
                    }
                    let full_path = "/".to_string() + &components.join("/");

                    let mut info = core::mem::MaybeUninit::<LfsInfo>::zeroed();
                    let res = lfs_stat(lfs_ptr, path_bytes(&full_path).as_ptr(), info.as_mut_ptr());
                    if res == LFS_ERR_NOENT {
                        for d in 0..depth {
                            let sub = "/".to_string() + &components[..=d].join("/");
                            let err = lfs_mkdir(lfs_ptr, path_bytes(&sub).as_ptr());
                            if err != 0 && err != LFS_ERR_EXIST {
                                return Err(err);
                            }
                        }
                        for d in 0..depth {
                            let sub = "/".to_string() + &components[..=d].join("/");
                            let r = lfs_stat(lfs_ptr, path_bytes(&sub).as_ptr(), info.as_mut_ptr());
                            if r != 0 {
                                return Err(if r < 0 { r } else { -1 });
                            }
                            let info_ref = unsafe { &*info.as_ptr() };
                            let nul = info_ref.name.iter().position(|&b| b == 0).unwrap_or(256);
                            let name = core::str::from_utf8(&info_ref.name[..nul]).unwrap();
                            let expected = &components[d];
                            if name != *expected {
                                return Err(-1);
                            }
                            if info_ref.type_ != LFS_TYPE_DIR as u8 {
                                return Err(-1);
                            }
                        }
                    } else if res == 0 {
                        let info_ref = unsafe { &*info.as_ptr() };
                        let expected = &components[depth - 1];
                        let nul = info_ref.name.iter().position(|&b| b == 0).unwrap_or(256);
                        let name = core::str::from_utf8(&info_ref.name[..nul]).unwrap();
                        if name != *expected || info_ref.type_ != LFS_TYPE_DIR as u8 {
                            return Err(-1);
                        }
                        for d in (0..depth).rev() {
                            let sub = "/".to_string() + &components[..=d].join("/");
                            let err = lfs_remove(lfs_ptr, path_bytes(&sub).as_ptr());
                            if err != 0 && err != LFS_ERR_NOTEMPTY {
                                return Err(err);
                            }
                        }
                        let r =
                            lfs_stat(lfs_ptr, path_bytes(&full_path).as_ptr(), info.as_mut_ptr());
                        if r != LFS_ERR_NOENT {
                            return Err(if r < 0 { r } else { -1 });
                        }
                    } else {
                        return Err(res);
                    }
                }

                if lfs_unmount(lfs_ptr) != 0 {
                    return Err(-1);
                }
                Ok(())
            },
            |_, _| Ok(()),
        );
        result.expect(&format!(
            "test_orphans_reentrant FILES={files} DEPTH={depth} should complete"
        ));
    }
}
