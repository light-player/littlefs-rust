# Parameterized test failures (rstest bugs)

**Date:** 2026-03-03

This document records bugs exposed when running parameterized tests with broader ranges. Tests were originally parameterized to match upstream littlefs defines-driven permutations; when failures occurred, ranges were trimmed to make tests pass. This report documents the failures observed when those ranges are restored, and what they suggest about implementation defects.

## 1. test_dirs_many_rename (n=8)

**Test:** Create `n` dirs, rename each d{i} -> x{i}, assert `dir_entry_names` returns exactly `n` entries.

**Failure:** For n=8 (and likely n=6), `dir_entry_names` returns too many entries (e.g. 11 instead of 6 or 8).

**Actual symptom (from trace):** `names=["d0", "d1", "d2", "x0", "x1", "x2", "x3", "x4", "x5", "x6", "x7"]` — old names d0..d2 remain visible alongside new x0..x7.

**Investigation:**
- `dir_find_for_create` returns insert id (9, 10, 11, ...) for each rename when target doesn't exist — matches upstream `lfs_dir_find` behaviour (newid = dir->count).
- Upstream uses same approach: allocate new slot, CREATE newid, DELETE oldid. Our rename logic does the same (delete_old=true, CommitAttr::delete(newoldid)).
- Trace shows dir_read returns d0,d1,d2 at ids 1,2,3 and Noent for 4-8, then x0-x7 at ids 9-16. So ids 1-3 are not seeing their DELETE tags when read; ids 4-8 correctly show Noent.
- Root cause likely: DELETE tags for ids 1-3 not visible when reading — perhaps split/relocate copies entries to a new block without the corresponding DELETEs, or revision/block selection reads stale block. Needs deeper commit/compact/split tracing.
- **Upstream alignment:** Upstream uses new slot + delete. A workaround of reusing old_id when same_pair+Noent would fix the symptom but diverges from upstream; prefer fixing the DELETE visibility bug.

**Implied bug:** DELETE tags for renamed entries may not be persisted or visible after split/relocate; or block selection reads from the wrong revision.

**Fix (2026-03-03):** `dir_traverse_tags` was skipping SPLICE (CREATE/DELETE) tags entirely. During compact, DELETEs were not copied to the new block, so `get_tag_backwards` found old NAMEs but not DELETEs. Now we copy SPLICE tags when `include_splice=true` (compact); keep them excluded from `dir_traverse_size` to match C's split decision. Also fixed `get_tag_backwards`: when matching a DELETE tag, return `Some` so `get_entry_info` correctly returns Noent.

---

## 2. test_dirs_file_rename (n=8)

**Test:** Create `n` files (test000..test007), rename each test{i} -> x{i}, assert dir listing shows only new names.

**Failure:** Dir listing shows both old names (test000..test003) and new names (x000..x007). Inconsistent rename/directory state.

**Implied bug:** Renamed entries are not correctly removed from the directory, or old and new names coexist incorrectly. Suggests rename or directory metadata update bug.

---

## 3. test_truncate_simple / test_truncate_read / test_truncate_write (32, 33)

**Test:** Write 33 bytes, truncate to 32, remount, read. Assert exactly 32 bytes read with correct pattern.

**Failure:** Truncate to 32 bytes is not respected; read returns 33 bytes.

**Implied bug:** Truncate near block boundary (32 < block size) may not be applied correctly. Possible size field not updated or boundary logic error.

---

## 4. test_truncate_* (2048, 2049)

**Test:** Write 2049 bytes, truncate to 2048, read back. Assert 2048 bytes with correct content.

**Failure:** Wrong data read (e.g. byte 49 vs expected 104). Truncation appears to succeed by size but read returns incorrect bytes.

**Implied bug:** Block-boundary or cache interaction. Truncate at block boundary may leave stale data in cache or fail to invalidate/update correctly. Likely bdcache or commit interaction.

---

## 5. test_interspersed_remove_files (10, 10)

**Test:** Create 10 files, write to each, close all, then remove each in order.

**Failure:** `Noent` when removing a file we just created and closed. File appears to vanish or not persist before removal.

**Implied bug:** Remove/append or creation/commit interaction. Possibly directory compaction or metadata pair update bug when many files are present.

---

## 6. test_interspersed_files (10, 26)

**Test:** Create 26 single-letter files (a..z), write 10 bytes to each, close, assert all 26 exist and are readable.

**Failure:** Only 5 files appear in dir listing instead of 26.

**Implied bug:** Limit on directory entries or file creation. May hit metadata pair size limit or some internal cap. Suggests directory or metadata pair capacity bug.

---

## Summary

| Test                    | Params    | Symptom                                   |
|-------------------------|-----------|-------------------------------------------|
| test_dirs_many_rename   | n=8       | Too many dir entries (duplicates?)        |
| test_dirs_file_rename   | n=8       | Old and new names both visible            |
| test_truncate_*         | (32, 33)  | Truncate not respected, 33 bytes read     |
| test_truncate_*         | (2048,2049)| Wrong data after truncate at block boundary |
| test_interspersed_remove_files | (10, 10) | Noent when removing just-created file |
| test_interspersed_files | (10, 26) | Only 5 files created / visible            |

## Suggested investigation order

1. **Directory capacity / metadata pair** (items 5, 6) — root cause may limit how many entries or files can coexist.
2. **Rename and dir iteration** (items 1, 2) — shared with metadata/dir logic.
3. **Truncate boundary handling** (items 3, 4) — possibly separate bdcache/commit path.
