# Phase 1: Slow Test Feature Flag

## Scope

Add a `slow_tests` feature flag to `lp-littlefs`. Replace `#[ignore = "slow: ..."]` on all power-loss/reentrant and long-running tests with `#[cfg(feature = "slow_tests")]` so they compile and run only when the feature is active.

## Motivation

25 tests are `#[ignore]` purely because they're slow. They're fully implemented and pass when run. Currently they occupy `--ignored` output alongside genuinely broken/stubbed tests, making it hard to see the real gap. A feature flag:

- Separates "slow but working" from "actually broken"
- Lets CI run `cargo test -p lp-littlefs --features slow_tests` in a nightly lane
- Removes noise from `cargo test -- --ignored` (only genuinely blocked tests remain)

## Changes

### 1. Cargo.toml

Add to `[features]`:

```toml
slow_tests = []  # Enable slow power-loss/reentrant tests (CI nightly)
```

### 2. Test files — replace `#[ignore]` with `#[cfg]`

For each test currently ignored for slowness, replace:

```rust
#[test]
#[ignore = "slow: power-loss iteration"]
fn test_foo() {
```

with:

```rust
#[test]
#[cfg(feature = "slow_tests")]
fn test_foo() {
```

For `#[rstest]` parameterized tests:

```rust
#[rstest]
#[case(6, 1, 2000)]
#[case(26, 1, 2000)]
#[case(3, 3, 2000)]
#[ignore = "slow: CYCLES=2000"]
fn test_relocations_nonreentrant(
```

becomes:

```rust
#[rstest]
#[case(6, 1, 2000)]
#[case(26, 1, 2000)]
#[case(3, 3, 2000)]
#[cfg(feature = "slow_tests")]
fn test_relocations_nonreentrant(
```

### 3. Affected tests

| File | Test | Current ignore reason |
|------|------|-----------------------|
| test_dirs.rs | `test_dirs_many_reentrant` | slow: power-loss iteration |
| test_dirs.rs | `test_dirs_file_reentrant` | slow: power-loss iteration |
| test_files.rs | `test_files_many_power_loss` | slow: 300 files x power-loss iteration |
| test_interspersed.rs | `test_interspersed_reentrant_files` | power-loss |
| test_orphans.rs | `test_orphans_reentrant` | slow: power-loss iteration |
| test_relocations.rs | `test_relocations_nonreentrant` (3) | slow: CYCLES=2000 |
| test_relocations.rs | `test_relocations_nonreentrant_renames` (3) | slow |
| test_relocations.rs | `test_relocations_reentrant` (3) | slow: power-loss iteration |
| test_relocations.rs | `test_relocations_reentrant_renames` (3) | slow: power-loss iteration |
| test_seek.rs | `test_seek_reentrant_write` (3) | slow: power-loss iteration |
| test_superblocks.rs | `test_superblocks_reentrant_format` | slow: power-loss iteration |
| test_superblocks.rs | `test_superblocks_reentrant_expand` | slow: power-loss iteration |
| test_truncate.rs | `test_truncate_reentrant_write` (3) | slow: power-loss iteration |

## Validate

```bash
# Normal run — slow tests should not appear at all
cargo test -p lp-littlefs 2>&1 | grep -c "reentrant\|power_loss\|nonreentrant"
# Expected: 0

# With feature — slow tests compile and run
cargo test -p lp-littlefs --features slow_tests -- --list 2>&1 | grep -c "reentrant\|power_loss\|nonreentrant"
# Expected: 25+

# Remaining ignored tests are only genuinely blocked ones
cargo test -p lp-littlefs -- --list --ignored 2>&1
# Should show only: dir_seek, oopsallspaces, fewer_blocks, orphans, metadata_max, shrink, compat
```
