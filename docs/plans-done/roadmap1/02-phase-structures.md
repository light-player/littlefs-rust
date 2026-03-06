# Phase 2: Data Structures

Translate C structs, enums, typedefs, and macros to Rust. Identify and replace stdlib dependencies for `no_std`.

## Tasks

1. **Catalog types from lfs.h and lfs_util.h**
   - Structs: `lfs_t`, `lfs_cache_t`, `lfs_mdir_t`, `lfs_dir_t`, `lfs_file_t`, `lfs_config`, `lfs_info`, `lfs_attr`, `lfs_superblock_t`, `lfs_gstate_t`, `lfs_commit`, etc.
   - Enums: `lfs_error`, `lfs_type`, `lfs_open_flags`, `lfs_whence_flags`
   - Typedefs: `lfs_size_t`, `lfs_off_t`, `lfs_block_t`, etc.

2. **Identify stdlib usage (lfs_util.h)**
   - `memcpy`, `memset` → `core::ptr::copy_nonoverlapping`, `core::ptr::write_bytes`
   - `assert` → `debug_assert!` or panic
   - `malloc` (if LFS_MALLOC) → `alloc::alloc` or config-provided; `LFS_NO_MALLOC` for no_std
   - `lfs_crc` → port from lfs_util.c (CRC-32 table)

3. **Translate to Rust**
   - Use `#[repr(C)]` where layout must match on-disk or C FFI
   - Macros: `LFS_MKTAG`, `lfs_min`, `lfs_max`, `lfs_alignup`, `lfs_aligndown`, `lfs_tag_*` helpers
   - Constants: `LFS_BLOCK_NULL`, `LFS_BLOCK_INLINE`, `LFS_TYPE_*`, `LFS_ERR_*`, etc.

4. **File layout**
   - `src/types.rs` or split: `src/error.rs`, `src/config.rs`, `src/cache.rs`, etc.
   - Match logical groupings from lfs.h

## Success

- All types and constants compile
- `no_std` builds without std/alloc unless explicitly enabled
- Layout-critical structs have `#[repr(C)]` where required
