# Phase 04: File read (inline + CTZ)

## Scope of phase

Implement file reading for inline files (INLINESTRUCT) and CTZ skip-list files (CTZSTRUCT). Enables `file_open` (RDONLY), `file_read`, `file_seek`, `file_tell`, `file_size`, `file_close`.

Refer to the C implementation /Users/yona/dev/photomancer/oss/littlefs/lfs.c for the implementation details,
match the C implementation as closely as possible while keeping the Rust code clean and idiomatic.

## API targets

| API                                                  | Upstream                       | Description                                |
| ---------------------------------------------------- | ------------------------------ | ------------------------------------------ |
| `file_open(path, flags) -> Result<(), Error>`        | `lfs_file_opencfg` (lfs.h:437) | Open file; flags RDONLY, CREAT, EXCL, etc. |
| `file_read(file, buf) -> Result<usize, Error>`       | `lfs_file_read` (lfs.h:449)    | Read bytes; 0 = EOF                        |
| `file_seek(file, off, whence) -> Result<i64, Error>` | `lfs_file_seek` (lfs.h:455)    | SEEK_SET, SEEK_CUR, SEEK_END               |
| `file_tell(file) -> Result<i64, Error>`              | `lfs_file_tell` (lfs.h:467)    | Current position                           |
| `file_size(file) -> Result<i64, Error>`              | `lfs_file_size` (lfs.h:472)    | File size                                  |
| `file_close(file) -> Result<(), Error>`              | `lfs_file_close` (lfs.h:433)   | Close file                                 |

**File handle**: Needs block cache for file data (per-file or shared). Upstream `lfs_file_t` (lfs.h:402–416).

## Upstream tests to port

| Source                        | Case                              | Validates                                                                                                   |
| ----------------------------- | --------------------------------- | ----------------------------------------------------------------------------------------------------------- |
| `tests/test_files.toml`       | `test_files_simple`               | Create+write "Hello World!", mount, read back                                                               |
| `tests/test_files.toml`       | `test_files_large`                | Large file (32–262144 bytes), chunked write/read                                                            |
| `tests/test_superblocks.toml` | `test_superblocks_unknown_blocks` | Includes file create/write/read after mount with block_count=0 — **skip block_count=0**, use core file part |
| `tests/test_seek.toml`        | (basic seek cases)                | file_seek, file_tell, file_size                                                                             |

**Minimal set**: `test_files_simple`, `test_files_large` (subset of SIZE/CHUNKSIZE). Note: `test_files_simple` requires file create+write. Either (a) include minimal `file_open(CREAT|EXCL|WRONLY)` + `file_write` + `file_close` in this phase to enable the test, or (b) defer `test_files_simple` to phase 05 and use format-from-C-then-read for phase 04 validation.

## SPEC references

- **INLINESTRUCT**: SPEC.md "0x201 LFS_TYPE_INLINESTRUCT" — data in metadata tag
- **CTZSTRUCT**: SPEC.md "0x202 LFS_TYPE_CTZSTRUCT" — head block, file size
- **CTZ skip-list**: SPEC.md "CTZ skip-lists", DESIGN.md "CTZ skip-lists" — reverse block layout, skip pointers
- **REG**: SPEC.md "0x001 LFS_TYPE_REG" — regular file entry

## Code organization

- CTZ traversal (`lfs_ctz_find`, `lfs_ctz_index`) in dedicated module
- Inline read in metadata/dir layer
