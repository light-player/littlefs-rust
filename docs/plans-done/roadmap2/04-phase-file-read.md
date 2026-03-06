# Phase 04: File Read (inline + CTZ)

## Scope

Implement file reading for inline files (INLINESTRUCT) and CTZ skip-list files (CTZSTRUCT). Enables `lfs_file_open` (RDONLY), `lfs_file_read`, `lfs_file_seek`, `lfs_file_tell`, `lfs_file_size`, `lfs_file_close`.

**Translation rules**: [docs/rules.md](../../rules.md). Translate callees first (§0); include C source comments (§3); match logic (§2).

---

## API Targets

| API | Upstream | Description |
|-----|----------|-------------|
| `lfs_file_open(file, path, flags) -> i32` | lfs.h | Open file; flags RDONLY, CREAT, EXCL, etc. |
| `lfs_file_read(file, buf, size) -> i32` | lfs.h | Read bytes; 0 = EOF |
| `lfs_file_seek(file, off, whence) -> i32` | lfs.h | SEEK_SET, SEEK_CUR, SEEK_END |
| `lfs_file_tell(file) -> i64` | lfs.h | Current position |
| `lfs_file_size(file) -> lfs_ssize_t` | lfs.h | File size |
| `lfs_file_close(file) -> i32` | lfs.h | Close file |

File handle: `lfs_file_t`; may need block cache for file data.

---

## Tests to Port (All Relevant)

From lp-littlefs-old/test_files.rs, test_entries.rs. Same names per [rules.md §10](../../rules.md).

| Source | Test | Validates |
|--------|------|-----------|
| test_files.toml | `test_files_simple_read` | Read "Hello World!" (requires fs_with_hello) |
| test_files.toml | `test_files_seek_tell` | file_seek, file_tell, file_size, rewind |
| test_files.toml | `test_files_large` | Large file chunked read |
| test_entries.toml | `test_entries_grow` | Create file, grow, read (needs write; may split) |
| test_entries.toml | `test_entries_shrink` | Shrink, read |
| test_entries.toml | `test_entries_spill` | Metadata spill, read |
| test_entries.toml | `test_entries_push_spill` | Push causes spill |
| test_entries.toml | `test_entries_drop` | Drop/dir compaction |

**Note**: `test_files_simple_read` requires a filesystem with "hello" file. Either (a) include minimal `file_open(CREAT|WRONLY)` + `file_write` + `file_close` in this phase for setup, or (b) use format-from-C-then-read fixture for Phase 04 only.

**Minimal set**: `test_files_simple_read`, `test_files_seek_tell`. Add `test_files_large` (subset) once stable. Start with narrow parameter ranges per [rules.md §10](../../rules.md).

---

## SPEC References

- INLINESTRUCT: 0x201 LFS_TYPE_INLINESTRUCT
- CTZSTRUCT: 0x202 LFS_TYPE_CTZSTRUCT
- CTZ skip-list: reverse block layout, skip pointers
- REG: 0x001 LFS_TYPE_REG

---

## Validation

Before considering this phase complete:

1. **Build**: `cargo build -p lp-littlefs`
2. **Tests**: `cargo test -p lp-littlefs`
3. **Format**: `cargo fmt`
4. **Warnings**: Zero warnings
5. **Phase tests**: `test_files_simple_read`, `test_files_seek_tell` pass
