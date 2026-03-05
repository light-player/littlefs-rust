# test_alloc_bad_blocks: Pacman Metadata Corruption (unfixed)

## Symptom

- `test_alloc_bad_blocks_minimal` fails: `lfs_file_read` returns 0 (EOF) when reading pacman after ghost fill + GC
- On open, pacman has `ctz.head=4`, `ctz.size=0` instead of `head=6` (fileblock), `size=504`
- The root dir on disk has `CTZSTRUCT id=2` with payload `(4, 0)` instead of `(6, 504)`
- Upstream C littlefs passes the same test scenario

## Flow Comparison

**test_alloc_two_files_ctz (passes):**
1. Pacman fill to NOSPC, truncate, rewrite
2. Unmount, mount
3. Ghost create, fill to NOSPC
4. GC, unmount
5. Mount, read pacman — succeeds

**test_alloc_bad_blocks_minimal (fails):**
1. Pacman fill to NOSPC, truncate, rewrite
2. Unmount, mount
3. **Set pacman head block bad**
4. Ghost create, fill until CORRUPT or NOSPC
5. Close ghost, **clear bad block**
6. Ghost reopen, fill to NOSPC
7. GC, unmount
8. Mount, read pacman — fails (size=0, wrong head)

## Hypothesis

The corrupted value `(4, 0)` matches the "traverse exhaust-pop buffer bug" symptom (ghost's ctz written as pacman's). A fix exists in `lp-littlefs/src/dir/traverse.rs`:

- `disk_override: Option<lfs_diskoff>` in `ProcessTag`
- When popping a frame whose tag was from disk, use `Some(frame.disk)` instead of `frame.buffer`
- Verified by `test_alloc_two_files_ctz` passing

The bad-block flow introduces:

1. **CORRUPT during first ghost fill** — alloc or extend may hit the bad block; relocates occur (`lfs_alloc_lookahead`, `continue 'relocate`)
2. **More commits/compacts** — possibly a path where the disk_override fix does not apply
3. **Different block layout** — with 16 blocks, tighter layout may trigger different traverse/merge order

The `(4, 0)` pattern suggests a struct for a newly allocated block (head=4) with size 0 — consistent with ghost's state at some moment being written to pacman's id slot during a compact or commit.

## Relevant Paths

- `lp-littlefs/tests/test_alloc.rs` — `run_badblocks_minimal`, `test_alloc_bad_blocks_minimal`
- `lp-littlefs/src/dir/traverse.rs` — `disk_override` fix, `LfsDirTraverseStack`
- `lp-littlefs/src/dir/commit.rs` — `lfs_dir_commitattr`, `lfs_dir_compact`
- `lp-littlefs/src/file/ctz.rs` — `lfs_ctz_extend`, `lfs_ctz_traverse`
- `lp-littlefs/src/block_alloc/alloc.rs` — `lfs_alloc`, `lfs_alloc_scan`

## Next Steps

1. Add targeted trace in commit path when writing `CTZSTRUCT id=2` to capture source (disk location vs attr) and actual bytes
2. Compare C and Rust trace/log when running equivalent bad-block scenario
3. Check whether a second traverse/compact path exists that bypasses the disk_override logic
