# Bug Fix Report: Bad-block CTZSTRUCT Corruption (Pacman Metadata)

---

## 1. Initial Symptoms

- **Symptom**: `test_alloc_bad_blocks_minimal` fails: `lfs_file_read` returns 0 (EOF) when reading pacman after ghost fill + GC. On open, pacman has `ctz.head=4`, `ctz.size=0` instead of `head=6`, `size=504`. Root dir on disk has CTZSTRUCT id=2 with payload `(4, 0)` instead of `(6, 504)`.
- **Reproduction**: `run_badblocks_minimal(16)` — pacman fill to NOSPC, truncate, rewrite; unmount; mark pacman head bad; ghost fill until CORRUPT; clear bad; ghost to NOSPC; GC; unmount; mount and read pacman.
- **Deviation**: Upstream C littlefs passes the same scenario. The `(4, 0)` pattern matches ghost's intermediate CTZ state being written to pacman's id slot during compact.

---

## 2. Debug Strategy

Staged investigation (following MAX_FIND_ITER fix approach):

1. Minimal fast reproduction test with timeout
2. C reproducer verification
3. Rust tracing (CTZSTRUCT commitattr, compact, traverse)
4. Trace analysis script
5. Line-by-line comparison and fix verification

---

## 3. Stage 1: Quick Checks

**Goal**: Confirm bug is Rust divergence, not config.

**Actions**:
1. Build and run C reproducer: `make -f docs/c_reference/Makefile badblock_and_dump && ./badblock_and_dump`.
2. Verify Rust config matches C: 16 blocks, 512-byte blocks, read/prog 16, cache 512, `compact_thresh = u32::MAX`, `block_cycles = -1`.

**Result**: C passes (SUCCESS, read 512 bytes). Config verified. At time of this investigation, Rust `test_alloc_bad_blocks_minimal` and `test_bad_blocks_ctz_repro` also pass — indicating the bug was already fixed by prior work.

---

## 4. Stage 2: Rust Tracing

**Goal**: Confirm the disk_override fix is in use and CTZSTRUCT commits are correct.

**Actions**:
1. Add CTZSTRUCT-targeted trace in `lfs_dir_commitattr` (id, from_disk, head/size or disk block/off).
2. Add trace in `lfs_dir_compact` (source pair, begin, end, dir pair).
3. Add trace in `lfs_dir_traverse` ProcessTag and PopAndProcess for CTZSTRUCT (disk_override usage).
4. Run with `RUST_LOG=lp_littlefs=trace cargo test --features log test_bad_blocks_ctz_repro -- --nocapture 2>&1 | tee rust-trace.log`.
5. Write `parse_trace.py` to extract CTZSTRUCT events.

**Findings**:
- `commitattr CTZSTRUCT`: id=1 (ghost) from_mem head=6 size=512; id=2 (pacman) from_disk disk=(1,58).
- `traverse PopAndProcess CTZSTRUCT`: id=2 with `disk_override=TRUE` — the disk_override mechanism correctly supplies the saved `frame.disk` when the outer `disk` variable would have been overwritten by subsequent reads.
- No corruption: pacman read succeeds. The disk_override fix (lp-littlefs/src/dir/traverse.rs) is working.

---

## 5. Stage 3: C Reproducer

**Goal**: Verify C passes; compare outcomes.

**Actions**:
1. Run existing `badblock_and_dump` (docs/c_reference/badblock_and_dump.c).
2. Capture output to c-trace.log.

**Result**: C passes. No CTZSTRUCT-level tracing added (would require patching upstream lfs.c). Both C and Rust pass the same scenario.

---

## 6. Stage 4: Root Cause and Fix

**Root cause** (from original bug report): During `lfs_dir_traverse`, when a tag is read from disk, `buffer = &disk` points at the shared `disk` variable. The loop continues; further `GetNextTag` reads overwrite `disk`. When the inner traversal exhausts tags and pops, `frame.buffer` still points at the shared `disk`, which now contains data from the last read — i.e., another file's CTZ (e.g. ghost's) instead of the original tag's (pacman's).

**Fix** (already present in lp-littlefs): The `disk_override` mechanism in `lfs_dir_traverse` (traverse.rs):

- When pushing a frame, `frame.disk` stores a copy of the current `disk` (lfs_diskoff).
- On pop, if the tag came from disk (`!lfs_tag_isvalid(frame.tag)`), set `disk_override = Some(frame.disk)`.
- In ProcessTag, when dispatching to the callback, use `disk_override` (the saved copy) instead of `buffer` (which pointed at the overwritten `disk`), so the callback receives the correct lfs_diskoff for reading from disk.

**Verification**: No code change required. Tests pass; trace confirms disk_override used for pacman's CTZSTRUCT.

---

## 7. Fix Location

**File**: `lp-littlefs/src/dir/traverse.rs`

**Key logic** (PopAndProcess, ~lines 1066–1080):

```rust
let disk_override = if !crate::tag::lfs_tag_isvalid(frame.tag) {
    Some(frame.disk)
} else {
    None
};
// ...
phase = TraversePhase::ProcessTag {
    tag: proc_tag,
    buffer: frame.buffer,
    disk_override,
};
```

**Dispatch** (ProcessTag, ~lines 1012–1016):

```rust
let actual_buffer = match disk_override {
    Some(ref d) => d as *const _ as *const core::ffi::c_void,
    None => buffer,
};
res = dispatch_tag(cb, data, tag, actual_buffer, diff);
```

---

## 8. Verification

- `cargo test test_bad_blocks_ctz_repro` — PASS
- `cargo test test_alloc_bad_blocks_minimal` — PASS
- `cargo test -p lp-littlefs` — all tests pass
- `cargo fmt` — no new warnings

---

## 9. Artifacts

| File | Purpose |
|------|---------|
| `2026-03-04-bad-blocks-pacman-metadata.md` | Original bug report (docs/bugs/2026-03-04-bad-blocks/) |
| `parse_trace.py` | Trace log analysis (CTZSTRUCT commit events) |
| `rust-trace.log` | Captured trace with CTZSTRUCT events |
| `c-trace.log` | C reproducer output |
| `badblock_and_dump.c` | C reproducer (docs/c_reference/) |

---

## 10. Summary

The bad-block CTZSTRUCT corruption was fixed by the `disk_override` mechanism in `lfs_dir_traverse`. When compacting a directory, tags read from disk use `buffer = &disk`. After recursion (e.g. filter) exhausts and pops, the outer `disk` has been overwritten by later reads. Passing `frame.buffer` to the commit callback would pass wrong data (e.g. ghost's CTZ for pacman's id). Using `disk_override = Some(frame.disk)` ensures the callback receives the correct disk offset. The fix was implemented previously; this report documents the investigation and confirms it addresses the bad-block scenario.
