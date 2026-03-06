# test_superblocks_magic Debug Strategy

## Failure Summary

- **Expected**: `[108, 105, 116, 116, 108, 101, 102, 115]` ("littlefs") at `MAGIC_OFFSET` (12) in blocks 0 and 1
- **Actual**: `[0, 0, 0, 0, 48, 0, 0, 20]` at that offset
- **Layout**: `[rev 4][CREATE 4][SUPERBLOCK 4]["littlefs" 8]` → magic at bytes 12–19

## Isolation Results (Implemented)

| Test | Result | Conclusion |
|------|--------|------------|
| `test_superblocks_magic_bypass` | **PASS** | Direct commitattr (no traverse) writes magic correctly |
| `test_traverse_attrs_callback_order` | **PASS** | Traverse with tmask=0 passes SUPERBLOCK+buffer correctly to callback |
| `test_superblocks_magic` (real format) | **FAIL** | Format path (compact + traverse with filter) still fails |

**Conclusion**: Bug is in the **traverse filter path** (when `tmask` has `lfs_tag_id(tmask) != 0`). Format uses compact with `tmask = LFS_MKTAG(0x400, 0x3ff, 0)`, which triggers the filter recursion. The filter callback or push/pop logic is dropping or corrupting the SUPERBLOCK tag/buffer.

## Bug Location Analysis

### Traverse vs. Commit

`lfs_dir_traverse` is involved in the format path, but **only for committing**, not for reading. During format:

1. First commit → `lfs_dir_relocatingcommit` → `lfs_dir_splittingcompact` → `lfs_dir_compact`
2. `lfs_dir_compact` calls `lfs_dir_traverse` with `lfs_dir_commit_commit_raw` as the callback to emit tags onto the new block
3. Traverse iterates: disk (empty) then attrs: CREATE, SUPERBLOCK ("littlefs"), INLINESTRUCT

So the bug could be in:

| Component | Role | Suspect? |
|-----------|------|----------|
| `lfs_dir_traverse` | Iterates attrs, invokes callback with (tag, buffer) | **Yes** – goto/stack logic is error‑prone |
| `lfs_dir_commitattr` | Writes tag + buffer via `commitprog` | Possible |
| `lfs_bd_prog` / pcache | Programs block device, cache sync | Possible |
| Block layout / pair | Wrong block or offset | Lower – mount works, so layout is plausible |

### Conclusion

`lfs_dir_traverse` is a strong suspect because:

1. The C version uses goto and explicit stack recursion that is easy to mis‑translate
2. For attr‑backed tags (e.g. SUPERBLOCK), `buffer` must be the attr’s `.buffer`; wrong `.buffer` or lifetime issues would corrupt what gets committed
3. The traverse‑restructure notes already mention control‑flow mismatches (e.g. break vs. continue when exhausting tags)

But the bug may also be in commitattr (wrong size, wrong buffer passed) or in pcache (sync/prog). The strategy below tests both hypotheses.

---

## Testing Strategy

### 1. In‑Module Unit Tests for Traverse (attr iteration)

Add `#[cfg(test)] mod tests` in `littlefs-rust/src/dir/traverse.rs`:

- **Setup**: Minimal `Lfs` + RAM BD, no format/mount.
- **Test** `traverse_attrs_callback_order`: Call `lfs_dir_traverse` with synthetic attrs `[CREATE, SUPERBLOCK, INLINESTRUCT]` and a callback that appends `(tag_type, buffer_ptr, first_byte)` to a vec. Assert:
  - 3 callbacks total
  - Second callback has type SUPERBLOCK and buffer with first byte `b'l'` (or equivalent)
- **Test** `traverse_attrs_buffer_contents`: Same setup; callback copies `buffer` into a mutable buffer when tag is SUPERBLOCK. Assert contents are `b"littlefs"`.

These tests validate that traverse correctly walks attrs and passes the right pointers for attr‑backed tags.

### 2. In‑Module Unit Tests for Commit Path (commitattr)

Add tests in `littlefs-rust/src/dir/commit.rs`:

- **Setup**: `Lfs` with RAM BD, `lfs_init`, `lfs_dir_alloc` to get a fresh root.
- **Helper** `commit_superblock_only(lfs, dir)`: Build attrs `[CREATE, SUPERBLOCK with "littlefs"]`, call `lfs_dir_compact` (or the minimal commit path that writes these tags), then `lfs_bd_sync`.
- **Test** `commit_superblock_raw_read`: After `commit_superblock_only`, use `read_block_raw` and assert `buf[12..20] == b"littlefs"`.

This isolates whether the commit path (commitattr + bd_prog) writes the expected bytes at the expected offset.

### 3. Integration Test with Tracing

Add a `test_superblocks_magic_traced` (or gate tracing behind `RUST_LOG`) that:

- Uses `env_logger` / `log` (or `eprintln!` with `#[cfg(test)]`) in:
  - `lfs_dir_commit_commit_raw`: log `(tag, buffer as usize)` when tag is SUPERBLOCK
  - `lfs_dir_commitattr`: log `(tag, dsize, buffer as usize)` when tag is SUPERBLOCK
  - `lfs_bd_prog`: log `(block, off, size, first 8 bytes)` when the prog touches the superblock block
- Run with `RUST_LOG=littlefs_rust=trace cargo test test_superblocks_magic -- --nocapture`
- Confirm: (a) callback receives non‑null buffer for SUPERBLOCK, (b) commitattr receives that same buffer, (c) bd_prog sees "littlefs" in the payload.

### 4. Test Helpers (shared by integration and in‑module tests)

In `littlefs-rust/tests/common/mod.rs` (or a `dev_helpers` module):

- `format_and_read_superblock_blocks(env) -> (Vec<u8>, Vec<u8>)`: format, sync, return raw block 0 and block 1.
- `dump_block_hex(block: &[u8], label: &str)`: pretty‑print first 64 bytes for inspection (useful when a test fails).

### 5. Narrow Down: Traverse vs. Direct commitattr

Add a **bypass** test that skips traverse for format:

- New `lfs_dir_commit_format_minimal`: manually call `lfs_dir_commitattr` for CREATE, SUPERBLOCK, INLINESTRUCT in order (same as format attrs), then `lfs_dir_commitcrc`, then sync.
- If this produces the correct magic at offset 12, the bug is in traverse (or in how the real format path invokes compact/traverse).
- If this still fails, the bug is in commitattr, commitprog, or bd layer.

### 6. Traverse Control Flow (if traverse is guilty)

If tests point to traverse:

- Add `TraversePhase` logging: which phase we’re in when we fetch each tag.
- Compare with C step‑by‑step: for a single attr `[CREATE, SUPERBLOCK]`, trace C’s `off`, `ptag`, `attrs`, `attrcount` and our Rust equivalents at each iteration.
- Focus on:
  - When attrs are advanced (C: `attrs += 1; attrcount -= 1` vs Rust `attr_i += 1`)
  - When `buffer` is taken from disk vs. attrs (disk = `&disk`, attrs = `attr.buffer`)
  - Stack push/pop and what `.buffer` is stored in the frame for attr‑backed tags

---

## Implementation Order

1. **Tracing** (4) – add logging, run failing test, see where data diverges.
2. **Commit isolation** (2) – verify commitattr + bd produce correct bytes.
3. **Bypass test** (5) – distinguish traverse vs. commit/bd.
4. **Traverse unit tests** (1) – validate attr iteration if traverse is implicated.
5. **Traverse control flow** (6) – if needed, step‑debug against C.

---

## Findings (2026-03-04)

### H1 Confirmed (A/B Comparison)

**Root cause**: Rust `lfs_dir_traverse` popped at the start of GetNextTag when `sp > 0`. C never pops inside the loop; pop happens only after break (exhaust or callback res!=0). After push+continue, C gets the next tag from disk/attrs. Old Rust popped instead, skipping SUPERBLOCK.

### Fix Implemented

- **Change**: GetNextTag now never pops. It always gets the next tag from disk or attrs.
- **Pop**: Only in `PopAndProcess` (after exhaust or callback res!=0).

### Test Results After Fix

| Test | Result |
|------|--------|
| `test_traverse_filter_gets_superblock_after_push` | **PASS** – callback receives SUPERBLOCK with `b'l'` after push |
| `test_traverse_attrs_callback_order` | **PASS** |
| `test_superblocks_magic` | **FAIL** – magic still wrong; possible secondary bug in format’s second commit (compact with empty attrs) |

### H2: lfs_fs_pred early return (2026-03-04)

**Root cause**: `lfs_fs_pred` returned 0 when `pdir.tail == pair` before any fetch. With init `tail = [0, 1]` and root at `[0, 1]`/`[1, 0]`, it matched immediately, so `hasparent = true`. That triggered the tail_attrs `[NOOP, TAIL]` path in orphaningcommit, overwriting the root with a commit that had no SUPERBLOCK.

**Fix**: When `tail == pair` and we haven't fetched yet (`!have_fetched`), fetch first. For the root, the fetched dir has `tail == null`, so return `LFS_ERR_NOENT`. Otherwise return 0. File: `littlefs-rust/src/fs/parent.rs`.

### H3: MAGIC_OFFSET and block_has_magic (2026-03-04)

- **MAGIC_OFFSET=8**: C format_and_dump and Rust block 0 have "littlefs" at bytes 8–15.
- **block_has_magic**: Helper checks offset 8 or 12 (layout varies by commit path). Bypass uses offset 12.
- **Block 0**: First commit output has magic at 8. PASS.
- **Block 1**: Compact output had "lefs"+bytes at 8 (wrong). FAIL.

### H4: traverse off advancement (2026-03-04)

**Root cause**: Rust advanced `off += lfs_tag_dsize(ptag)` *after* the read; C advances *before*. That caused disk.off to point to the wrong offset when reading tag data, so commitattr read/wrote the wrong bytes for block 1.

**Fix**: Move `off += lfs_tag_dsize(ptag)` to before `lfs_bd_read`, matching C. File: `littlefs-rust/src/dir/traverse.rs`.

### Remaining Investigation

Fixed. Block 0 and block 1 both pass after H4 fix.

---

## Quick Sanity Checks Before Full Strategy

1. **Root pair after format**  
   Add `eprintln!("root.pair = {:?}", root.pair)` after format. Expect `[0, 1]`.

2. **Block content dump**  
   After format, dump `buf[0..64]` for block 0. If bytes 12–19 are zeros, commit/traverse never wrote magic; if they match some other structure (e.g. tag bytes), layout may differ from the expected one.
