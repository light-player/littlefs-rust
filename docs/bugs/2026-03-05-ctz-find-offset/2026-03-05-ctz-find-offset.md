# lfs_ctz_find returns wrong offset — infinite loop on multi-block file reads

**Status**: fixed  
**Affected test**: `test_alloc_bad_blocks` (128 blocks, hangs forever)  
**File**: `littlefs-rust/src/file/ctz.rs`, function `lfs_ctz_find`

---

## 1. Initial Symptoms

- `test_alloc_bad_blocks` hangs indefinitely (100% CPU), killed only by timeout.
- The hang occurs after all writes, GC, and remount complete — during the final `lfs_file_read` loop for pacman.
- Trace log (3M+ lines in 12s) shows a repeating cycle of `bd_read` calls: blocks 33→25→21→19→13→11→3→113→81→49→33→... endlessly.
- The 30s `run_with_timeout` kills the test before it completes.

---

## 2. Root Cause

**Translation bug in `lfs_ctz_find`**: `*off = pos` on line 203 should be `*off = target_off`.

### C reference (lfs.c:2886-2918)

```c
static int lfs_ctz_find(lfs_t *lfs,
        const lfs_cache_t *pcache, lfs_cache_t *rcache,
        lfs_block_t head, lfs_size_t size,
        lfs_size_t pos, lfs_block_t *block, lfs_off_t *off) {
    // ...
    lfs_off_t target = lfs_ctz_index(lfs, &pos);  // modifies pos IN-PLACE
    // ...
    *off = pos;   // pos now holds the within-block offset (modified by lfs_ctz_index)
    return 0;
}
```

In C, `pos` is a local copy of the argument. `lfs_ctz_index(lfs, &pos)` modifies `pos` through the pointer, converting it from a file position to a within-block offset. Then `*off = pos` returns this within-block offset.

### Rust translation (ctz.rs:153-205)

```rust
let mut target_off = pos;
let target = lfs_ctz_index(lfs, &mut target_off);  // modifies target_off
// ...
*off = pos;    // BUG: pos is the original file position, not the within-block offset
```

In Rust, `target_off` was correctly created as a mutable copy and modified by `lfs_ctz_index`. But line 203 uses the original `pos` (unchanged) instead of `target_off`.

### Why this causes an infinite loop

`lfs_ctz_find` is called from `lfs_file_flushedread` (file/ops.rs:1026). The returned `*off` is stored in `file_ref.off`. Then:

```rust
let diff = lfs_min(nsize, block_size - file_ref.off);
```

When the file position crosses a block boundary (e.g. pos=512 for block_size=512):
- **Correct** (C): `*off = 4` (within-block offset after skip pointers) → `diff = min(4, 512-4) = 4` → reads 4 bytes, advances
- **Buggy** (Rust): `*off = 512` (raw file position) → `diff = min(4, 512-512) = 0` → nothing read, pos unchanged

Since `nsize -= 0`, the `while nsize > 0` loop re-enters. `file_ref.off == 512 == block_size` triggers another `lfs_ctz_find` call with the same `pos=512`. Same result. Infinite loop, with each iteration doing a full CTZ skip-list traversal (~7 bd_read calls).

---

## 3. Why this only affects multi-block files

For files fitting in a single block (≤ 512 bytes):
- `lfs_ctz_find` is called with `pos=0`
- `lfs_ctz_index(lfs, &0)` returns 0 without modifying `*off`
- `*off = pos = 0` is correct by coincidence
- The while loop in `lfs_file_flushedread` never hits `off == block_size`

This is why `test_alloc_bad_blocks_minimal` (16 blocks, pacman truncated to 512 bytes) passes, while `test_alloc_bad_blocks` (128 blocks, pacman ~60KB spanning ~120 blocks) hangs.

No existing test creates a file larger than one block and reads it back, so this bug has been latent.

---

## 4. Proposed Fix

In `littlefs-rust/src/file/ctz.rs`, line 203:

```rust
// Before (buggy):
*off = pos;

// After (correct):
*off = target_off;
```

### Secondary issue: wrong `lfs_bd_read` hint

Line 188 passes `block_size` as the read hint. The C reference uses `sizeof(head)` = 4. This only affects caching behavior (not correctness) but should also be fixed:

```rust
// Before:
block_size,   // hint

// After:
4,            // hint: sizeof(head), matching C
```

---

## 5. Verification Plan

1. Apply the `*off = target_off` fix
2. Run `test_alloc_bad_blocks` (`cargo test -p littlefs-rust test_alloc_bad_blocks -- --ignored --nocapture`). Should complete within the 30s timeout.
3. Run full suite (`cargo test -p littlefs-rust`) to check for regressions
4. Add a dedicated multi-block file read test (write >block_size, read back, verify) to prevent future regressions

---

## 6. Trace Evidence

Trace captured with:
```
RUST_LOG=littlefs_rust=trace cargo test -p littlefs-rust test_alloc_bad_blocks --features log -- --ignored --nocapture
```

Last non-bd_read events before hang:
```
lfs_fs_gc: after forceconsistency err=0
lfs_mount(...)
mount: loop tail=[0, 1]
fetchmatch: FOUND besttag=0x00100806 pair=[0,1] count=3
dir_find: iter=1 tag=3144704 split=false tail=[0,1] namelen=6
fetchmatch: FOUND besttag=0x00100806 pair=[0,1]
```

Then 3,052,927 lines of repeating `bd_read` calls in a 10-block cycle:
```
bd_read block=33 off=0 size=512    # skip-list traversal (cache fill)
bd_read block=25 off=0 size=512
bd_read block=21 off=0 size=512
bd_read block=19 off=0 size=512
bd_read block=13 off=0 size=512
bd_read block=11 off=0 size=512
bd_read block=3  off=16 size=496   # data read (after 16-byte skip-list header)
bd_read block=113 off=16 size=496
bd_read block=81  off=16 size=496
bd_read block=49  off=16 size=496
# (repeats)
```

Each cycle is one `lfs_ctz_find` call returning with `off = 512` (the file position), triggering immediate re-entry.

---

## 7. Relationship to Other Bugs

This bug was previously noted in `docs/bugs/2026-03-04-bugs-traverse-and-shrink.md` section 3 ("GC hang") with an incorrect hypothesis about `lfs_alloc_scan` being the hang site. The actual hang is in `lfs_file_flushedread`, not in GC or allocation. GC completes successfully; the hang occurs during the subsequent file read phase.

The "Pacman Metadata Corruption" bug (fixed via `disk_override` in traverse.rs) is a separate issue affecting 16-block tests. This offset bug affects 128-block tests where files span multiple blocks.
