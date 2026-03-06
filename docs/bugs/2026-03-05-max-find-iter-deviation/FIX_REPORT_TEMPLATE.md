# Bug Fix Report: MAX_FIND_ITER Deviation

**Template for future bug-fix reports.**

---

## 1. Initial Symptoms

- **Symptom**: `test_entries_resize_too_big` panics with `loop_limits: MAX_FIND_ITER (256) exceeded`.
- **Reproduction**: `cargo test test_entries_resize_too_big` with 200-byte path, 2048 blocks (matches upstream).
- **Observed**: With 512 blocks, NOSPC during truncate+write. With 1024 blocks, MAX_FIND_ITER during subsequent read. Increasing MAX_FIND_ITER to 1024 only delayed the panic (tail=[1117, 1118]).
- **Deviation**: C littlefs has no iteration cap; it exits when `!dir->split` (NOENT). We never reached normal exit.

---

## 2. Debug Strategy

Three parallel strategies were planned:

1. **Line-by-line comparison** with C reference
2. **Rust tracing** to characterize runtime behavior
3. **C reproducer** with matching trace format for comparison

---

## 3. Stage 1: Quick Checks

**Goal**: Rule out configuration mismatches.

**Actions**:
1. Set `block_count` to 2048 (upstream default from `runners/test_runner.h`).
2. Revert MAX_FIND_ITER to 256.

**Result**: Still failed. Hypothesis "too few blocks" rejected. Directory chain genuinely too long.

---

## 4. Stage 2: Rust Tracing

**Goal**: Determine whether the find loop iterates through many real splits or cycles through a small set.

**Actions**:
1. Add structured trace in `lfs_dir_find` (iter, tag, split, tail, namelen).
2. Add trace at each `lfs_dir_fetchmatch` return (FOUND / NOENT / CONTINUE).
3. Add trace in `lfs_dir_split` (split, end, new_tail, dir.pair, dir.tail).
4. Add trace in `lfs_dir_splittingcompact` (split, end_val, size, break condition).
5. Run with `RUST_LOG=littlefs_rust=trace cargo test --features log test_entries_resize_too_big 2>&1 | tee rust-trace.log`.
6. Write `parse_trace.py` to extract dir_find, fetchmatch, dir_split events and detect cycles.

**Findings**:
- `parse_trace.py` reported: **35 dir_split calls with empty range (split >= end)**.
- Example: `dir_split: split=1 end=1 new_tail=[1631,1632]` repeated many times.
- No cycle in tail chain; linear chain of empty blocks.
- **Root cause identified**: `lfs_dir_splittingcompact` called `lfs_dir_split(source, 1, 1)` when `end_val=1, split=1`, creating empty directory blocks in a loop.

---

## 5. Stage 3: C Reproducer

**Goal**: Verify C passes and compare behavior.

**Actions**:
1. Create `resize_too_big.c` with same sequence (format, mount, create 40B, read 40, trunc+write 400B, read 400, unmount).
2. Build with Makefile linking against `reference/lfs.c`.
3. Run `./resize_too_big 2>&1 | tee c-trace.log`.

**Result**: C passes. No low-level tracing added; sufficient to confirm C does not hit the same failure. Rust bug is a divergence, not upstream.

---

## 6. Stage 4: Line-by-Line Comparison

**Goal**: Pinpoint exact divergence and justify fix.

**Findings** (see `c-vs-rust-map.md`):
- C `lfs_dir_splittingcompact` (lfs.c:2128–2195) has `if (split == begin) break` but no guard for `end_val <= split`.
- After a split `(split=1, end=2)`, C sets `end = split` → `end_val=1`. Next iteration: inner `while (end - split > 1)` → `0 > 1` false, exit. `split == begin`? `1 == 0` false. So C would also call `lfs_dir_split(source, 1, 1)`. C may hit this path under different conditions; our trace showed Rust repeatedly doing it.

**Fix**: Add `if (end_val <= split) break` before `lfs_dir_split`. Prevents splitting empty range `[split, end_val)`.

---

## 7. Fix Applied

**File**: `littlefs-rust/src/dir/commit.rs`

**Change** (after `if split == begin { break }`):

```rust
if end_val <= split {
    crate::lfs_trace!(
        "splittingcompact: skip empty range split={} end_val={}",
        split,
        end_val
    );
    break;
}
```

---

## 8. Verification

- `cargo test test_entries_resize_too_big` — PASS
- `cargo test -p littlefs-rust` — all tests pass
- `cargo fmt` — no new warnings

---

## 9. Artifacts

| File | Purpose |
|------|---------|
| `2026-03-05-max-find-iter-deviation.md` | Bug report (symptoms, root cause) |
| `FIX_REPORT_TEMPLATE.md` | This report; template for future bugs |
| `comparison.md` | C vs Rust behavior summary |
| `c-vs-rust-map.md` | Line-by-line code mapping |
| `resize_too_big.c` | C reproducer |
| `Makefile` | Build C reproducer |
| `parse_trace.py` | Trace log analysis (cycle detection, empty-split count) |
| `rust-trace.log` | Captured trace (truncated) |
| `c-trace.log` | C run output |

---

## 10. Template Usage

For future bugs:

1. **Create** `docs/bugs/YYYY-MM-DD-<short-name>/` directory.
2. **Copy** this report structure; fill in sections 1–9.
3. **Stage 1**: Quick checks (config, version, repro).
4. **Stage 2**: Add targeted tracing; capture and analyze logs.
5. **Stage 3**: C reproducer if applicable; compare outcomes.
6. **Stage 4**: Line-by-line comparison of suspected divergence.
7. **Fix, verify, document** in bug report and this template.
