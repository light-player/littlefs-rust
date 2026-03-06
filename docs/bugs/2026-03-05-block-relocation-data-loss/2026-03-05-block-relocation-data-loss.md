# Bug: Block relocation data loss in lfs_dir_orphaningcommit

## Symptom

When `block_cycles > 0` (wear-leveling enabled), writing multiple files to the
same directory causes data loss. A file's data reverts to the content from a
previous cycle after `lfs_file_close`. The stat size shows the old size, and
re-reading the file returns old data.

Minimal reproducer: format, mkdir, write 10 files per mount cycle with
`block_cycles=5`. At cycle 4, file 5's data is lost — stat reports 128 bytes
(cycle 3's size) instead of 32 bytes (cycle 4's size).

With `block_cycles=-1` (wear-leveling disabled), all tests pass.
C reference passes all 10+ cycles with identical parameters.

## Root cause

Three translation errors in `lfs_dir_orphaningcommit` (commit.rs):

### 1. Predecessor tail update was outside the while loop

In C (lfs.c:2549-2593), the `lfs_fs_pred` lookup and tail update are **inside**
the `while (state == LFS_OK_RELOCATED)` loop. In the Rust translation, this
code was placed **outside** the loop.

When a directory pair relocates (new block allocated due to wear-leveling), the
predecessor directory's tail pointer must be updated to reference the new pair.
With the pred update outside the loop, the predecessor's tail was never updated,
leaving the old (now-stale) block as the only reachable version. Subsequent
`lfs_dir_find` → `lfs_dir_fetch` would find the old block and return stale data.

### 2. Missing mlist pair update

C (lfs.c:2486-2497) has an explicit loop inside the relocation handler that
updates all mlist entries' `m.pair` to point to the new pair, and also updates
`lfs_dir_t::head` for open directory iterators. This loop was completely missing
from the Rust translation.

Without this, open file handles referencing the relocated directory would
continue pointing to the old pair. While `relocatingcommit_fixmlist` handles
mlist updates for the *committed* directory, the relocation handler in
`lfs_dir_orphaningcommit` needs its own mlist sweep for the pair replacement.

### 3. Missing moveid tag adjustment

C (lfs.c:2523-2525): `if (moveid < lfs_tag_id(tag)) { tag -= LFS_MKTAG(0, 1, 0); }`

When fixing a pending move during relocation, if the move target has a lower id
than the parent tag, the parent tag's id must be decremented. This adjustment
was missing, which could cause the wrong directory entry to be updated.

## Additional fix: CTZ buffer lifetime in lfs_file_sync_

A secondary issue was found in `lfs_file_sync_` (ops.rs): the `ctz` copy used
for LE byte-swapping was declared inside the `else` arm of an if-else expression,
making the raw pointer to it a dangling reference after the arm completed. The
C code declares `struct lfs_ctz ctz;` before the if/else, keeping it alive
through `lfs_dir_commit`. Fixed by hoisting the declaration outside the if/else
to match C.

This was not the primary cause of the observed failure (stack reuse timing made
it latent) but is a correctness issue that could manifest under different
optimization levels or stack layouts.

## Fix

- Moved the `lfs_fs_pred` + tail update inside the `while LFS_OK_RELOCATED`
  loop (matching C structure)
- Added the mlist pair update loop inside the relocation handler
- Added the `tag -= LFS_MKTAG(0, 1, 0)` moveid adjustment
- Hoisted `let mut ctz = file_ref.ctz` outside the if/else in `lfs_file_sync_`

## Impact

- 15 of 17 `test_exhaustion` variants now pass (previously all 17 failed)
- The 2 remaining failures (`test_exhaustion_wear_leveling`,
  `test_exhaustion_wear_leveling_superblocks`) are a separate bug: `LFS_ERR_CORRUPT`
  leaks to the user at high cycle counts with actual bad blocks

## Files changed

- `lp-littlefs/src/dir/commit.rs` — `lfs_dir_orphaningcommit` relocation handler
- `lp-littlefs/src/file/ops.rs` — `lfs_file_sync_` CTZ buffer lifetime
- `lp-littlefs/tests/test_exhaustion.rs` — updated ignore annotations
