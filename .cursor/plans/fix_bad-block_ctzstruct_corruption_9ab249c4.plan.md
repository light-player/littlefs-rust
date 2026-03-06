---
name: Fix bad-block CTZSTRUCT corruption
overview: Investigate and fix the CTZSTRUCT metadata corruption in the bad-block test flow, where pacman's CTZ gets overwritten with ghost's CTZ `(4, 0)` during compact. Follows the staged debug approach from the MAX_FIND_ITER fix.
todos:
  - id: stage0-repro
    content: "Stage 0: Create minimal fast reproduction test with timeout, confirm smallest failing block count"
    status: completed
  - id: stage1-checks
    content: "Stage 1: Run C reproducer to confirm it passes; verify config match between C and Rust"
    status: completed
  - id: stage2-tracing
    content: "Stage 2: Add CTZSTRUCT-targeted traces in commitattr, compact, traverse; capture and analyze Rust trace log"
    status: completed
  - id: stage2-parse
    content: "Stage 2b: Write parse_trace.py to extract CTZSTRUCT commit events and flag the corruption point"
    status: completed
  - id: stage3-c-compare
    content: "Stage 3: Add matching CTZSTRUCT traces to C reproducer; compare C vs Rust commit sequences"
    status: completed
  - id: stage4-fix
    content: "Stage 4: Line-by-line comparison, identify divergence, apply fix, verify all tests pass"
    status: completed
  - id: stage5-docs
    content: "Stage 5: Write fix report following FIX_REPORT_TEMPLATE.md format"
    status: completed
isProject: false
---

# Fix bad-block CTZSTRUCT corruption (pacman metadata)

## Bug summary

`test_alloc_bad_blocks_minimal` (16 blocks) fails: after the bad-block + GC flow, pacman's CTZSTRUCT on disk is `(head=4, size=0)` instead of `(head=6, size=504)`. The `(4, 0)` pattern matches ghost's intermediate CTZ state being written to pacman's id slot during compact. C passes the same scenario.

An existing `disk_override` fix in [traverse.rs](lp-littlefs/src/dir/traverse.rs) (lines 1057-1087) handles one variant of this buffer-pointer confusion, but the bad-block flow introduces relocations and extra compacts that may hit a different code path.

## Stage 0: Minimal fast reproduction

**Goal**: A test that reproduces the bug as fast as possible and never hangs.

- Create `docs/bugs/2026-03-05-bad-blocks-ctzstruct/` directory for all artifacts.
- Add a focused test `test_bad_blocks_ctz_repro` in [test_alloc.rs](lp-littlefs/tests/test_alloc.rs) that:
  - Uses the smallest block count that reliably reproduces (run `test_alloc_bad_blocks_minimal_narrow` to confirm 16 blocks).
  - Wraps in `run_with_timeout(10, ...)` — if it hangs, the test aborts in 10s instead of forever.
  - Calls `dump_fs` on failure (already done in `run_badblocks_minimal`).
  - Optionally captures a **snapshot** of the RAM disk right before the final read, so we can inspect the on-disk CTZSTRUCT without rerunning.

## Stage 1: Quick checks

**Goal**: Confirm bug is a Rust divergence, not a config issue.

- Build and run the existing C reproducer: `make -f docs/c_reference/Makefile badblock_and_dump && ./docs/c_reference/badblock_and_dump`. Confirm it prints SUCCESS.
- Verify Rust config matches C: 16 blocks, 512-byte blocks, read/prog 16, cache 512, `compact_thresh = u32::MAX`, `block_cycles = -1`.

## Stage 2: Rust tracing

**Goal**: Capture exactly when and how the wrong CTZSTRUCT is committed.

Add targeted `lfs_trace!` calls:

1. **[commit.rs](lp-littlefs/src/dir/commit.rs) `lfs_dir_commitattr`** (line ~105): When `lfs_tag_type1(tag) == LFS_TYPE_CTZSTRUCT`, log:

- `tag` (hex), `lfs_tag_id(tag)`, `lfs_tag_isvalid(tag)` (disk vs memory)
- If from disk: `disk.block`, `disk.off`, and the actual bytes read (head, size)
- If from memory: the buffer bytes (head, size)
- The commit block and offset

1. **[commit.rs](lp-littlefs/src/dir/commit.rs) `lfs_dir_compact`** (line ~721): Log source pair, begin, end, dir pair at entry and after traverse completes.
2. **[traverse.rs](lp-littlefs/src/dir/traverse.rs) `ProcessTag`** (line ~771): When dispatching a CTZSTRUCT tag, log whether `disk_override` was used, the tag, and the buffer pointer/content.
3. **[traverse.rs](lp-littlefs/src/dir/traverse.rs) `PopAndProcess`** (line ~1057): Log the frame being popped: `frame.tag`, `frame.disk`, `frame.buffer`, whether disk_override triggers.

Run:

```
RUST_LOG=lp_littlefs=trace cargo test --features log test_bad_blocks_ctz_repro -- --nocapture 2>&1 | tee docs/bugs/2026-03-05-bad-blocks-ctzstruct/rust-trace.log
```

Write a `parse_trace.py` that:

- Extracts all CTZSTRUCT commit events
- Shows the sequence of (id, head, size, from_disk, disk_override) for each compact
- Flags the event where `id=2` (pacman) gets `(4, 0)` instead of the expected values

## Stage 3: C reproducer comparison

**Goal**: Add matching trace points to the C reproducer for comparison.

- Add `fprintf(stderr, ...)` in `lfs_dir_commitattr` (via a patched `reference/lfs.c` copy or via the existing `badblock_and_dump.c` with `#define LFS_TRACE` instrumentation) when committing CTZSTRUCT.
- Run and capture: `./docs/c_reference/badblock_and_dump 2>&1 | tee docs/bugs/2026-03-05-bad-blocks-ctzstruct/c-trace.log`
- Compare the CTZSTRUCT commit sequence between C and Rust.

## Stage 4: Line-by-line comparison and fix

**Goal**: Identify the exact divergence and fix it.

Likely areas based on the `(4, 0)` pattern (ghost's CTZ state landing in pacman's slot):

- **Traverse filter/dedup during compact**: The filter (`lfs_dir_traverse_filter`) marks tags as redundant (NOOP). If the filter incorrectly NOOPs pacman's CTZSTRUCT and then the popped tag carries ghost's data, we get the corruption. Check that `redundant_tag` / `redundant_buffer` handling is correct.
- `**disk` variable reuse after relocation: During compact with `'relocate` retry, the `disk` variable in traverse may point to stale block data if the source block was relocated. C restores `disk` from the stack frame on pop; verify Rust does the same.
- **LFS_FROM_MOVE path**: If gstate has a pending move (from the bad-block relocation), the traverse recurses with `LFS_FROM_MOVE`. The `buffer` here is an `LfsMdir`, not a `lfs_diskoff`. Check this path for buffer confusion.
- `**disk_override` edge cases: The current fix only applies when `!lfs_tag_isvalid(frame.tag)`. Check if there's a case where a disk tag gets its validity bit cleared before pop, bypassing the override.

Apply the fix, verify with:

```
cargo test test_bad_blocks_ctz_repro
cargo test -p lp-littlefs
cargo fmt
```

## Stage 5: Documentation

- Write a fix report in `docs/bugs/2026-03-05-bad-blocks-ctzstruct/` following the [FIX_REPORT_TEMPLATE.md](docs/bugs/2026-03-05-max-find-iter-deviation/FIX_REPORT_TEMPLATE.md) format.
- Include: symptoms, trace analysis, root cause, fix, verification.
