# Phase 07a: Paths and Move

## Scope

Port remaining path tests and all move/rename tests. Path resolution and absolute-path handling may overlap with Phases 03 and 05; verify coverage and add missing tests. Cross-dir rename requires `lfs_rename_` (reference/lfs.c:3961–4138) to handle `!samepair` and FROM_MOVE gstate.

**Translation rules**: [docs/rules.md](../rules.md). Translate callees first (§0); include C source comments (§3); match logic (§2). Keep test names (§10).

---

## Tests to Port

### test_paths (from lp-littlefs-old/tests/test_paths.rs)

| Test | Validates | Notes |
|------|-----------|-------|
| `test_paths_simple_dirs` | Nested mkdir, stat | Phase 03 may cover |
| `test_paths_simple_files` | Nested file create | Phase 05 may cover |
| `test_paths_absolute_files` | Absolute paths (`/coffee/name`) | |
| `test_paths_absolute_dirs` | Absolute path dirs | |
| `test_paths_noent` | NOENT on bad names | |
| `test_paths_root` | `stat("/")`, `dir_open("/")` | |

**Deferred** (may need `#[ignore]` initially): `test_paths_redundant_slashes`, `test_paths_trailing_slashes`, `test_paths_dots`, `test_paths_dotdots`, `test_paths_leading_dots`, `test_paths_root_dotdots`, `test_paths_noent_parent`, `test_paths_notdir_parent`, `test_paths_empty`, `test_paths_root_aliases`, `test_paths_magic_noent`, `test_paths_magic_conflict`, `test_paths_nametoolong`, `test_paths_namejustlongenough`, `test_paths_utf8`, `test_paths_spaces`, `test_paths_nonprintable`, `test_paths_nonutf8`, etc. Per [rules.md §10](../rules.md): port with `#[ignore]` if edge-case behavior unclear.

### test_move

| Test | Validates | Notes |
|------|-----------|-------|
| `test_move_nop` | Rename same to same | |
| `test_move_create_delete_same` | Create/delete same name | May need `#[ignore]` if rename with open files differs |
| `test_move_create_delete_delete_same` | Create/delete/delete | |
| `test_move_create_delete_different` | Cross-dir (if FROM_MOVE) | |
| `test_move_file` | Cross-dir rename file | Requires `lfs_rename_` cross-dir |
| `test_move_dir` | Cross-dir dir rename | |
| `test_move_state_stealing` | Chain move, remove intermediates | |

**Deferred**: `test_move_file_corrupt_*`, `test_move_dir_corrupt_*`, `test_move_reentrant_*`, `test_move_fix_relocation_*` (corruption/power-loss infra).

---

## C Reference

- `lfs_rename_`: reference/lfs.c:3961–4138
- Path resolution: `lfs_dir_find` (dir/find.rs), `lfs_path_isdir`, `lfs_path_islast`, `lfs_path_namelen`
- Cross-dir: `lfs_pair_cmp`, `lfs_fs_prepmove`, `lfs_fs_pred`

---

## Validation

1. **Build**: `cargo build -p lp-littlefs`
2. **Tests**: All in-scope path and move tests pass
3. **Format**: `cargo fmt`
4. **Warnings**: Zero warnings
