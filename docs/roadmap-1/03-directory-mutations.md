# Phase 03: Directory mutations (mkdir, remove, rename)

## Scope of phase

Implement directory creation, removal, and rename. Requires block allocation (lookahead), directory commit machinery (CREATE, DELETE, NAME, DIRSTRUCT, TAIL tags), and path resolution for writes.

Refer to the C implementation /Users/yona/dev/photomancer/oss/littlefs/lfs.c for the implementation details,
match the C implementation as closely as possible while keeping the Rust code clean and idiomatic.

Implement module-level tests for individual functions to validate compliance with source material,
in addition to the integration tests.

## API targets

| API                                     | Upstream                 | Description              |
| --------------------------------------- | ------------------------ | ------------------------ |
| `mkdir(path) -> Result<(), Error>`      | `lfs_mkdir` (lfs.h:439)  | Create directory         |
| `remove(path) -> Result<(), Error>`     | `lfs_remove` (lfs.h:402) | Remove file or empty dir |
| `rename(old, new) -> Result<(), Error>` | `lfs_rename` (lfs.h:411) | Move/rename file or dir  |

**Config additions**: `block_cycles`, `lookahead_size`, `lookahead_buffer` (optional).

## Upstream tests to port

| Source                    | Case                      | Validates                                                |
| ------------------------- | ------------------------- | -------------------------------------------------------- |
| `tests/test_dirs.toml`    | `test_dirs_root`          | dir_read (prereq from phase 01)                          |
| `tests/test_dirs.toml`    | `test_dirs_many_creation` | mkdir N dirs, dir_read lists them                        |
| `tests/test_dirs.toml`    | `test_dirs_many_removal`  | mkdir N, remove all, dir_read empty                      |
| `tests/test_dirs.toml`    | `test_dirs_many_rename`   | mkdir N, rename all, verify                              |
| `tests/test_entries.toml` | `test_entries_grow`       | Create files, grow one (requires file write — may split) |

**Minimal set**: `test_dirs_many_creation`, `test_dirs_many_removal`, `test_dirs_many_rename`. Start with creation; removal and rename follow.

## SPEC references

- **CREATE**: SPEC.md "0x401 LFS_TYPE_CREATE"
- **DELETE**: SPEC.md "0x4ff LFS_TYPE_DELETE"
- **NAME**: SPEC.md "0x0xx LFS_TYPE_NAME" — file type, name string
- **DIRSTRUCT**: SPEC.md "0x200 LFS_TYPE_DIRSTRUCT" — metadata pair pointer
- **TAIL**: SPEC.md "0x600 SOFTTAIL", "0x601 HARDTAIL" — next metadata pair
- **Commit layout**: SPEC.md — CRC tag, padding to prog_size, FCRC for next commit (defer FCRC to phase 06 if simpler)
- **Block allocation**: DESIGN.md — lookahead bitmap; lfs.c `lfs_alloc`, `lfs_alloc_scan`, `lfs_alloc_lookahead`

## Code organization

- Directory commit, compact, split in dedicated module(s)
- Allocation (lookahead) separate from metadata

## Validation

Run `just fci` to format, fix, and validate the code before committing.
