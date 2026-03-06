# littlefs-rust-core

Pure Rust LittleFS implementation, translated function-by-function from
the [reference C source](https://github.com/littlefs-project/littlefs). On-disk format compatible
with upstream LittleFS v2.

`#![no_std]` by default. No C dependencies, no bindgen, no cross-compilation toolchain required.

**Most users should use [`littlefs-rust`](../littlefs-rust/) instead**, which provides a safe Rust API
on top of this core. Use this crate directly only if you need the low-level C-style API for FFI
interop, custom wrappers, or testing.

## API

The public API mirrors the C littlefs function signatures: raw pointers, `i32` return codes,
`unsafe extern "C"` callbacks. This is intentional — it keeps the translation verifiable against
upstream and makes porting bug fixes straightforward.

### Filesystem lifecycle

```rust
lfs_format(lfs: * mut Lfs, config: * const LfsConfig) -> i32
lfs_mount(lfs: * mut Lfs, config: * const LfsConfig) -> i32
lfs_unmount(lfs: * mut Lfs) -> i32
```

### File operations

```rust
lfs_file_open(lfs, file, path, flags) -> i32
lfs_file_opencfg(lfs, file, path, flags, config) -> i32
lfs_file_close(lfs, file) -> i32
lfs_file_read(lfs, file, buffer, size) -> i32   // bytes read or negative error
lfs_file_write(lfs, file, buffer, size) -> i32   // bytes written or negative error
lfs_file_seek(lfs, file, off, whence) -> i32
lfs_file_tell(lfs, file) -> i32
lfs_file_size(lfs, file) -> i32
lfs_file_sync(lfs, file) -> i32
lfs_file_truncate(lfs, file, size) -> i32
lfs_file_rewind(lfs, file) -> i32
```

### Directory operations

```rust
lfs_mkdir(lfs, path) -> i32
lfs_dir_open(lfs, dir, path) -> i32
lfs_dir_close(lfs, dir) -> i32
lfs_dir_read(lfs, dir, info) -> i32
lfs_dir_seek(lfs, dir, off) -> i32
lfs_dir_tell(lfs, dir) -> i32
lfs_dir_rewind(lfs, dir) -> i32
```

### Path operations

```rust
lfs_remove(lfs, path) -> i32
lfs_rename(lfs, oldpath, newpath) -> i32
lfs_stat(lfs, path, info) -> i32
lfs_getattr(lfs, path, type , buffer, size) -> i32
lfs_setattr(lfs, path, type , buffer, size) -> i32
lfs_removeattr(lfs, path, type ) -> i32
```

### Filesystem-level

```rust
lfs_fs_stat(lfs, fsinfo) -> i32
lfs_fs_size(lfs) -> i32
lfs_fs_traverse(lfs, cb, data) -> i32
lfs_fs_mkconsistent(lfs) -> i32
lfs_fs_gc(lfs) -> i32
lfs_fs_grow(lfs, block_count) -> i32
```

## Block device configuration

`LfsConfig` carries block device callbacks and geometry parameters:

```rust
LfsConfig {
context: * mut c_void,       // user data passed to callbacks
read:  Option<lfs_read_t>,  // (cfg, block, off, buf, size) -> i32
prog:  Option<lfs_prog_t>,  // (cfg, block, off, buf, size) -> i32
erase: Option<lfs_erase_t>, // (cfg, block) -> i32
sync:  Option<lfs_sync_t>,  // (cfg) -> i32
block_size, block_count, read_size, prog_size,
cache_size, lookahead_size, block_cycles,
name_max, file_max, attr_max, metadata_max, inline_max,
read_buffer, prog_buffer, lookahead_buffer,
compact_thresh,
}
```

## Error handling

Functions return `0` on success, negative `i32` on error, or positive values for byte counts. Error
codes match the C littlefs constants (`LFS_ERR_IO`, `LFS_ERR_CORRUPT`, `LFS_ERR_NOENT`, etc.).

## Module structure

| Module           | Responsibility                                                                           |
|------------------|------------------------------------------------------------------------------------------|
| `bd`             | Block device read/prog/erase/sync with read and program caching                          |
| `block_alloc`    | Block allocator using a lookahead bitmap                                                 |
| `crc`            | CRC-32 (polynomial `0x04c11db7`, 16-entry lookup table matching `lfs_util.c`)            |
| `dir`            | Metadata pair operations: commit, fetch, find, open, traverse                            |
| `file`           | File I/O and the CTZ skip-list data structure                                            |
| `fs`             | High-level filesystem: format, mount, mkdir, remove, rename, stat, attrs, grow, traverse |
| `tag`            | Tag encoding/decoding for the metadata log                                               |
| `lfs_config`     | Block device configuration struct and callback types                                     |
| `lfs_gstate`     | Global state tracking (orphans, in-progress moves)                                       |
| `lfs_info`       | `LfsInfo`, `LfsAttr`, `LfsFileConfig`, `LfsFsinfo`                                       |
| `lfs_superblock` | Superblock read/write                                                                    |
| `lfs_type`       | Tag type constants                                                                       |
| `types`          | Primitive type aliases (`lfs_block_t`, `lfs_size_t`, etc.)                               |

## Feature flags

| Feature        | Default | Description                                                                |
|----------------|---------|----------------------------------------------------------------------------|
| `alloc`        | yes     | Heap allocation; enables `lfs_file_open`                                   |
| `loop_limits`  | yes     | Iteration caps to detect infinite loops in mount, traverse, fetch, commit  |
| `std`          | no      | Standard library support                                                   |
| `log`          | no      | Logging via the `log` crate (`RUST_LOG=littlefs_rust_core=trace`)            |
| `readonly`     | no      | Omit all write operations                                                  |
| `no_malloc`    | no      | Disable `lfs_file_open`; only `lfs_file_opencfg` (caller-provided buffers) |
| `multiversion` | no      | On-disk version selection via `disk_version` in config                     |
| `shrink`       | no      | Filesystem shrink support in `lfs_fs_grow`                                 |
| `slow_tests`   | no      | Power-loss and long-running tests (for CI nightly)                         |

Without `alloc`, the crate is fully `no_std` + `no_alloc`. Use `lfs_file_opencfg` with
caller-provided buffers in this mode.

## Tests

Integration tests cover: allocation, attributes, bad blocks, directories, file operations, entries,
path handling, seek, truncation, moves, orphans, relocations, superblocks, storage exhaustion,
interspersed operations, evil/corruption scenarios, and power loss.

Test infrastructure (`tests/common/`):

- **`RamStorage`** — in-memory block device
- **`BadBlockRamStorage`** — configurable bad-block simulation
- **`WearLevelingBd`** — per-block erase cycle tracking
- **`PowerLossCtx`** — write-count based power-loss injection (`Noop` and `Ooo` behaviors)
- Deterministic PRNG matching the C test suite for reproducible data generation

```bash
cargo test -p littlefs-rust-core
cargo test -p littlefs-rust-core --features slow_tests  # includes power-loss tests
```

## Translation approach

The codebase is a function-by-function translation of the reference `lfs.c`, preserving the original
control flow, data structures, and assertions. Original C line references are included as comments.
`unsafe` is used where the C code uses pointer arithmetic and raw buffer access. See `docs/rules.md`
for the full translation rules.
