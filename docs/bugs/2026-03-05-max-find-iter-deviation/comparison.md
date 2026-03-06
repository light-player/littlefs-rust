# C vs Rust Comparison: test_entries_resize_too_big

## Summary

**Root cause (Rust)**: `lfs_dir_splittingcompact` called `lfs_dir_split` with an empty range (`split=1, end_val=1`), creating hundreds of empty directory blocks. Each block became a tail in the chain; `lfs_dir_find` had to traverse them all, exceeding `MAX_FIND_ITER`.

**Fix**: Add guard `if (end_val <= split) break` before calling `lfs_dir_split` in `lfs_dir_splittingcompact` (littlefs-rust/src/dir/commit.rs).

## Behavior Comparison

| Aspect | C (upstream) | Rust (before fix) | Rust (after fix) |
|--------|-------------|-------------------|------------------|
| Test result | PASS | MAX_FIND_ITER panic | PASS |
| dir_split empty-range calls | 0 (never reaches) | 35 | 0 |
| Dir block pairs created | ~2–3 | 36+ | ~2–3 |

## Why C Avoids the Bug

The C `lfs_dir_splittingcompact` uses the same logic (`split == begin` to break). The difference is that C does not reach the `split=1, end_val=1` state in the same way. Possible reasons:

1. Different call patterns or entry counts when compacting.
2. The C `lfs_dir_split` with `split >= end` may be a latent bug that is not exercised by this test.

The Rust trace showed `dir_split: split=1 end=1` repeated 35 times. The guard `end_val <= split` correctly prevents splitting an empty range.

## Files

- **resize_too_big.c**: C reproducer; passes.
- **c-trace.log**: C run output (no low-level tracing).
- **rust-trace.log**: Rust trace before fix; used to identify empty splits.
- **parse_trace.py**: Extracts dir_find/fetchmatch/dir_split events and flags empty-range splits.
