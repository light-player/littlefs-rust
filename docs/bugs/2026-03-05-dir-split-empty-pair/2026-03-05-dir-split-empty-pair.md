# Dir Split Empty Pair – Spurious "/" in dir listing after mass remove

## Summary

After creating 26 files in root and removing all of them (interleaved with writes to a "zzz" file), `lfs_dir_read` returns a spurious "/" entry and enters an infinite loop. Root cause: the Rust translation of `lfs_dir_relocatingcommit` failed to skip the commit/compact section when `LFS_OK_DROPPED` was set, a mistranslation of the C `goto fixmlist`.

## Reproduction

```
cargo test -p lp-littlefs --test test_interspersed test_interspersed_remove_files -- files_3_26
```

**Config**: 128 blocks, 512-byte blocks (default).

**Sequence**:
1. Format, mount
2. Create 26 files (a-z) with 10 bytes each (root directory splits into 3-4 metadata pairs)
3. Unmount, remount
4. Open "zzz" for writing
5. Loop: write 1 byte to "zzz", sync, remove lettered file
6. Close "zzz"
7. `lfs_dir_read("/")` returns: ".", "..", **"/"** (bug), then hits loop limit

## Root Cause (FIXED)

### C behavior

In C `lfs_dir_relocatingcommit` (lfs.c:2257-2268), when a delete causes `dir->count == 0`:

```c
if (hasdelete && dir->count == 0) {
    LFS_ASSERT(pdir);
    int err = lfs_fs_pred(lfs, dir->pair, pdir);
    if (err && err != LFS_ERR_NOENT) { return err; }
    if (err != LFS_ERR_NOENT && pdir->split) {
        state = LFS_OK_DROPPED;
        goto fixmlist;           // <-- skips commit/compact entirely
    }
}
```

The `goto fixmlist` skips the commit and compact sections. The caller (`lfs_dir_orphaningcommit`) then updates the predecessor's tail to skip over the dropped pair, removing it from the metadata chain.

### Rust bug

The Rust translation had `// goto fixmlist` as a comment but the code fell through to the commit/compact section:

```rust
if pdir_ref.split {
    state = crate::error::LFS_OK_DROPPED;
    // goto fixmlist   <-- NO ACTUAL JUMP
}
// ... falls through to commit/compact code ...
```

Since `count == 0 < 0xff`, the commit section was entered, writing the deletion to the pair. This left the pair on disk with `count=0` and `split=true` — a dangling entry in the metadata chain.

### Symptom chain

1. Empty metadata pair stays in the tail chain with `count=0, split=true`
2. `lfs_dir_read_` follows the tail into this pair, resets `id=0`
3. Falls through to `getinfo(0)` without re-checking `id == count` (same structure as C)
4. `getinfo(0)` returns `LFS_ERR_NOENT` (no entries)
5. `id` increments to 1; `1 != 0` so the tail-follow check (`id == count`) never matches again
6. `id` keeps incrementing until 1023 (0x3ff), where `getinfo(0x3ff)` returns "/" (the root special case)
7. Loop continues until the 2048 iteration limit

## Fix Applied

**File**: `lp-littlefs/src/dir/commit.rs`

When `state == LFS_OK_DROPPED`, return early to fixmlist, skipping commit/compact:

```rust
if state == crate::error::LFS_OK_DROPPED {
    return relocatingcommit_fixmlist(lfs, dir, pair, attrs, attrcount, state);
}
```

## Verification

- `cargo test -p lp-littlefs --test test_interspersed test_interspersed_remove_files` — all 6 variants PASS
- `cargo test -p lp-littlefs` — all tests PASS (except pre-existing sparse-hole issue in test_seek)
- C reproducer (`repro_remove26.c`) — PASS (confirms C handles this correctly)

## C Trace Comparison

C trace shows pairs being absorbed when their count reaches 0:

```
after remove 'l' (#12): [0,1]→[23,24](count=1)→[25,26]→[106,107]
after remove 'm' (#13): [0,1]→[25,26]→[107,106]          ← [23,24] absorbed
...
after remove 't' (#20): [0,1]→[106,107](count=7)          ← [25,26] absorbed
...
FINAL:                   [0,1]→[107,106](count=1)          ← only "zzz" remains
```

Rust (before fix) left empty pairs in the chain, causing the dir_read loop to iterate past count=0 pairs.

## Artifacts

| File | Purpose |
|------|---------|
| `2026-03-05-dir-split-empty-pair.md` | This report |
| `repro_remove26.c` | C reproducer (PASS) |
| `repro_remove26_trace.c` | C reproducer with metadata chain tracing |
| `Makefile` | Build C reproducers |
| `c-trace.log` | C basic run output |
| `c-trace-detail.log` | C trace with per-remove chain dumps |
