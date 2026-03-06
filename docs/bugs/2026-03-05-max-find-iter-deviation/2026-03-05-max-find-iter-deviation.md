# MAX_FIND_ITER Deviation – Panic in lfs_dir_find

## Summary

We have a **defensive iteration cap** `MAX_FIND_ITER = 256` in `lfs_dir_find` (dir/find.rs:327–339) that C littlefs does not have. When exceeded, we panic. C would either find the entry or return `LFS_ERR_NOENT` when `!dir.split`. This is a deviation we must track down.

## Reproduction

```
cargo test --package lp-littlefs --test test_entries test_entries_resize_too_big
```

**Config**: 2048 blocks, 512-byte blocks, cache 512 (matches upstream ERASE_COUNT=1M/512). Path: 200 × `"m"` (single-component 200-byte name).

**Panic**:
```
loop_limits: MAX_FIND_ITER (256) exceeded name_len=200 tail=[742, 743]
```

At panic: `tail=[742, 743]` – we have walked ~372 directory block pairs (each pair = 2 blocks) before hitting the cap.

## Affected Code

**File**: `lp-littlefs/src/dir/find.rs`  
**Lines**: 325–365  

The loop at C lfs.c:1567–1584:

```c
// C has no iteration cap – exits when tag != 0 (found) or !dir->split (NOENT)
while (true) {
    tag = lfs_dir_fetchmatch(...);
    if (tag < 0) return tag;
    if (tag != 0) break;
    if (!dir->split) return LFS_ERR_NOENT;
}
```

Our version adds:

```rust
#[cfg(feature = "loop_limits")]
if find_iter > MAX_FIND_ITER {
    panic!("loop_limits: MAX_FIND_ITER ({}) exceeded name_len={} tail={:?}", ...);
}
```

`loop_limits` is on by default (Cargo.toml default features).

## Why This Is a Deviation

1. **C has no cap** – the C code relies on `!dir.split` to stop when there are no more blocks.
2. **We hit the cap** – for one file with a 200-byte name, we iterate 256+ times before the cap, so we never reach a normal exit.
3. **Implied bug** – either:
   - Our directory layout is wrong (too many splits for a long name), or
   - Our `lfs_dir_fetchmatch` / `split` handling is wrong (we never get `!split` when we should), or
   - Path parsing / `dir.tail` advancement is wrong.

## Context

- **512 blocks**: same test fails earlier with `LFS_ERR_NOSPC` during truncate+write (outline/relocate).
- **1024 blocks**: we avoid NOSPC but hit `MAX_FIND_ITER` during the subsequent read. **2048 blocks** matches upstream geometry; same bug before fix.
- **40-byte path**: test passes with 512 blocks, so short names behave correctly.

## Likely Root Cause

A single file with a 200-byte name should not require hundreds of directory block pairs. Either:

1. **Dir split logic** – `lfs_dir_split` or commit creates far more blocks than C for long names.
2. **Fetchmatch / split flag** – we advance to the next tail but never set `split = false` when there are no more blocks, so the loop never exits.
3. **Tail chain** – directory tail chain is corrupted or built incorrectly, producing an overly long chain.

## Root Cause (FIXED)

**`lfs_dir_splittingcompact`** called `lfs_dir_split` with an empty range (`split=1, end_val=1`), creating hundreds of empty directory blocks. Each block became a tail in the chain; `lfs_dir_find` had to traverse them all, exceeding `MAX_FIND_ITER`.

**Fix**: Add guard `if (end_val <= split) break` before calling `lfs_dir_split` in `lfs_dir_splittingcompact` (lp-littlefs/src/dir/commit.rs).

Trace analysis (parse_trace.py) showed 35 `dir_split` calls with empty range before the fix. C reproducer (resize_too_big.c) passes without modification.

## Completed Steps

1. Ran C reproducer — PASS.
2. Added Rust tracing; identified empty-range splits.
3. Added `end_val <= split` guard; test now passes.

**Full fix report**: [FIX_REPORT_TEMPLATE.md](FIX_REPORT_TEMPLATE.md) — staged debug process, root cause, fix, verification; use as template for future bugs.
