# Phase 8a: Parameterize test_alloc.rs

## Goal

Exact replication of upstream `reference/tests/test_alloc.toml` parameter sets. Every upstream define combination must appear as a Rust test parameterization. If new combinations fail, mark them `#[ignore = "bug: <description>"]` and move on — bug fixes come later.

## Reference

- Upstream TOML: `reference/tests/test_alloc.toml`
- Rust file: `littlefs-rust/tests/test_alloc.rs`
- Top-level guard: `if = 'BLOCK_CYCLES == -1'` (all existing tests already use `block_cycles: -1`)

## Current State

All 12 upstream cases exist as Rust functions, but every one uses plain `#[test]` with a single fixed configuration. None use `#[rstest]`.

Existing Rust-only extras at the bottom: `test_alloc_two_files_ctz`.

## Cases to Parameterize

### test_alloc_parallel

Upstream defines:
```
FILES = 3
SIZE = (((BLOCK_SIZE-8)*(BLOCK_COUNT-6)) / FILES)
GC = [false, true]
COMPACT_THRESH = [-1, 0, BLOCK_SIZE/2]
INFER_BC = [false, true]
```

Add `#[rstest]` with `#[values]` for `gc: bool`, `compact_thresh: i32` (using -1, 0, BLOCK_SIZE/2), `infer_bc: bool`. 12 combinations.

When `infer_bc == true`, set `cfg.block_count = 0` before mount.
When `gc == true`, call `lfs_fs_gc` during write loop.
Set `cfg.compact_thresh` from the parameter.

### test_alloc_serial

Upstream defines: same as parallel.
```
FILES = 3, SIZE = ..., GC = [false, true], COMPACT_THRESH = [-1, 0, BLOCK_SIZE/2], INFER_BC = [false, true]
```
Same 12 combinations. Same approach.

### test_alloc_parallel_reuse

Upstream defines:
```
FILES = 3, SIZE = ..., CYCLES = [1, 10], INFER_BC = [false, true]
```
4 combinations. Add `cycles: u32` and `infer_bc: bool`.

### test_alloc_serial_reuse

Upstream defines: same as parallel_reuse.
```
FILES = 3, SIZE = ..., CYCLES = [1, 10], INFER_BC = [false, true]
```
4 combinations.

### test_alloc_exhaustion

Upstream defines:
```
INFER_BC = [false, true]
```
2 combinations. Add `infer_bc: bool`.

### test_alloc_exhaustion_wraparound

Upstream defines:
```
SIZE = (((BLOCK_SIZE-8)*(BLOCK_COUNT-4)) / 3)
INFER_BC = [false, true]
```
2 combinations. Add `infer_bc: bool`.

### test_alloc_dir_exhaustion

Upstream defines:
```
INFER_BC = [false, true]
```
2 combinations.

### test_alloc_bad_blocks

Upstream defines (fixed, not parameterized):
```
ERASE_CYCLES = 0xffffffff
BADBLOCK_BEHAVIOR = LFS_EMUBD_BADBLOCK_READERROR
```
No parameterization needed — already matches. Verify the comment header is accurate.

### test_alloc_chained_dir_exhaustion

Upstream defines (fixed):
```
if = 'ERASE_SIZE == 512'
ERASE_COUNT = 1024
```
No parameterization — already uses `config_with_geometry(512, 1024)`. Verify comment.

### test_alloc_split_dir

Upstream defines (fixed):
```
if = 'ERASE_SIZE == 512'
ERASE_COUNT = 1024
```
No parameterization — already uses `config_with_geometry(512, 1024)`. Verify comment.

### test_alloc_outdated_lookahead

Upstream defines (fixed):
```
if = 'ERASE_SIZE == 512'
ERASE_COUNT = 1024
```
No parameterization. Verify comment.

### test_alloc_outdated_lookahead_split_dir

Upstream defines (fixed):
```
if = 'ERASE_SIZE == 512'
ERASE_COUNT = 1024
```
No parameterization. Verify comment.

## Implementation Notes

- The `GC` and `COMPACT_THRESH` parameters for parallel/serial require the test body to branch on them. The current Rust tests do not call `lfs_fs_gc` or set `compact_thresh`. These code paths need adding.
- `INFER_BC` means setting `block_count = 0` on a cloned config before mount (not on the format config).
- The `SIZE` define is computed from BLOCK_SIZE and BLOCK_COUNT — keep as computed, not hardcoded.
- Existing Rust extra `test_alloc_two_files_ctz` stays at the bottom, unchanged.

## Process

```
1. Add rstest to imports
2. For each case above with parameterization:
   a. Replace #[test] with #[rstest]
   b. Add #[values] parameters to function signature
   c. Wire parameters into test body (gc, compact_thresh, infer_bc, cycles)
   d. Update upstream comment header with full defines
3. For fixed-config cases: update comment header only
4. cargo test -p littlefs-rust --test test_alloc
5. Mark any new failures: #[ignore = "bug: <description>"]
6. cargo fmt && cargo clippy
```

## Validate

```
cargo test -p littlefs-rust --test test_alloc 2>&1
cargo fmt -p littlefs-rust
cargo clippy -p littlefs-rust
```
