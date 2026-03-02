# Phase 05: File write (create, write, sync, truncate, append)

## Scope of phase

Implement file creation, writing, sync, truncate, append. Handles inline ↔ CTZ migration when file grows/shrinks. Depends on phase 03 (directory commits) and phase 04 (CTZ read).

Refer to the C implementation /Users/yona/dev/photomancer/oss/littlefs/lfs.c for the implementation details,
match the C implementation as closely as possible while keeping the Rust code clean and idiomatic.

## API targets

| API                                              | Upstream                        | Description                      |
| ------------------------------------------------ | ------------------------------- | -------------------------------- |
| `file_write(file, data) -> Result<usize, Error>` | `lfs_file_write` (lfs.h:456)    | Write bytes; buffered until sync |
| `file_sync(file) -> Result<(), Error>`           | `lfs_file_sync` (lfs.h:441)     | Flush writes to storage          |
| `file_truncate(file, size) -> Result<(), Error>` | `lfs_file_truncate` (lfs.h:466) | Truncate file                    |
| `file_rewind(file) -> Result<(), Error>`         | `lfs_file_rewind` (lfs.h:469)   | Seek to start                    |

Open flags used: WRONLY, RDWR, CREAT, EXCL, TRUNC, APPEND. Upstream `lfs_open_flags` (lfs.h:126–154).

## Upstream tests to port

| Source                  | Case                          | Validates                         |
| ----------------------- | ----------------------------- | --------------------------------- |
| `tests/test_files.toml` | `test_files_simple`           | Create, write, close, mount, read |
| `tests/test_files.toml` | `test_files_large`            | Chunked write, read               |
| `tests/test_files.toml` | `test_files_rewrite`          | Overwrite file (different size)   |
| `tests/test_files.toml` | `test_files_append`           | APPEND flag                       |
| `tests/test_files.toml` | `test_files_truncate`         | TRUNC, then write                 |
| `tests/test_files.toml` | `test_files_many`             | Many small files                  |
| `tests/test_files.toml` | `test_files_many_power_cycle` | Mount/unmount between each file   |

**Minimal set**: `test_files_simple`, `test_files_append`, `test_files_truncate`, `test_files_many`. Defer `test_files_reentrant_write`, `test_files_reentrant_write_sync`, `test_files_many_power_loss` to phase 06 (power-loss).

## SPEC references

- **CTZ extend**: lfs.c `lfs_ctz_extend`; DESIGN.md CTZ skip-list append
- **CTZ traverse**: For truncate/relocate; SPEC.md CTZ layout
- **Inline ↔ CTZ**: File grows beyond inline_max → outline to CTZ; shrinks → may inline
- **Commit**: Same as phase 03; INLINESTRUCT/CTZSTRUCT updates in directory commits

## Code organization

- `lfs_ctz_extend` logic for appending blocks
- Truncate: traverse and free blocks; update struct tag
- File cache/prog buffer handling consistent with block cache

## Validation

Run `just fci` to format, fix, and validate the code before committing.
