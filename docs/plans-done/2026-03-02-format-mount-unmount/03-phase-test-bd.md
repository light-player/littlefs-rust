# Phase 3: Block device tests (test_bd)

## Scope of phase

Create `tests/test_bd.rs` corresponding to upstream `test_bd.toml`. Implement `test_bd_one_block` to validate RamBlockDevice erase/prog/read behavior. Structure tests for easy geometry changes.

## Code organization reminders

- Test helpers at bottom of test module
- Clear test names; one assertion focus per test
- Header comment linking to upstream GitHub source

## Implementation details

### 1. Create tests/test_bd.rs

```rust
//! Block device tests.
//!
//! Corresponds to upstream test_bd.toml
//! Source: https://github.com/littlefs-project/littlefs/blob/master/tests/test_bd.toml
//!
//! These tests validate the block device abstraction, not littlefs itself.

use lp_littlefs::{BlockDevice, RamBlockDevice};

/// Default test geometry. Easy to change for different upstream geometries.
fn default_geometry() -> (u32, u32) {
    let block_size = 512;
    let block_count = 128;
    (block_size, block_count)
}

// --- test_bd_one_block ---
// Upstream: erase block 0, prog in chunks, read back and verify.
// Uses (block_index + offset + j) % 251 to avoid powers-of-two aliasing.
#[test]
fn test_bd_one_block() {
    let (block_size, block_count) = default_geometry();
    let bd = RamBlockDevice::new(block_size, block_count);

    let read_size = 16u32;
    let prog_size = 16u32;
    let mut buffer = vec![0u8; read_size.max(prog_size) as usize];

    // Erase block 0
    bd.erase(0).unwrap();

    // Prog in chunks
    for i in (0..block_size).step_by(prog_size as usize) {
        for j in 0..prog_size {
            buffer[j as usize] = ((i + j) % 251) as u8;
        }
        bd.prog(0, i, &buffer[..prog_size as usize]).unwrap();
    }

    // Read back in chunks
    for i in (0..block_size).step_by(read_size as usize) {
        bd.read(0, i, &mut buffer[..read_size as usize]).unwrap();
        for j in 0..read_size {
            assert_eq!(
                buffer[j as usize],
                ((i + j) % 251) as u8,
                "offset {i}, byte {j}"
            );
        }
    }
}
```

Note: Upstream test_bd_one_block has `defines.READ = ['READ_SIZE', 'BLOCK_SIZE']` and `defines.PROG = ['PROG_SIZE', 'BLOCK_SIZE']` — two permutations each. For phase 3 we use one (READ_SIZE=16, PROG_SIZE=16) to match default geometry. We can add parameterized tests later.

### 2. Integration test setup

Integration tests in `tests/` have access to `std` by default. We use `vec!` which needs alloc; lp_littlefs has alloc feature. Ensure Cargo.toml has:

```toml
[dev-dependencies]
# none needed - alloc is in default features
```

The test uses `lp_littlefs::RamBlockDevice` — the crate must expose it. Phase 2 already added that.

## Validate

```bash
cd lp-littlefs && cargo test test_bd_one_block
```
