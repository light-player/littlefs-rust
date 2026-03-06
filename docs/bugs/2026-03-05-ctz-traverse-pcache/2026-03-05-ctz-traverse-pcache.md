# Bug: lfs_ctz_traverse ignores pcache parameter

**Date:** 2026-03-05
**Status:** Fixed
**Severity:** High — causes LFS_ERR_CORRUPT to leak to users during wear-leveling

## Symptoms

- `test_exhaustion_wear_leveling` fails at cycle 94: `write returned -84` (LFS_ERR_CORRUPT)
- `test_exhaustion_wear_leveling_superblocks` fails with data corruption (reads 0xFF instead of expected data)
- Only occurs when `block_cycles > 0` and many blocks are worn out (>85% of device)

## Root cause

Translation bug in `lfs_ctz_traverse` (file/ctz.rs). The function accepts a `pcache` parameter
but passes `core::ptr::null()` to `lfs_bd_read` instead of forwarding it.

C reference (lfs.c:3042-3044):
```c
err = lfs_bd_read(lfs,
        pcache, rcache, count*sizeof(head),
        head, 0, &heads, count*sizeof(head));
```

Rust (before fix):
```rust
let err = lfs_bd_read(
    lfs,
    core::ptr::null(),  // BUG: should be pcache
    &mut *rcache,
    ...
);
```

## Failure mechanism

1. File write triggers a cache flush to a worn-out block → prog returns CORRUPT
2. `lfs_file_relocate` allocates a new block, erases it (fills with 0xFF), copies cached data
3. After several relocation retries (all hitting worn blocks), the allocator needs to rescan
4. `lfs_alloc_scan` → `lfs_fs_traverse_` iterates open files on the mlist
5. The open file's current block was erased (0xFF on device) but the real data is only in
   the file's prog cache — never flushed because the device prog kept failing
6. `lfs_ctz_traverse` reads skip pointers from the block device (bypassing cache) → gets
   0xFFFFFFFF → `lfs_bd_read` bounds check returns CORRUPT → propagates to user

In C, the pcache is correctly forwarded, so the read serves data from the file cache
(which has the correct skip pointers), avoiding the bad device read entirely.

A secondary issue: the `hint` parameter was also wrong (passed `block_size` instead of
`count*sizeof(head)` as in C), though this only affects read caching efficiency.

## Fix

- Forward `pcache` to `lfs_bd_read` in `lfs_ctz_traverse` instead of passing NULL
- Correct the `hint` parameter to match C: `read_size` (= `count * sizeof(lfs_block_t)`)

## Files changed

- `lp-littlefs/src/file/ctz.rs` — fix `lfs_ctz_traverse` to use pcache parameter
- `lp-littlefs/tests/test_exhaustion.rs` — un-ignore the two wear leveling tests

## Verification

All 17 `test_exhaustion` variants pass, including:
- `test_exhaustion_wear_leveling`: 94 cycles (half) / 206 cycles (full), ratio 2.19
- `test_exhaustion_wear_leveling_superblocks`: 99 cycles (half) / 216 cycles (full), ratio 2.18

Full test suite: zero failures, zero warnings.
