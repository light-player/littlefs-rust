# Phase 05: File Write (create, write, sync, truncate, append)

## Scope

Implement file creation, writing, sync, truncate, append. Handles inline ↔ CTZ migration when file grows/shrinks. Depends on Phase 03 (directory commits) and Phase 04 (CTZ read).

**Translation rules**: [docs/rules.md](../../rules.md). Translate callees first (§0); include C source comments (§3); match logic (§2). Preserve call graph (§8).

---

## API Targets

| API | Upstream | Description |
|-----|----------|-------------|
| `lfs_file_write(file, buf, size) -> i32` | lfs.h | Write bytes; buffered until sync |
| `lfs_file_sync(file) -> i32` | lfs.h | Flush writes to storage |
| `lfs_file_truncate(file, size) -> i32` | lfs.h | Truncate file |
| `lfs_file_rewind(file) -> i32` | lfs.h | Seek to start |

Open flags: WRONLY, RDWR, CREAT, EXCL, TRUNC, APPEND. Upstream `lfs_open_flags`.

---

## Tests to Port (All Relevant)

From littlefs-rust-old/test_files.rs. Same names per [rules.md §10](../../rules.md).

| Source | Test | Validates |
|--------|------|-----------|
| test_files.toml | `test_files_simple` | Create, write, close, mount, read |
| test_files.toml | `test_files_append` | APPEND flag |
| test_files.toml | `test_files_truncate` | TRUNC, then write |
| test_files.toml | `test_files_rewrite` | Overwrite file (different size) |
| test_files.toml | `test_files_many` | Many small files |
| test_files.toml | `test_files_many_power_cycle` | Mount/unmount between each file |
| test_files.toml | `test_rename_file_same_dir` | Rename file |
| test_files.toml | `test_fs_gc` | fs_gc (may need Phase 06) |
| test_files.toml | `test_truncate_simple` | Truncate to medium/large |
| test_files.toml | `test_seek_read` | Seek then read |
| test_files.toml | `test_truncate_read` | Truncate, read |
| test_files.toml | `test_truncate_write_read` | Truncate, write, read |
| test_files.toml | `test_truncate_write` | Truncate then write |
| test_files.toml | `test_seek_write` | Seek then write |
| test_files.toml | `test_seek_filemax` | file_max boundary |

**Deferred to Phase 06**: `test_files_reentrant_write`, `test_files_many_power_loss`, `test_truncate_reentrant_write`, `test_seek_reentrant_write`.

**Deferred** (edge cases): `test_truncate_aggressive`, `test_truncate_nop`, `test_seek_boundary_read`, `test_seek_boundary_write`, `test_seek_out_of_bounds`, `test_seek_inline_write`, `test_seek_underflow`, `test_seek_overflow`.

**Minimal set**: `test_files_simple`, `test_files_append`, `test_files_truncate`, `test_files_many`. Use narrow parameter ranges initially.

---

## SPEC References

- CTZ extend: lfs_ctz_extend; DESIGN.md CTZ skip-list append
- Inline ↔ CTZ: File grows beyond inline_max → outline to CTZ; shrinks → may inline
- Commit: Same as Phase 03; INLINESTRUCT/CTZSTRUCT updates

---

## Validation

Before considering this phase complete:

1. **Build**: `cargo build -p littlefs-rust`
2. **Tests**: `cargo test -p littlefs-rust`
3. **Format**: `cargo fmt`
4. **Warnings**: Zero warnings
5. **Phase tests**: Minimal set passes; expand ranges once stable
