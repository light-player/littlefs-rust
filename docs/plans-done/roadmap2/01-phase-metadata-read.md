# Phase 01: Metadata Read (stat, dir_open, dir_read, dir_close)

## Scope

Implement read-only metadata traversal: parse commits and tags, resolve paths, expose `lfs_stat`, `lfs_dir_open` / `lfs_dir_read` / `lfs_dir_close`, and `lfs_fs_stat` (already present). No writes. Foundation for all subsequent phases.

**Translation rules**: [docs/rules.md](../../rules.md). Translate callees first (§0); include C source comments (§3); match logic and control flow (§2).

---

## API Targets

| API | Upstream | Description |
|-----|----------|-------------|
| `lfs_stat(path) -> i32` | lfs.h | File/dir info (type, size, name) into `lfs_info` |
| `lfs_dir_open(dir, path) -> i32` | lfs.h | Open directory for iteration |
| `lfs_dir_read(dir, info) -> i32` | lfs.h | Read next entry; 0 = end, 1 = entry |
| `lfs_dir_close(dir) -> i32` | lfs.h | Close directory |
| `lfs_fs_stat(info) -> i32` | lfs.h | Already implemented |

Types: `lfs_info` (type, size, name), `lfs_fsinfo`. Upstream lfs.h.

---

## Tests to Port (All Relevant)

Port from lp-littlefs-old and upstream. Keep same names per [rules.md §10](../../rules.md).

| Source | Test | Validates |
|--------|------|-----------|
| test_dirs.toml | `test_dirs_root` | dir_open("/"), dir_read returns ".", "..", then 0 |
| test_superblocks.toml | `test_superblocks_stat_tweaked` | fs_stat with custom name_max/file_max/attr_max (when format supports) |

**Minimal set to drive implementation**: `test_dirs_root`. Add `test_superblocks_stat_tweaked` when format accepts tweaked config.

---

## SPEC References

- Metadata pairs: revision count, commits, CRC, XOR tag encoding
- Metadata block fields: revision (LE), commits, CRC
- Tag format: valid bit, type3, id, length; big-endian
- Tag types: CREATE, DELETE, NAME, SUPERBLOCK, REG, DIR, STRUCT, TAIL, USERATTR, CRC
- Superblock: 0x0ff LFS_TYPE_SUPERBLOCK, magic "littlefs"
- Root directory: last metadata pair in superblock chain is root

---

## Validation

Before considering this phase complete:

1. **Build**: `cargo build -p lp-littlefs`
2. **Tests**: `cargo test -p lp-littlefs`
3. **Format**: `cargo fmt`
4. **Warnings**: Zero warnings; fix or document any
5. **Phase tests**: `test_dirs_root` passes; `test_superblocks_stat` still passes
