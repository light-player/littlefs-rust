# Phase 1: Add bd_crc to bdcache

## Scope of phase

Add a cached CRC function to the block device cache layer. Per C's `lfs_bd_crc` (lfs.c:155): reads a region through pcache/rcache and accumulates CRC-32. Used by FCRC (in dir_commit_crc) and post-commit verification.

## Code organization reminders

- Place helper utility functions at the bottom of files
- Keep related functionality grouped together

## Implementation details

### 1. Add bd_crc to bdcache.rs

```rust
/// Cached CRC of a block region. Per lfs_bd_crc.
/// Reads through pcache/rcache, accumulates CRC-32 (poly 0x04c11db7, init 0xffffffff).
pub fn bd_crc<B: BlockDevice>(
    bd: &B,
    config: &Config,
    rcache: &RefCell<ReadCache>,
    pcache: &RefCell<ProgCache>,
    block: u32,
    off: u32,
    size: usize,
    init: u32,
) -> Result<u32, Error>
```

- Loop: read chunks via `bd_read`, accumulate with `crc::crc32(c, &chunk)`
- Chunk size: use a small buffer (e.g. 64 bytes) to avoid large allocs
- Return final CRC value

### 2. Expose on BdContext

```rust
pub fn crc(&self, block: u32, off: u32, size: usize, init: u32) -> Result<u32, Error>
```

### 3. Verify CRC behavior

- `crc::crc32(0xffff_ffff, &[]).unwrap()` or similar — ensure we use the same polynomial/init as C (0x04c11db7, 0xffffffff). The `crc` crate's `Crc32` with IEEE polynomial matches.

## Validate

```bash
cd lp-littlefs && cargo test
cargo fmt
```
