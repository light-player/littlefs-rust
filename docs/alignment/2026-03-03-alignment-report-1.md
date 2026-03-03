# Upstream Alignment Report

**Date:** 2026-03-03

This report summarizes the current state of alignment between the Rust lp-littlefs implementation and upstream littlefs (reference/lfs.c), reflecting recent fixes and remaining deviations.

## Test Results Summary

### lp-littlefs unit + integration

| Suite         | Passed | Failed | Ignored |
|---------------|--------|--------|---------|
| lib (unit)    | 43     | 0      | 0       |
| test_alloc    | 6      | 0      | 6       |
| test_attrs    | 0      | 0      | 4       |
| test_badblocks| 0      | 0      | 4       |
| test_bd       | 10     | 0      | 0       |
| test_compat   | 0      | 0      | 17      |
| test_dirs     | 13     | 5      | 8       |
| test_files    | 31     | 0      | 22      |
| test_interspersed | 2   | 0      | 7       |

**test_dirs failures:**
- `test_dirs_many_rename::case_2` (n=8): assertion `left: 9, right: 8` — one extra entry (d7 and x7 both visible)

### lp-littlefs-c-align (C↔Rust interop)

| Result  | Count |
|---------|-------|
| Passed  | 19    |
| Ignored | 3     |
| Failed  | 0     |

**Ignored (known divergence):**
- `rust_rename_c_sees_new_name`: Rust rename may not persist DELETE correctly; C sees both old and new names
- `rust_rmdir_c_sees_gone`: Rust remove may not persist; C rmdir returns NotEmpty
- `rust_write_c_reads_content_ctz`: Rust bdcache overflow when writing CTZ; C read fails

---

## Recent Fixes (2026-03-03)

### 1. dir_traverse_tags: include_splice for compact

**Problem:** During `dir_compact`, SPLICE tags (CREATE/DELETE) were skipped. Compact copied NAME, STRUCT, TAIL, etc., but not DELETEs. After a split/relocate, `get_tag_backwards` found old NAMEs but not the corresponding DELETEs, so renamed or deleted entries reappeared.

**Fix:** `dir_traverse_tags` now accepts `include_splice: bool`. For compact, we pass `true` so CREATE and DELETE tags are copied. For `dir_traverse_size` (split decision), we keep `include_splice: false` to match C’s NAME-only filter.

**Location:** `metadata.rs` `dir_traverse_tags`, `commit.rs` `dir_compact` (line 495).

### 2. get_tag_backwards: return Some for matched DELETE

**Problem:** When `get_tag_backwards` matched a DELETE tag (e.g. searching for `delete_gtag` in `get_entry_info`), it returned `None` instead of signaling “deleted”. `get_entry_info` checks `get_tag_backwards(..., delete_gtag)?.is_some()` to decide Noent; with `None`, deleted entries still appeared.

**Fix:** When the matched tag is `tag_isdelete(tag)`, return `Ok(Some((0, 0, 0)))` as a sentinel so `get_entry_info` correctly returns Noent.

**Location:** `metadata.rs` `get_tag_backwards` (lines 348–349).

### 3. Format refactor (dir_commit_append + mirror)

Format now uses `dir_commit_append` + block mirror instead of a custom write path, producing C-compatible layout and passing `rust_format_c_mount_root` and `c_format_rust_mount_root`.

### 4. Compact range: use count before applying attrs (removal fix)

**Problem:** When `dir_relocatingcommit` compacted after a DELETE, it used `source.count`, which was already reduced by `apply_attr_to_state`. The compact copied only tags with id in [0, count), dropping the last id. Removing the final entry (e.g. d4 in n=5, d7 in n=8) then failed with Noent because the entry was missing from the compacted block.

**Fix:** Store `count_before_apply` before the apply loop; use it as the compact range end in `dir_splittingcompact`. Also set `dir.count = end - begin` in `dir_compact` unconditionally (not only when attrs are empty) so the compacted block has the correct count for iteration.

**Location:** `commit.rs` `dir_relocatingcommit`, `dir_compact`.

---

## Remaining Deviations

### Count logic (max_id + 1)

**Status:** Kept intentionally. See `2026-03-03-count-max-id.md`.

C derives count only from splice; Rust uses `tempcount = max(tempcount, max_id + 1)` when `seen_name_or_splice`. C’s splice-only logic can yield a count that is too low for append renames; the Rust clamp avoids that. Empty child dirs are excluded via `seen_name_or_splice` so `remove` does not incorrectly return NotEmpty.

**Feasibility:** Do not align; reverting would reintroduce rename bugs.

---

### Path resolution (dir_find vs dir_find_for_create)

**Status:** Architectural split; behavior matches.

C uses one `lfs_dir_find`; the create path adds `lfs_path_islast`. Rust has separate `dir_find` and `dir_find_for_create`. Semantics are equivalent. C sets insertion id from `lfs_dir_fetchmatch`’s besttag; Rust uses `find_insertion_id` with an explicit alphabetical scan.

**Feasibility:** Low value to unify; would require broader refactor of fetch/traverse.

---

### CRC layout (single CCRC vs FCRC)

**Status:** Divergence documented.

C uses `lfs_dir_commitcrc`: alignment to `prog_size`, multiple CRC blocks, optional FCRC, padding not CRC’d. Rust uses a single 8-byte CCRC at commit end. Interop tests pass for format, mkdir, file creation, insert-before, and basic reads; layout divergence mainly affects power-loss and gc.

**Feasibility:** High effort; see `plans-done/2026-03-03-crc-layout-alignment/`.

---

### Synthetic moves in get_tag_backwards

**Status:** Not implemented.

C’s `lfs_dir_getslice` (726–734) applies synthetic moves when `lfs_gstate_hasmovehere(&lfs->gdisk, dir->pair)` before iterating backward. Rust’s `get_tag_backwards` does not take `gstate`/`gdisk` and omits this logic.

**Feasibility:** Medium. Would require threading `gstate` and `gdisk` through `get_tag_backwards` and all callers (`get_entry_info`, `get_struct`, `get_file_struct`, etc.). Most impactful for rename/move-heavy workloads.

---

### SPLICE on insert-before

**Status:** Aligned.

C uses CREATE (type1=SPLICE, splice +1) when inserting; no separate SPLICE tag. Rust does the same. The gdiff handling in `get_tag_backwards` (add gdiff only for positive splice) handles both append and insert-before. `test_dirs_mkdir_file_open_remount` and `test_dirs_other_errors` pass; c-align `rust_insert_before_c_sees_all` and `c_insert_before_rust_sees_all` pass.

---

### Other documented divergences

From `metadata-vs-lfs-c.md`:

- **dir->erased path:** C may do inline append when `dir->erased && dir->count < 0xff`; Rust always tries append first.
- **Deorphan:** C runs `lfs_fs_deorphan` after orphaningcommit; Rust does not.
- **Mlist fixup:** C updates open handles on relocation; Rust has no mlist.
- **Root update in dir_split:** C updates `lfs->root` when splitting root; Rust handles root update elsewhere.
- **Inline eviction:** C evicts large inline files before orphaningcommit; Rust does not.

---

## Failure Analysis

### test_dirs_many_rename::case_2 (n=8)

Expects 8 entries; gets 9. Suggests one stale or duplicate entry after renames, likely from count, DELETE visibility, or block selection. `case_1` (n=5) passes.

### test_dirs_many_removal / test_dirs_file_removal

**FIXED.** Root cause: when `dir_relocatingcommit` compacts after a DELETE, it used `source.count` (already reduced by `apply_attr`). The compact range [0, count) therefore excluded the last id; e.g. removing d4 (id 5) with count reduced to 5 copied only ids 0–4, dropping d4. Fix: use `count_before_apply` for the compact range so all tags are copied.

---

## Suggested Investigation Order

1. **Removal failures** — FIXED (2026-03-03): `dir_relocatingcommit` used post-apply `source.count` for the compact range. Applying DELETE reduces count before compact, so we were copying [0, count-1) and dropping the last entry. Fix: use `count_before_apply` for the compact range. See `commit.rs` `dir_relocatingcommit`.
2. **many_rename n=8** — Still fails: 9 entries (d7 and x7 both visible). Likely related to split/relocate: when root is split, block selection or DELETE visibility in the split head vs tail may be wrong. Or synthetic moves (see below).
3. **Synthetic moves** — Add `gstate`/`gdisk` to `get_tag_backwards` if rename/remove issues persist after (1)–(2).
4. **CRC/FCRC alignment** — Defer until core metadata behavior is stable; needed for power-loss and gc.

---

## References

- `alignment/2026-03-03-count-max-id.md` — Count logic rationale
- `alignment/metadata-vs-lfs-c.md` — Detailed metadata/commit/path/file comparison
- `docs/2026-03-03-parameterized-test-bugs.md` — Original failure notes and fix log
- `plans-done/2026-03-03-crc-layout-alignment/` — CRC layout alignment plan
