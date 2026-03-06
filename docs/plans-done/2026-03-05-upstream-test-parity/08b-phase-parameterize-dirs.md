# Phase 8b: Parameterize test_dirs.rs

## Goal

Exact replication of upstream `reference/tests/test_dirs.toml` parameter sets. Every upstream define combination must appear as a Rust test parameterization. If new combinations fail, mark them `#[ignore = "bug: <description>"]` and move on.

## Reference

- Upstream TOML: `reference/tests/test_dirs.toml`
- Rust file: `littlefs-rust/tests/test_dirs.rs`

## Current State

All upstream cases exist as Rust functions. Most use plain `#[test]` with either a single N value or a `for n in [...]` inner loop. None use `#[rstest]`. Several cases note they use a subset of upstream N values (e.g. `n = 1` instead of `range(3, 100, 3)`).

Existing Rust-only extra at the top: `test_dirs_one_mkdir`.

## Cases to Parameterize

### test_dirs_root

Upstream: no defines. No change needed.

### test_dirs_many_creation

Upstream:
```
defines.N = range(3, 100, 3)
if = 'N < BLOCK_COUNT/2'
```
Expanded: N in [3, 6, 9, 12, 15, 18, 21, 24, 27, 30, 33, 36, 39, 42, 45, 48, 51, 54, 57, 60, 63, 66, 69, 72, 75, 78, 81, 84, 87, 90, 93, 96, 99].

Current Rust: `n = 1` (comment says "subset").

Convert to `#[rstest]` with `#[values(3, 6, 9, ..., 99)]`. Add early return when `n >= block_count / 2`. Use `block_count = 256` or higher so most N values pass the guard.

Note: upstream format strings use `"dir%03d"` — current Rust uses `"d{i}"`. The Rust test body needs to match upstream's naming to be a faithful replication.

### test_dirs_many_removal

Upstream:
```
defines.N = range(3, 100, 11)
if = 'N < BLOCK_COUNT/2'
```
Expanded: N in [3, 14, 25, 36, 47, 58, 69, 80, 91].

Current Rust: `n = 1`.

Same approach as many_creation. Upstream names: `"removeme%03d"`.

### test_dirs_many_rename

Upstream:
```
defines.N = range(3, 100, 11)
if = 'N < BLOCK_COUNT/2'
```
Expanded: N in [3, 14, 25, 36, 47, 58, 69, 80, 91].

Current Rust: `n = 1`. Upstream names: `"test%03d"` → `"tedd%03d"`.

### test_dirs_many_rename_append

Upstream:
```
defines.N = range(5, 13, 2)
if = 'N < BLOCK_COUNT/2'
```
Expanded: N in [5, 7, 9, 11].

Current Rust: `for n in [5, 7, 9, 11]` — already matches. Convert inner loop to `#[rstest]` with `#[values(5, 7, 9, 11)]` for consistency, or leave as-is since coverage matches.

### test_dirs_many_reentrant

Upstream:
```
defines.N = [5, 11]
if = 'BLOCK_COUNT >= 4*N'
reentrant = true
defines.POWERLOSS_BEHAVIOR = [LFS_EMUBD_POWERLOSS_NOOP, LFS_EMUBD_POWERLOSS_OOO]
```

Current Rust: `for n in [5, 11]` with `#[ignore = "slow: power-loss iteration"]`. Already matches N values. POWERLOSS_BEHAVIOR not yet parameterized (currently runs NOOP only via `run_powerloss_linear`). Add OOO when powerloss infra supports it, or add `#[ignore]` variant.

### test_dirs_file_creation

Upstream:
```
defines.N = range(3, 100, 11)
if = 'N < BLOCK_COUNT/2'
```
Expanded: N in [3, 14, 25, 36, 47, 58, 69, 80, 91].

Current Rust: `for n in [3, 14, 25, 36, 47, 58]` — missing [69, 80, 91].

Add the missing values.

### test_dirs_file_removal

Upstream: same N range as file_creation.

Current Rust: `for n in [3, 14, 25, 36, 47, 58]` — missing [69, 80, 91].

### test_dirs_file_rename

Upstream: same N range as file_creation.

Current Rust: `for n in [3, 14, 25, 36, 47, 58]` — missing [69, 80, 91].

### test_dirs_file_reentrant

Upstream:
```
defines.N = [5, 25]
if = 'N < BLOCK_COUNT/2'
reentrant = true
defines.POWERLOSS_BEHAVIOR = [NOOP, OOO]
```

Current Rust: `for n in [5, 25]` with `#[ignore]`. Already matches N. POWERLOSS_BEHAVIOR same situation as many_reentrant.

### test_dirs_nested

Upstream: no defines. No change needed.

### test_dirs_recursive_remove

Upstream:
```
defines.N = [10, 100]
if = 'N < BLOCK_COUNT/2'
```

Current Rust: `for n in [10, 100]`. Already matches.

### test_dirs_remove_read

Upstream:
```
defines.N = 10
if = 'N < BLOCK_COUNT/2'
```
Single value. No parameterization needed.

### test_dirs_other_errors

Upstream: no defines. No change needed.

### test_dirs_seek

Upstream:
```
defines.COUNT = [4, 128, 132]
if = 'COUNT < BLOCK_COUNT/2'
```

Current Rust: `for count in [4, 128, 132]` with `#[ignore = "requires lfs_dir_seek/tell/rewind"]`. Already matches values.

### test_dirs_toot_seek

Upstream: same as seek.

Current Rust: `for count in [4, 128, 132]` with `#[ignore]`. Already matches.

## Summary of Actual Work

| Case | Status | Action |
|------|--------|--------|
| test_dirs_root | matches | none |
| test_dirs_many_creation | N=1 vs range(3,100,3) | expand N, fix format strings |
| test_dirs_many_removal | N=1 vs range(3,100,11) | expand N, fix format strings |
| test_dirs_many_rename | N=1 vs range(3,100,11) | expand N, fix format strings |
| test_dirs_many_rename_append | matches [5,7,9,11] | optional rstest conversion |
| test_dirs_many_reentrant | matches N=[5,11] | POWERLOSS_BEHAVIOR gap (defer) |
| test_dirs_file_creation | missing [69,80,91] | add 3 values |
| test_dirs_file_removal | missing [69,80,91] | add 3 values |
| test_dirs_file_rename | missing [69,80,91] | add 3 values |
| test_dirs_file_reentrant | matches N=[5,25] | POWERLOSS_BEHAVIOR gap (defer) |
| test_dirs_nested | no defines | none |
| test_dirs_recursive_remove | matches [10,100] | none |
| test_dirs_remove_read | matches N=10 | none |
| test_dirs_other_errors | no defines | none |
| test_dirs_seek | matches [4,128,132] | none (still ignored) |
| test_dirs_toot_seek | matches [4,128,132] | none (still ignored) |

## Implementation Notes

- The main work is expanding `test_dirs_many_creation`, `many_removal`, and `many_rename` from N=1 to the full upstream ranges. These are currently documented as "subset" due to bugs. Expand them and `#[ignore]` the failing combinations.
- The `many_creation` etc. cases currently use non-upstream naming like `"d{i}"` instead of `"dir%03d"`. Align to upstream format strings.
- Use `block_count = 256` (or compute minimum) so the `if = 'N < BLOCK_COUNT/2'` guard passes for all N up to 99.
- `file_creation`, `file_removal`, `file_rename` just need 3 more N values appended.
- Convert `for n in [...]` inner loops to `#[rstest]` with `#[values]` for consistency with other parameterized tests.

## Process

```
1. Add rstest to imports
2. For many_creation/removal/rename: replace #[test] with #[rstest], add #[values(...)] with full upstream range, fix format strings, add block_count/2 guard
3. For file_creation/removal/rename: add missing N values [69, 80, 91]
4. For rename_append/recursive_remove etc.: convert inner loops to rstest if desired
5. Update all upstream comment headers
6. cargo test -p littlefs-rust --test test_dirs
7. Mark any new failures: #[ignore = "bug: <description>"]
8. cargo fmt && cargo clippy
```

## Validate

```
cargo test -p littlefs-rust --test test_dirs 2>&1
cargo fmt -p littlefs-rust
cargo clippy -p littlefs-rust
```
