//! Orphan and power-loss consistency tests.
//!
//! Per upstream test_orphans.toml, test_powerloss.toml.
//! Phase 06: mkconsistent persists gstate; remount without orphans.

mod common;

use common::{default_config, init_log, uncached_bd};
use lp_littlefs::{BlockDevice, LittleFs};

// --- test_orphans_mkconsistent_no_orphans ---
// With lazy force_consistency, mkdir/remove run deorphan first. So preporphans(1)
// gets cleared before the commit. We verify: mkconsistent clears (no-op here)
// and persists; remount shows no orphans.
#[test]
fn test_orphans_mkconsistent_no_orphans() {
    init_log();
    let config = default_config();
    let bd = uncached_bd(&config);
    let mut lfs = LittleFs::new();
    lfs.format(&bd, &config).unwrap();
    lfs.mount(&bd, &config).unwrap();

    lfs.fs_preporphans(1).unwrap();
    assert!(lfs.fs_has_orphans(&bd, &config).unwrap());
    // mkdir runs force_consistency first, which clears orphan count
    lfs.mkdir(&bd, &config, "_p").unwrap();
    lfs.remove(&bd, &config, "_p").unwrap();
    assert!(
        !lfs.fs_has_orphans(&bd, &config).unwrap(),
        "force_consistency before mkdir clears orphans"
    );
    lfs.unmount(&bd, &config).unwrap();

    lfs.mount(&bd, &config).unwrap();
    assert!(
        !lfs.fs_has_orphans(&bd, &config).unwrap(),
        "persisted gstate has no orphans (cleared before commit)"
    );
    lfs.fs_mkconsistent(&bd, &config).unwrap();
    assert!(
        !lfs.fs_has_orphans(&bd, &config).unwrap(),
        "after mkconsistent, gstate should have no orphans (same mount)"
    );
    lfs.unmount(&bd, &config).unwrap();

    bd.sync().unwrap();
    lfs.mount(&bd, &config).unwrap();
    assert!(
        !lfs.fs_has_orphans(&bd, &config).unwrap(),
        "after remount, gstate persisted to disk should have no orphans"
    );
    lfs.unmount(&bd, &config).unwrap();
}

// --- test_orphans_normal ---
// Upstream: create parent/orphan, parent/child, remove orphan, corrupt block to orphan it
#[test]
#[ignore = "block-level corruption simulation not implemented"]
fn test_orphans_normal() {}

// --- test_orphans_no_orphans ---
// Upstream: preporphans(+1), commit, mount, force_consistency clears
#[test]
fn test_orphans_no_orphans() {
    init_log();
    let config = default_config();
    let bd = uncached_bd(&config);
    let mut lfs = LittleFs::new();
    lfs.format(&bd, &config).unwrap();
    lfs.mount(&bd, &config).unwrap();
    lfs.fs_preporphans(1).unwrap();
    assert!(lfs.fs_has_orphans(&bd, &config).unwrap());
    lfs.mkdir(&bd, &config, "_x").unwrap();
    lfs.remove(&bd, &config, "_x").unwrap();
    assert!(!lfs.fs_has_orphans(&bd, &config).unwrap());
    lfs.unmount(&bd, &config).unwrap();
}

// --- test_orphans_one_orphan ---
// Upstream: alloc dir, preporphans, commit, mount, force_consistency deorphans
#[test]
#[ignore = "low-level dir_alloc/preporphans/commit sequence not exposed"]
fn test_orphans_one_orphan() {}

// --- test_orphans_mkconsistent_one_orphan ---
// Upstream: one orphan, mkconsistent persists cleared gstate
#[test]
#[ignore = "low-level orphan creation not exposed"]
fn test_orphans_mkconsistent_one_orphan() {}

// --- test_orphans_reentrant ---
// Upstream: orphan creation with powerloss
#[test]
#[ignore = "powerloss runner not implemented"]
fn test_orphans_reentrant() {}

// --- test_orphans_nonreentrant ---
// Upstream: orphan operations without powerloss
#[test]
fn test_orphans_nonreentrant() {
    init_log();
    let config = default_config();
    let bd = uncached_bd(&config);
    let mut lfs = LittleFs::new();
    lfs.format(&bd, &config).unwrap();
    lfs.mount(&bd, &config).unwrap();
    lfs.mkdir(&bd, &config, "a").unwrap();
    lfs.mkdir(&bd, &config, "b").unwrap();
    lfs.remove(&bd, &config, "a").unwrap();
    lfs.remove(&bd, &config, "b").unwrap();
    assert!(!lfs.fs_has_orphans(&bd, &config).unwrap());
    lfs.unmount(&bd, &config).unwrap();
}
