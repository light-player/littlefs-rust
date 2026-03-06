# Phase 8c: Parameterize test_superblocks.rs

## Goal

Exact replication of upstream `reference/tests/test_superblocks.toml` parameter sets. Every upstream define combination must appear as a Rust test parameterization. If new combinations fail, mark them `#[ignore = "bug: <description>"]` and move on.

## Reference

- Upstream TOML: `reference/tests/test_superblocks.toml`
- Rust file: `lp-littlefs/tests/test_superblocks.rs`

## Current State

All upstream cases exist as Rust functions. Several already iterate over the correct parameter values using inner `for` loops (expand, magic_expand, expand_power_cycle, reentrant_expand, fewer_blocks). Others have partial or missing parameterization. None use `#[rstest]`.

Existing Rust-only extras: `test_traverse_attrs_callback_order`, `test_traverse_filter_gets_superblock_after_push`.

## Cases to Parameterize

### test_superblocks_format

Upstream: no defines. No change.

### test_superblocks_mount

Upstream: no defines. No change.

### test_superblocks_magic

Upstream: no defines. No change.

### test_superblocks_mount_unknown_block_count

Upstream: no defines. No change. Already implemented.

### test_superblocks_reentrant_format

Upstream:
```
reentrant = true
POWERLOSS_BEHAVIOR = [LFS_EMUBD_POWERLOSS_NOOP, LFS_EMUBD_POWERLOSS_OOO]
```

Already implemented with powerloss. Verify POWERLOSS_BEHAVIOR coverage matches.

### test_superblocks_invalid_mount

Upstream: no defines. No change.

### test_superblocks_stat

Upstream:
```
if = 'DISK_VERSION == 0'
```
No parameterized defines. Already implemented. Verify `if` guard comment.

### test_superblocks_stat_tweaked

Upstream:
```
if = 'DISK_VERSION == 0'
TWEAKED_NAME_MAX = 63
TWEAKED_FILE_MAX = (1 << 16)-1
TWEAKED_ATTR_MAX = 512
```
Fixed defines, no parameterization. Already implemented.

### test_superblocks_expand

Upstream:
```
BLOCK_CYCLES = [32, 33, 1]
N = [10, 100, 1000]
```
9 combinations.

Current Rust: `for &block_cycles in &[32, 33, 1]` + `for &n in &[10, 100, 1000]` — already iterates all 9 combos.

Convert inner loops to `#[rstest]` with `#[values]` for consistency. Or leave as-is since coverage matches.

### test_superblocks_magic_expand

Upstream: same as expand (`BLOCK_CYCLES = [32, 33, 1]`, `N = [10, 100, 1000]`).

Current Rust: same nested loops. Already matches.

### test_superblocks_expand_power_cycle

Upstream: same as expand.

Current Rust: same nested loops. Already matches.

### test_superblocks_reentrant_expand

Upstream:
```
BLOCK_CYCLES = [2, 1]
N = 24
reentrant = true
POWERLOSS_BEHAVIOR = [NOOP, OOO]
```

Current Rust: `for &block_cycles in &[2, 1]` — iterates both. N=24 is fixed. Already matches BLOCK_CYCLES. POWERLOSS_BEHAVIOR coverage depends on powerloss infra.

### test_superblocks_unknown_blocks

Upstream: no defines. No change. Already implemented.

### test_superblocks_fewer_blocks

Upstream:
```
BLOCK_COUNT = [ERASE_COUNT/2, ERASE_COUNT/4, 2]
```

Current Rust: `for &block_count in &[ERASE_COUNT / 2, ERASE_COUNT / 4, 2]` with `#[ignore]`. Already matches.

### test_superblocks_more_blocks

Upstream:
```
FORMAT_BLOCK_COUNT = 2*ERASE_COUNT
in = 'lfs.c'
```
No parameterization. Fixed define. Already implemented (uses internal API). The `in = 'lfs.c'` means it accesses internals.

### test_superblocks_grow

Upstream:
```
BLOCK_COUNT = [ERASE_COUNT/2, ERASE_COUNT/4, 2]
BLOCK_COUNT_2 = ERASE_COUNT
KNOWN_BLOCK_COUNT = [true, false]
```
6 combinations (3 BLOCK_COUNT × 2 KNOWN_BLOCK_COUNT).

Current Rust: tests only one fixed configuration. **Needs expansion** to cover all 6 combinations.

Convert to `#[rstest]` with `#[values]` for `block_count_frac` and `known_block_count`.

### test_superblocks_shrink

Upstream:
```
BLOCK_COUNT = ERASE_COUNT
BLOCK_COUNT_2 = [ERASE_COUNT/2, ERASE_COUNT/4, 2]
KNOWN_BLOCK_COUNT = [true, false]
```
6 combinations. Guarded by `#ifdef LFS_SHRINKNONRELOCATING`.

Current Rust: `#[ignore = "requires LFS_SHRINKNONRELOCATING feature"]` with `todo!()`. Leave as-is — this is a feature gate, not a parameterization issue.

### test_superblocks_metadata_max

Upstream:
```
METADATA_MAX = [lfs_max(512, PROG_SIZE), lfs_max(BLOCK_SIZE/2, PROG_SIZE), BLOCK_SIZE]
N = [10, 100, 1000]
```
9 combinations.

Current Rust: `#[ignore = "requires metadata_max in config during compaction cycles"]` with `todo!()`. Needs implementation + parameterization. If metadata_max is not yet wired, leave ignored but add the parameterized skeleton.

## Summary of Actual Work

| Case | Status | Action |
|------|--------|--------|
| format, mount, magic, invalid_mount | no defines | none |
| mount_unknown_block_count | no defines | none |
| reentrant_format | matches | verify POWERLOSS_BEHAVIOR |
| stat, stat_tweaked | no defines / fixed | none |
| expand, magic_expand, expand_power_cycle | matches [32,33,1]×[10,100,1000] | optional rstest conversion |
| reentrant_expand | matches [2,1] × N=24 | none |
| unknown_blocks | no defines | none |
| fewer_blocks | matches [E/2, E/4, 2] | none (still ignored) |
| more_blocks | fixed | none |
| **grow** | **1 config vs 6 combos** | **expand to BLOCK_COUNT × KNOWN_BLOCK_COUNT** |
| shrink | feature-gated | leave ignored |
| **metadata_max** | **todo!() stub** | **add parameterized skeleton, keep ignored** |

## Implementation Notes

- The main real work is `test_superblocks_grow` — it needs the 3 BLOCK_COUNT fractions × 2 KNOWN_BLOCK_COUNT booleans.
- For `metadata_max`, add the `#[rstest]` skeleton with the 9 combinations but keep `#[ignore]` until the config wiring exists.
- The expand/magic_expand/expand_power_cycle tests already iterate correctly via nested `for` loops. Converting to `#[rstest]` is optional — it gives better test output but the coverage is already correct.
- `ERASE_COUNT` in the Rust tests corresponds to `block_count` passed to `default_config()` (the backing storage size).

## Process

```
1. test_superblocks_grow: convert to #[rstest] with BLOCK_COUNT and KNOWN_BLOCK_COUNT parameters
2. test_superblocks_metadata_max: add #[rstest] skeleton with METADATA_MAX and N parameters, keep #[ignore]
3. Optionally convert expand/magic_expand/expand_power_cycle inner loops to #[rstest]
4. Update upstream comment headers on all cases
5. cargo test -p lp-littlefs --test test_superblocks
6. Mark any new failures: #[ignore = "bug: <description>"]
7. cargo fmt && cargo clippy
```

## Validate

```
cargo test -p lp-littlefs --test test_superblocks 2>&1
cargo fmt -p lp-littlefs
cargo clippy -p lp-littlefs
```
