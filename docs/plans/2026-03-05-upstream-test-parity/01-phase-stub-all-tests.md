# Phase 1: Stub All Missing Tests

## Scope

Create skeleton files for all 8 missing test files and add stubs for all missing cases in existing files. Every upstream C case gets a Rust function with `#[ignore = "stub"]` and `todo!()`. Parameterized tests include the `#[rstest]` + `#[values]` skeleton.

This makes the full gap visible via `cargo test -- --list` and `cargo test -- --ignored`.

Also: move Rust-specific extras to a clearly marked section at the bottom of each existing file.

## Code Organization Reminders

- Place upstream cases first, extras at the bottom
- Include the upstream comment header on every test
- Keep related functionality grouped together
- Any temporary code should have a TODO comment

## New Test Files to Create

### test_seek.rs (10 cases)

```rust
//! Upstream: tests/test_seek.toml
mod common;
use rstest::rstest;

/// Upstream: [cases.test_seek_read]
/// defines = [{COUNT=132, SKIP=4}, {COUNT=132, SKIP=128}, {COUNT=200, SKIP=10},
///            {COUNT=200, SKIP=100}, {COUNT=4, SKIP=1}, {COUNT=4, SKIP=2}]
///
/// Write COUNT copies of "kittycatcat", seek to SKIP, verify read at various positions.
#[rstest]
#[case(132, 4)]
#[case(132, 128)]
#[case(200, 10)]
#[case(200, 100)]
#[case(4, 1)]
#[case(4, 2)]
#[ignore = "stub"]
fn test_seek_read(#[case] count: u32, #[case] skip: u32) { todo!() }
```

Apply this pattern for all 10 cases. Cases with no parameterization use plain `#[test]`.

### test_truncate.rs (7 cases)

Cases: `test_truncate_simple`, `test_truncate_read`, `test_truncate_write_read`, `test_truncate_write`, `test_truncate_reentrant_write`, `test_truncate_aggressive`, `test_truncate_nop`.

For `simple`/`read`/`write`:
```rust
/// Upstream: [cases.test_truncate_simple]
/// defines.MEDIUMSIZE = [31, 32, 33, 511, 512, 513, 2047, 2048, 2049]
/// defines.LARGESIZE = [32, 33, 512, 513, 2048, 2049, 8192, 8193]
/// if = 'MEDIUMSIZE < LARGESIZE'
///
/// Write LARGESIZE "hair", truncate to MEDIUMSIZE, remount, verify.
#[rstest]
#[ignore = "stub"]
fn test_truncate_simple(
    #[values(31, 32, 33, 511, 512, 513, 2047, 2048, 2049)] medium: u32,
    #[values(32, 33, 512, 513, 2048, 2049, 8192, 8193)] large: u32,
) { todo!() }
```

### test_interspersed.rs (4 cases)

Cases: `test_interspersed_files`, `test_interspersed_remove_files`, `test_interspersed_remove_inconveniently`, `test_interspersed_reentrant_files`.

### test_badblocks.rs (4 cases)

Cases: `test_badblocks_single`, `test_badblocks_region_corruption`, `test_badblocks_alternating_corruption`, `test_badblocks_superblocks`.

All share: `ERASE_VALUE = [0x00, 0xff, -1]`, `BADBLOCK_BEHAVIOR = [PROGERROR, ERASEERROR, READERROR, PROGNOOP, ERASENOOP]`.

### test_evil.rs (8 cases)

Cases: `test_evil_invalid_tail_pointer`, `test_evil_invalid_dir_pointer`, `test_evil_invalid_file_pointer`, `test_evil_invalid_ctz_pointer`, `test_evil_invalid_gstate_pointer`, `test_evil_mdir_loop`, `test_evil_mdir_loop2`, `test_evil_mdir_loop_child`.

### test_exhaustion.rs (5 cases)

Cases: `test_exhaustion_normal`, `test_exhaustion_superblocks`, `test_exhaustion_wear_leveling`, `test_exhaustion_wear_leveling_superblocks`, `test_exhaustion_wear_distribution`.

### test_shrink.rs (2 cases)

Cases: `test_shrink_simple`, `test_shrink_full`.

### test_compat.rs (17 cases)

All stubbed with `#[ignore = "stub: requires version compat infrastructure"]`. Cases: `test_compat_forward_mount`, `test_compat_forward_read_dirs`, `test_compat_forward_read_files`, `test_compat_forward_read_files_in_dirs`, `test_compat_forward_write_dirs`, `test_compat_forward_write_files`, `test_compat_forward_write_files_in_dirs`, `test_compat_backward_mount`, `test_compat_backward_read_dirs`, `test_compat_backward_read_files`, `test_compat_backward_read_files_in_dirs`, `test_compat_backward_write_dirs`, `test_compat_backward_write_files`, `test_compat_backward_write_files_in_dirs`, `test_compat_major_incompat`, `test_compat_minor_incompat`, `test_compat_minor_bump`.

## Stubs to Add to Existing Files

### test_files.rs — 6 missing upstream cases

- `test_files_large` — `#[rstest]` with SIZE, CHUNKSIZE, INLINE_MAX values
- `test_files_rewrite` — `#[rstest]` with SIZE1, SIZE2, CHUNKSIZE, INLINE_MAX
- `test_files_reentrant_write` — `#[rstest]` with SIZE, CHUNKSIZE, INLINE_MAX
- `test_files_reentrant_write_sync` — complex define set
- `test_files_many_power_cycle` — N=300
- `test_files_many_power_loss` — N=300, POWERLOSS_BEHAVIOR

Also: replace existing `test_files_append` and `test_files_truncate` with stubbed parameterized versions matching upstream defines. Move `test_files_same_session`, `test_files_simple_read`, `test_files_seek_tell`, `test_files_truncate_api` to an extras section.

### test_dirs.rs — 12 missing upstream cases

- `test_dirs_many_rename_append`, `test_dirs_many_reentrant`, `test_dirs_file_creation`, `test_dirs_file_removal`, `test_dirs_file_rename`, `test_dirs_file_reentrant`, `test_dirs_nested`, `test_dirs_recursive_remove`, `test_dirs_remove_read`, `test_dirs_other_errors`, `test_dirs_seek`, `test_dirs_toot_seek`

### test_superblocks.rs — 13 missing upstream cases

- `test_superblocks_mount_unknown_block_count`, `test_superblocks_reentrant_format`, `test_superblocks_stat_tweaked`, `test_superblocks_expand`, `test_superblocks_magic_expand`, `test_superblocks_expand_power_cycle`, `test_superblocks_reentrant_expand`, `test_superblocks_unknown_blocks`, `test_superblocks_fewer_blocks`, `test_superblocks_more_blocks`, `test_superblocks_grow`, `test_superblocks_shrink`, `test_superblocks_metadata_max`

### test_move.rs — 2 missing cases

- `test_move_fix_relocation`, `test_move_fix_relocation_predecessor`

### test_orphans.rs — 4 missing cases

- `test_orphans_normal`, `test_orphans_one_orphan`, `test_orphans_mkconsistent_one_orphan`, `test_orphans_reentrant`

### test_paths.rs — 7 missing cases

- `test_paths_noent_trailing_slashes`, `test_paths_noent_trailing_dots`, `test_paths_noent_trailing_dotdots`, `test_paths_utf8_ipa`, `test_paths_oopsallspaces`, `test_paths_oopsalldels`, `test_paths_oopsallffs`

### test_powerloss.rs — 1 missing case

- `test_powerloss_partial_prog`

## Validate

```
cargo test -p lp-littlefs -- --list 2>&1 | grep -c "stub"
# Should show the total count of stubbed tests

cargo test -p lp-littlefs 2>&1
# All non-ignored tests should still pass

cargo fmt -p lp-littlefs
cargo clippy -p lp-littlefs
```
