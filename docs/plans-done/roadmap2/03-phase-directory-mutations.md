# Phase 03: Directory Mutations (mkdir, remove, rename)

## Scope

Implement directory creation, removal, and rename. Requires block allocation (lookahead), directory commit machinery (CREATE, DELETE, NAME, DIRSTRUCT, TAIL tags), and path resolution for writes.

**Translation rules**: [docs/rules.md](../../rules.md). Translate callees first (§0); include C source comments (§3); match logic (§2). Preserve call graph (§8).

---

## API Targets

| API | Upstream | Description |
|-----|----------|-------------|
| `lfs_mkdir(path) -> i32` | lfs.h | Create directory |
| `lfs_remove(path) -> i32` | lfs.h | Remove file or empty dir |
| `lfs_rename(old, new) -> i32` | lfs.h | Move/rename file or dir |

Config additions: `block_cycles`, `lookahead_size`, `lookahead_buffer` (optional).

---

## Tests to Port (All Relevant)

From lp-littlefs-old/test_dirs.rs, test_paths.rs. Same names per [rules.md §10](../../rules.md).

| Source | Test | Validates |
|--------|------|-----------|
| test_dirs.toml | `test_dirs_root` | Prereq from Phase 01 |
| test_dirs.toml | `test_dirs_one_mkdir` | mkdir, stat, dir_read |
| test_dirs.toml | `test_dirs_many_creation` | mkdir N, dir_read lists them |
| test_dirs.toml | `test_dirs_many_removal` | mkdir N, remove all, dir_read empty |
| test_dirs.toml | `test_dirs_many_rename` | mkdir N, rename all, verify |
| test_dirs.toml | `test_dirs_one_rename` | Single rename |
| test_dirs.toml | `test_dirs_mkdir_remount` | mkdir, unmount, mount, dir_read |
| test_dirs.toml | `test_dirs_mkdir_file_open_remount` | mkdir, create file, remount |
| test_dirs.toml | `test_dirs_file_only_remount` | File create, remount |
| test_dirs.toml | `test_dirs_other_errors` | NOENT, ISDIR, NOTEMPTY, etc. |
| test_dirs.toml | `test_dirs_file_creation` | Create N files |
| test_dirs.toml | `test_dirs_file_removal` | Remove N files |
| test_dirs.toml | `test_dirs_file_rename` | Rename N files |
| test_paths.toml | `test_paths_simple_dirs` | Nested mkdir, stat |
| test_paths.toml | `test_paths_simple_files` | Nested file create |
| test_paths.toml | `test_paths_absolute_files` | Absolute paths |
| test_paths.toml | `test_paths_absolute_dirs` | Absolute path dirs |

**Deferred** (Phase 07 or later): `test_dirs_nested`, `test_dirs_recursive_remove`, `test_dirs_kitty_seek`, `test_dirs_toot_seek`, `test_dirs_many_reentrant`, `test_dirs_file_reentrant`.

**Minimal set**: `test_dirs_one_mkdir`, `test_dirs_many_creation`, `test_dirs_many_removal`, `test_dirs_many_rename`. Start with creation; removal and rename follow.

---

## SPEC References

- CREATE: 0x401 LFS_TYPE_CREATE
- DELETE: 0x4ff LFS_TYPE_DELETE
- NAME: 0x0xx LFS_TYPE_NAME
- DIRSTRUCT: 0x200 LFS_TYPE_DIRSTRUCT
- TAIL: 0x600 SOFTTAIL, 0x601 HARDTAIL
- Commit layout: CRC tag, padding, FCRC (defer FCRC to Phase 06 if simpler)
- Block allocation: lookahead bitmap; lfs_alloc, lfs_alloc_scan, lfs_alloc_lookahead

---

## Validation

Before considering this phase complete:

1. **Build**: `cargo build -p lp-littlefs`
2. **Tests**: `cargo test -p lp-littlefs`
3. **Format**: `cargo fmt`
4. **Warnings**: Zero warnings
5. **Phase tests**: All ported dir and path tests in minimal set pass
