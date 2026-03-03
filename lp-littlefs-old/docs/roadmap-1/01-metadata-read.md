# Phase 01: Metadata read (stat, dir_read, fs_stat)

## Scope of phase

Implement read-only metadata traversal: parse commits and tags, resolve paths, expose `stat`, `dir_open`/`dir_read`/`dir_close`, and `fs_stat`. No writes. Foundation for all subsequent phases.

## API targets

| API | Upstream | Description |
|-----|----------|-------------|
| `stat(path) -> Result<Info, Error>` | `lfs_stat` (lfs.h:407) | File/dir info (type, size, name) |
| `dir_open(path) -> Result<(), Error>` | `lfs_dir_open` (lfs.h:467) | Open directory for iteration |
| `dir_read(dir, info) -> Result<u32, Error>` | `lfs_dir_read` (lfs.h:477) | Read next entry; 0 = end |
| `dir_close(dir) -> Result<(), Error>` | `lfs_dir_close` (lfs.h:470) | Close directory |
| `fs_stat() -> Result<FsInfo, Error>` | `lfs_fs_stat` (lfs.h:500) | Disk version, block_size, block_count, name_max, file_max, attr_max |

**Types**: `Info` (type, size, name), `FsInfo` (lfs_fsinfo). Upstream `lfs.h` lines 295–329.

## Upstream tests to port

| Source | Case | Validates |
|--------|------|-----------|
| `tests/test_superblocks.toml` | `test_superblocks_stat` | fs_stat after format/mount |
| `tests/test_superblocks.toml` | `test_superblocks_stat_tweaked` | fs_stat with custom name_max/file_max/attr_max |
| `tests/test_dirs.toml` | `test_dirs_root` | dir_open("/"), dir_read returns ".", "..", then 0 |
| `tests/test_entries.toml` | (read-only dir iteration after format) | dir_read order |

**Minimal set**: `test_superblocks_stat`, `test_dirs_root`. Add `test_superblocks_stat_tweaked` when format supports tweaked config.

## SPEC references

- **Metadata pairs**: SPEC.md "Directories / Metadata pairs" — revision count, commits, CRC, XOR tag encoding
- **Metadata blocks**: SPEC.md "Metadata block fields" — revision (LE), commits, CRC (0x04c11db7, init 0xffffffff)
- **Tag format**: SPEC.md "Metadata tags" — valid bit, type3, id, length; tags stored **big-endian**
- **Tag types**: SPEC.md "Metadata types" — CREATE, DELETE, NAME, SUPERBLOCK, REG, DIR, STRUCT (DIRSTRUCT, INLINESTRUCT, CTZSTRUCT), TAIL (SOFTTAIL, HARDTAIL), USERATTR, CRC
- **Superblock**: SPEC.md "0x0ff LFS_TYPE_SUPERBLOCK" — magic "littlefs", struct at offset 8
- **Root directory**: DESIGN.md — last metadata pair in superblock chain is root; DIRSTRUCT for subdirs

## Code organization

- One concept per file; metadata parsing in dedicated module(s)
- Entry points and tests first; helpers at bottom
- Related functionality grouped
- Temporary code marked with TODO
