//! Orphan and power-loss consistency tests.
//!
//! Per upstream test_orphans.toml, test_powerloss.toml.
//! Phase 06: mkconsistent persists gstate; remount without orphans.

use lp_littlefs::{BlockDevice, CachedBlockDevice, Config, LittleFs, RamBlockDevice};

fn default_config() -> Config {
    Config::default_for_tests(128)
}

#[allow(dead_code)]
fn cached_bd(config: &Config) -> CachedBlockDevice<RamBlockDevice> {
    let ram = RamBlockDevice::new(config.block_size, config.block_count);
    CachedBlockDevice::new(ram, config).unwrap()
}

fn uncached_bd(config: &Config) -> RamBlockDevice {
    RamBlockDevice::new(config.block_size, config.block_count)
}

// --- test_orphans_mkconsistent_no_orphans ---
// Upstream: preporphans +1, commit (persist), unmount; remount, mkconsistent, remount.
// Persistence now happens via normal commit path (MOVESTATE in dir_commit).
// Use a no-op mkdir+remove to trigger a commit that persists gstate.
#[test]
fn test_orphans_mkconsistent_no_orphans() {
    let _ = env_logger::builder().is_test(true).try_init();
    let config = default_config();
    let bd = uncached_bd(&config);
    let mut lfs = LittleFs::new();
    lfs.format(&bd, &config).unwrap();
    lfs.mount(&bd, &config).unwrap();

    lfs.fs_preporphans(1).unwrap();
    assert!(lfs.fs_has_orphans(&bd, &config).unwrap());
    lfs.mkdir(&bd, &config, "_p").unwrap();
    lfs.remove(&bd, &config, "_p").unwrap();
    lfs.unmount().unwrap();

    lfs.mount(&bd, &config).unwrap();
    assert!(lfs.fs_has_orphans(&bd, &config).unwrap());
    lfs.fs_mkconsistent(&bd, &config).unwrap();
    assert!(
        !lfs.fs_has_orphans(&bd, &config).unwrap(),
        "after mkconsistent, gstate should have no orphans (same mount)"
    );
    lfs.unmount().unwrap();

    bd.sync().unwrap();
    lfs.mount(&bd, &config).unwrap();
    assert!(
        !lfs.fs_has_orphans(&bd, &config).unwrap(),
        "after remount, gstate persisted to disk should have no orphans"
    );
    lfs.unmount().unwrap();
}
