# Phase 07b: Entries (spill, drop, shrink)

## Scope

Port entry tests that stress metadata overflow and directory compaction. When inline metadata or a directory block fills, `lfs_dir_split` and `lfs_dir_compact` (reference/lfs.c:1880–2123, 1952–2123) must handle spill and reuse correctly.

**Translation rules**: [docs/rules.md](../../rules.md). Translate callees first (§0); include C source comments (§3); match logic (§2). Keep test names (§10).

---

## Tests to Port

### test_entries (from littlefs-rust-old/tests/test_entries.rs)

| Test | Validates | Notes |
|------|-----------|-------|
| `test_entries_grow` | Create file, grow | Phase 05 may cover |
| `test_entries_shrink` | Shrink file (truncate) | |
| `test_entries_spill` | Metadata spill (4 files × 200B inline) | Uses `config_with_cache(512, 128)` |
| `test_entries_push_spill` | Grow causes spill | |
| `test_entries_drop` | Remove + recreate; dir compaction | |

**Fixed**: `test_entries_create_too_big`, `test_entries_resize_too_big` (empty-range `lfs_dir_split` in `lfs_dir_splittingcompact`; see docs/bugs/2026-03-05-max-find-iter-deviation/).

---

## C Reference

- `lfs_dir_split`: reference/lfs.c:1880–1913
- `lfs_dir_compact`, `lfs_dir_splittingcompact`: reference/lfs.c:1952–2232
- `lfs_dir_commit`, `lfs_dir_alloc`, `lfs_dir_drop`: dir/commit.rs

---

## Validation

1. **Build**: `cargo build -p littlefs-rust`
2. **Tests**: All in-scope entry tests pass
3. **Format**: `cargo fmt`
4. **Warnings**: Zero warnings
