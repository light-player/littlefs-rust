# Phase 07: Remaining Tests (paths, move, entries, attrs, relocations, alloc)

## Scope

Bring over all remaining relevant tests from the reference code that are not covered by Phases 01–06. Implement whatever API or behavior each test requires. May span multiple sub-phases or be done incrementally.

**Translation rules**: [docs/rules.md](../rules.md). Translate callees first (§0); include C source comments (§3); match logic (§2). Keep test names (§10).

---

## Test Modules to Port (All Relevant)

### test_paths (remaining)

| Test | Validates | Notes |
|------|-----------|-------|
| `test_paths_simple_dirs` | Nested mkdir, stat | Phase 03 may cover |
| `test_paths_simple_files` | Nested file create | Phase 05 may cover |
| `test_paths_absolute_files` | Absolute paths | |
| `test_paths_absolute_dirs` | Absolute path dirs | |
| Remaining path edge cases | Dots, UTF-8, etc. | May need #[ignore] initially |

### test_move

| Test | Validates | Notes |
|------|-----------|-------|
| `test_move_nop` | Rename same to same | |
| `test_move_create_delete_same` | Create/delete same name | |
| `test_move_create_delete_delete_same` | Create/delete/delete | |
| `test_move_create_delete_different` | Cross-dir (if FROM_MOVE implemented) | |
| `test_move_file` | Cross-dir rename | Requires FROM_MOVE |
| `test_move_dir` | Cross-dir dir rename | Requires FROM_MOVE |
| `test_move_state_stealing` | Move state | Requires FROM_MOVE |

**Deferred** (corruption/power-loss): `test_move_file_corrupt_*`, `test_move_dir_corrupt_*`, `test_move_reentrant_*`, `test_move_fix_relocation_*`.

### test_entries

| Test | Validates | Notes |
|------|-----------|-------|
| `test_entries_grow` | Create file, grow | Phase 05 may cover |
| `test_entries_shrink` | Shrink file | |
| `test_entries_spill` | Metadata spill | |
| `test_entries_push_spill` | Push causes spill | |
| `test_entries_drop` | Drop/dir compaction | |

**Deferred**: `test_entries_create_too_big`, `test_entries_resize_too_big`.

### test_attrs

| Test | Validates | Notes |
|------|-----------|-------|
| `test_attrs_get_set` | getattr, setattr, removeattr | Requires attr API |
| `test_attrs_get_set_root` | Attrs on root | |
| `test_attrs_get_set_file` | Attrs on files | |
| `test_attrs_deferred_file` | file_opencfg attrs | |

All require `lfs_getattr`, `lfs_setattr`, `lfs_removeattr`. Deferred per overview; port tests, implement if needed.

### test_relocations

| Test | Validates | Notes |
|------|-----------|-------|
| `test_relocations_dangling_split_dir` | Dangling split dir | |
| `test_relocations_outdated_head` | Outdated head | |
| `test_relocations_nonreentrant` | Non-reentrant reloc | |
| `test_relocations_nonreentrant_renames` | Non-reentrant renames | |

**Deferred** (power-loss): `test_relocations_reentrant`, `test_relocations_reentrant_renames`.

### test_alloc

| Test | Validates | Notes |
|------|-----------|-------|
| `test_alloc_parallel` | Parallel alloc | |
| `test_alloc_serial` | Serial alloc | |
| `test_alloc_parallel_reuse` | Parallel reuse | |
| `test_alloc_serial_reuse` | Serial reuse | |
| `test_alloc_exhaustion` | Alloc exhaustion | |
| `test_alloc_split_dir` | Split dir alloc | |

**Deferred**: `test_alloc_exhaustion_wraparound`, `test_alloc_dir_exhaustion`, `test_alloc_bad_blocks`, `test_alloc_chained_dir_exhaustion`, `test_alloc_outdated_lookahead`, `test_alloc_outdated_lookahead_split_dir`.

---

## Implementation Order

1. Paths (remaining) — if not fully covered in Phase 03
2. Move (same-dir cases first; cross-dir requires FROM_MOVE)
3. Entries (spill, drop)
4. Relocations (non-reentrant)
5. Alloc (parallel, serial, exhaustion, split_dir)
6. Attrs (if in scope)

---

## Validation

Before considering this phase complete:

1. **Build**: `cargo build -p lp-littlefs`
2. **Tests**: `cargo test -p lp-littlefs` — all ported tests in scope pass
3. **Format**: `cargo fmt`
4. **Warnings**: Zero warnings
5. **Documentation**: Any deferred tests listed with rationale
