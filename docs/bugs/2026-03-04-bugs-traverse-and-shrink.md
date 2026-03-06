# Traverse Buffer Bug and Shrink Segfault

Notes on two bugs, both fixed.

## 1. Traverse Exhaust-Pop Buffer Bug (fixed)

### Symptom

- `lfs_dir_get` returns wrong ctz (invalid block numbers like 81706116, 43385152) for pacman when ghost exists in the root
- `test_alloc_bad_blocks`: CORRUPT when reading pacman after clearing a bad block
- Reproduced by: pacman fill+shrink, ghost fill to NOSPC, GC, then read pacman

### Root cause

In `lfs_dir_traverse`, when we push on a tag (e.g. NAME) read from disk, we store `frame.buffer = &disk` (pointer to the outer `disk` variable). When we later exhaust tags and pop, we dispatch using `frame.buffer`. By that time, the outer `disk` has been overwritten by subsequent reads—it points at the *last* tag’s data, not the tag we pushed for. The callback therefore gets the wrong data (e.g. ghost’s ctz written as pacman’s).

### Fix (in `littlefs-rust/src/dir/traverse.rs`)

- Add `disk_override: Option<lfs_diskoff>` to `ProcessTag`
- When popping a frame whose tag was read from disk (`!lfs_tag_isvalid(frame.tag)`), use `Some(frame.disk)`—the copy saved at push—instead of `frame.buffer`
- When dispatching, use `disk_override.as_ref()` if present, else `buffer`

### Verification

`test_alloc_two_files_ctz` passes.

---

## 2. Dangling file.cfg in lfs_file_open (fixed)

### Symptom

- `test_alloc_two_files_ctz` segfaults (SIGSEGV) during `lfs_file_sync` after the shrink step
- Reproduced by: pacman fill to NOSPC, close, reopen with `LFS_O_TRUNC`, write smaller amount, sync

### Root cause

`lfs_file_open_` passed `&defaults` (a stack-local `LfsFileConfig`) to `lfs_file_opencfg_`, which stored it in `file.cfg`. After return, `defaults` went out of scope; `file.cfg` became a dangling pointer. When `lfs_file_sync_` later accessed `file_ref.cfg.as_ref()` during the metadata commit, it dereferenced freed stack memory and segfaulted.

### Fix (in `littlefs-rust/src/file/ops.rs`)

- Use `static LFS_FILE_DEFAULTS: LfsFileConfig` instead of a stack-local (matches C: `static const struct lfs_file_config defaults = {0}`)
- Add `unsafe impl Sync for LfsFileConfig` in `lfs_info.rs` (required for static; safe since the default config has all nulls and is never mutated)

### Verification

`test_alloc_two_files_ctz` passes.

---

## 3. Bad-Block Relocate Infinite Loop (test_alloc_bad_blocks hang)

### Symptom

- `test_alloc_bad_blocks` hangs (60+ s, 100% CPU, many orphaned processes)
- Timeout now aborts after 30s instead of hanging indefinitely

### Hypothesis

When erase or prog returns `LFS_ERR_CORRUPT` (bad block), the code does `continue 'relocate` (or equivalent) and calls `lfs_alloc` again. The bad block is **never marked as used** in the lookahead buffer. After `lfs_alloc_scan` repopulates the buffer, that block can appear free again (it was never committed to the filesystem), so `lfs_alloc` returns it again → infinite loop.

### Affected paths

1. **lfs_ctz_extend** (`file/ctz.rs`): erase/prog fails → `continue 'relocate` → retry without marking `nblock`
2. **lfs_file_relocate** (`file/ops.rs`): same pattern
3. **lfs_dir_compact** (`dir/commit.rs`): erase/commitprog fails → alloc new `pair[1]` but never marks old `pair[1]` as used

### Fix (proposed)

Before each retry (`continue 'relocate` or before calling `lfs_alloc` for a replacement), call `lfs_alloc_lookahead(lfs, bad_block)` to mark the bad block as used in the lookahead bitmap. Then `lfs_alloc` will never return it again.

### Fix implemented

- **lfs_ctz_extend, lfs_file_relocate**: Call `lfs_alloc_lookahead(lfs, nblock)` before each `continue 'relocate` to mark the bad block as used.
- **lfs_dir_compact**: Call `lfs_alloc_lookahead(lfs, dir_ref.pair[1])` before each CORRUPT retry.

### GC hang (fixed — was actually a `lfs_ctz_find` offset bug)

The hang was not in GC or allocation. GC completed successfully. The actual hang occurred during the final `lfs_file_read` loop: `lfs_ctz_find` returned the raw file position as the within-block offset (`*off = pos` instead of `*off = target_off`), causing an infinite loop in `lfs_file_flushedread` when `diff = min(nsize, block_size - off) = 0`.

See [../2026-03-05-ctz-find-offset/2026-03-05-ctz-find-offset.md](../2026-03-05-ctz-find-offset/2026-03-05-ctz-find-offset.md) for the full report.
