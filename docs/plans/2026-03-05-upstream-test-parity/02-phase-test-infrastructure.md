# Phase 2: Test Infrastructure

## Scope

Add shared test infrastructure to `common/mod.rs` needed by subsequent phases. No test implementations yet — just helpers.

## Code Organization Reminders

- Place more abstract things, entry points, and tests first
- Place helper utility functions at the bottom of files
- Keep related functionality grouped together
- Any temporary code should have a TODO comment

## Components to Add

### 1. test_prng — xorshift32 PRNG

Port the C `TEST_PRNG` exactly. Must produce identical sequences for cross-validation with C.

```rust
/// xorshift32 PRNG matching C littlefs TEST_PRNG (reference/runners/test_runner.c:568).
/// Deterministic; same seed produces same sequence as C.
pub fn test_prng(state: &mut u32) -> u32 {
    let mut x = *state;
    x ^= x << 13;
    x ^= x >> 17;
    x ^= x << 5;
    *state = x;
    x
}
```

### 2. write_prng_file / verify_prng_file

Chunked write/read with PRNG data. Matches the C pattern used in `test_files_large`, `test_files_rewrite`, `test_truncate`, etc.

```rust
/// Write `size` bytes of PRNG data to an open file in `chunk_size` chunks.
/// PRNG seeded with `seed`. Returns total bytes written.
pub fn write_prng_file(
    lfs: *mut Lfs, file: *mut LfsFile,
    size: u32, chunk_size: u32, seed: u32,
) -> u32 { ... }

/// Read `size` bytes from an open file in `chunk_size` chunks and verify
/// against the same PRNG sequence (seeded with `seed`). Panics on mismatch.
pub fn verify_prng_file(
    lfs: *mut Lfs, file: *mut LfsFile,
    size: u32, chunk_size: u32, seed: u32,
) { ... }

/// Advance PRNG state by `n` bytes (call test_prng n times, discard results).
pub fn advance_prng(state: &mut u32, n: u32) { ... }
```

Buffer size: 1024 bytes (matching C `uint8_t buffer[1024]`).

### 3. config_with_inline_max

C tests parameterize `INLINE_MAX` as `[0, -1, 8]` where `-1` means "use default" (don't set the field).

```rust
/// Build a TestEnv with the given block_count and inline_max.
/// inline_max = -1 means use the library default (don't set it).
pub fn config_with_inline_max(block_count: u32, inline_max: i32) -> TestEnv { ... }
```

This requires checking whether `LfsConfig` has an `inline_max` field. If not, document the gap.

### 4. write_block_raw

For `test_evil` corruption injection. Mirrors existing `read_block_raw`.

```rust
/// Write raw bytes to a block at the given offset, bypassing the FS.
/// Uses the BD prog callback directly.
pub fn write_block_raw(config: &LfsConfig, block: u32, off: u32, data: &[u8]) { ... }
```

### 5. BadBlockBehavior enum

Extend `BadBlockRamStorage` to support all 5 upstream behaviors:

```rust
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum BadBlockBehavior {
    ProgError,   // prog returns LFS_ERR_CORRUPT
    EraseError,  // erase returns LFS_ERR_CORRUPT
    ReadError,   // read returns LFS_ERR_CORRUPT (current behavior)
    ProgNoop,    // prog silently does nothing
    EraseNoop,   // erase silently does nothing
}
```

Update `BadBlockRamStorage` to store a `BadBlockBehavior` and apply it in the appropriate callback (`bd_read`, `bd_prog`, `bd_erase`).

Update `config_badblock` to accept a `BadBlockBehavior` parameter (default `ReadError` for backward compat).

### 6. WearLevelingBd

For `test_exhaustion`. Wraps `RamStorage` with per-block erase-cycle tracking.

```rust
pub struct WearLevelingBd {
    pub ram: RamStorage,
    pub erase_cycles: u32,        // max cycles per block (0xffffffff = unlimited)
    pub wear: Vec<u32>,           // per-block erase count
    pub block_count: u32,
}

impl WearLevelingBd {
    pub fn new(block_count: u32, block_size: u32, erase_cycles: u32) -> Self { ... }

    /// Returns true if block has exceeded its erase cycle limit.
    pub fn is_worn(&self, block: u32) -> bool { ... }

    /// Set a specific block's wear count (for test setup).
    pub fn set_wear(&mut self, block: u32, cycles: u32) { ... }
}
```

BD callbacks: `erase` increments `wear[block]` and returns `LFS_ERR_CORRUPT` if worn out. `prog` returns `LFS_ERR_CORRUPT` if worn. `read` returns `LFS_ERR_CORRUPT` if worn.

Also needs `config_with_wear_leveling(block_count, erase_cycles)` helper and corresponding `WearLevelingEnv`.

## Validate

```
cargo test -p lp-littlefs 2>&1
# All non-ignored tests still pass

cargo fmt -p lp-littlefs
cargo clippy -p lp-littlefs
```

Write a few unit tests for the new infrastructure:

- `test_prng_matches_c` — verify first 10 values of `test_prng(1)` match the C sequence
- `test_badblock_behavior_prog_error` — verify ProgError triggers on prog
- `test_wear_leveling_bd_exhaustion` — verify erase fails after cycle limit
