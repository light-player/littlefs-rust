# Phase 02: Block device caching

## Scope of phase

Add read and program caches to the block device layer. All format/mount/file/dir operations use cached reads and program-through-cache where applicable. Matches upstream `lfs_bd_read`, `lfs_bd_prog`, `lfs_bd_cmp`, `lfs_bd_crc` behavior.

Refer to the C implementation /Users/yona/dev/photomancer/oss/littlefs/lfs.c for the implementation details,
match the C implementation as closely as possible while keeping the Rust code clean and idiomatic.

## API targets

No new public API. Internal:

- Read cache: block, offset, size; bypass for large aligned reads (hint-based)
- Program cache: block, offset, size; flush on sync/eviction
- Config: `cache_size`, `read_buffer`, `prog_buffer` (optional static buffers)

**Upstream**: lfs.c `lfs_bd_*` (lines 44–276), `lfs_cache_t` (lfs.h:370–375), `lfs_config` cache_size, read_buffer, prog_buffer (lfs.h:218–247).

## Upstream tests to port

No dedicated cache test. Use existing `test_superblocks_format`, `test_superblocks_mount`, `test_superblocks_magic` — they should pass unchanged. Phase validates by not regressing.

## SPEC references

- Block reads/programs: DESIGN.md block device semantics; SPEC.md read/prog alignment.

## Code organization

- Caching layer between LittleFs and BlockDevice
- Prefer granular modules (e.g. `cache.rs` or under `block/`)
