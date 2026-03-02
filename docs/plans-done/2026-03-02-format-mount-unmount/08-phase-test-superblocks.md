# Phase 8: Superblock tests (test_superblocks)

## Scope of phase

Create `tests/test_superblocks.rs` with format, mount, magic, invalid_mount tests. Link to upstream test_superblocks.toml. Each test documents the corresponding upstream case.

## Code organization reminders

- Header comment with GitHub source link
- Geometry helper for easy changes
- One test per upstream case; clear names

## Implementation details

### 1. Create tests/test_superblocks.rs

```rust
//! Superblock and format/mount tests.
//!
//! Corresponds to upstream test_superblocks.toml
//! Source: https://github.com/littlefs-project/littlefs/blob/master/tests/test_superblocks.toml

use lp_littlefs::{BlockDevice, Config, LittleFs, RamBlockDevice, MAGIC, MAGIC_OFFSET};

fn default_config() -> Config {
    Config::default_for_tests(128)
}

// --- test_superblocks_format ---
// Upstream: lfs_format(&lfs, cfg) => 0
#[test]
fn test_superblocks_format() {
    let config = default_config();
    let bd = RamBlockDevice::new(config.block_size, config.block_count);
    let mut lfs = LittleFs::new();
    lfs.format(&bd, &config).unwrap();
}

// --- test_superblocks_mount ---
// Upstream: format, mount, unmount
#[test]
fn test_superblocks_mount() {
    let config = default_config();
    let bd = RamBlockDevice::new(config.block_size, config.block_count);
    let mut lfs = LittleFs::new();
    lfs.format(&bd, &config).unwrap();
    lfs.mount(&bd, &config).unwrap();
    lfs.unmount().unwrap();
}

// --- test_superblocks_magic ---
// Upstream: format, then raw read to verify "littlefs" at offset 8 in both blocks
#[test]
fn test_superblocks_magic() {
    let config = default_config();
    let bd = RamBlockDevice::new(config.block_size, config.block_count);
    let mut lfs = LittleFs::new();
    lfs.format(&bd, &config).unwrap();

    let mut buf = [0u8; 16];
    bd.read(0, 0, &mut buf).unwrap();
    assert_eq!(&buf[MAGIC_OFFSET as usize..][..8], MAGIC);
    bd.read(1, 0, &mut buf).unwrap();
    assert_eq!(&buf[MAGIC_OFFSET as usize..][..8], MAGIC);
}

// --- test_superblocks_invalid_mount ---
// Upstream: mount on blank device => LFS_ERR_CORRUPT
#[test]
fn test_superblocks_invalid_mount() {
    let config = default_config();
    let bd = RamBlockDevice::new(config.block_size, config.block_count);
    let mut lfs = LittleFs::new();
    let err = lfs.mount(&bd, &config).unwrap_err();
    assert_eq!(err, lp_littlefs::Error::Corrupt);
}
```

Note: test_superblocks_magic—upstream says "if we lose power we may not have the magic string in both blocks". For our format we might only write to one block initially. Adjust test or format so both blocks have magic if needed for this test. Upstream format does two commits (first with superblock, second "compaction") and writes to both blocks of the pair. For minimal format we may write to block 0 only. The magic test expects both—we can either (a) have format write to both blocks, or (b) relax the test to only check block 0 for now. Design doc says blocks 0,1 are the metadata pair; typically both get the same commit. Let's have format write to both blocks to match upstream behavior.

### 2. Exports

Ensure MAGIC, MAGIC_OFFSET are pub in lib.rs.

## Validate

```bash
cd lp-littlefs && cargo test
```

All tests (test_bd + test_superblocks) should pass.
