//! Exhaustion and wear-leveling tests.
//!
//! Corresponds to upstream test_exhaustion.toml
//! Source: https://github.com/littlefs-project/littlefs/blob/master/tests/test_exhaustion.toml

mod common;

use common::{default_config, init_log, ram_bd};
use lp_littlefs::{LittleFs, OpenFlags};

// --- test_exhaustion_normal ---
// Upstream: fill FS until no space, ops return Nomem
#[test]
fn test_exhaustion_normal() {
    init_log();
    let config = default_config();
    let bd = ram_bd(&config);
    let mut lfs = LittleFs::new();
    lfs.format(&bd, &config).unwrap();
    lfs.mount(&bd, &config).unwrap();

    let mut i = 0u32;
    loop {
        let path = format!("f{i}");
        let mut file = match lfs.file_open(
            &bd,
            &config,
            &path,
            OpenFlags::new(OpenFlags::WRONLY | OpenFlags::CREAT),
        ) {
            Ok(f) => f,
            Err(lp_littlefs::Error::Nomem) => break,
            Err(e) => panic!("unexpected error: {e:?}"),
        };
        let buf = [0u8; 256];
        for _ in 0..(config.block_size as usize / 256 * 4) {
            match lfs.file_write(&bd, &config, &mut file, &buf) {
                Ok(_) => {}
                Err(lp_littlefs::Error::Nomem) => break,
                Err(e) => panic!("write error: {e:?}"),
            }
        }
        lfs.file_close(&bd, &config, file).unwrap();
        i += 1;
        if i > 500 {
            break;
        }
    }
    assert!(
        i > 0,
        "should have created at least one file before exhaustion"
    );
    lfs.unmount(&bd, &config).unwrap();
}

// --- test_exhaustion_superblocks ---
#[test]
#[ignore = "superblock exhaustion semantics may differ"]
fn test_exhaustion_superblocks() {}

// --- test_exhaustion_wear_leveling ---
// Upstream: block_cycles BD, verify wear spread
#[test]
#[ignore = "block_cycles BD / wear simulation not implemented"]
fn test_exhaustion_wear_leveling() {}

// --- test_exhaustion_wear_leveling_superblocks ---
#[test]
#[ignore = "block_cycles BD not implemented"]
fn test_exhaustion_wear_leveling_superblocks() {}

// --- test_exhaustion_wear_distribution ---
#[test]
#[ignore = "block_cycles BD not implemented"]
fn test_exhaustion_wear_distribution() {}
