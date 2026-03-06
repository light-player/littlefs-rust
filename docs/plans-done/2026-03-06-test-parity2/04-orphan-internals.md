# Phase 4: Orphan Test Internal APIs

## Scope

Expose internal dir APIs via `#[cfg(test)]` for orphan tests. Write test bodies for 3 stubbed tests. Unblocks 3 tests.

## Approach

The orphan tests (`test_orphans_one_orphan`, `test_orphans_mkconsistent_one_orphan`) need to call internal functions to create orphan entries. The upstream C tests use `in = 'lfs.c'` to get access to static functions. In Rust, we use `#[cfg(test)] pub` re-exports.

`test_orphans_normal` needs raw block corruption via `write_block_raw` + BD callbacks to corrupt a dir's most recent commit.

## Internal APIs to expose

### Already implemented, need `#[cfg(test)]` pub re-export

These functions exist internally. Add `#[cfg(test)]` public re-exports from `src/lib.rs` or a `src/test_api.rs` module:

| Function | Location | Used by |
|----------|----------|---------|
| `lfs_dir_alloc` | `src/dir/commit.rs` | one_orphan, mkconsistent_one_orphan |
| `lfs_dir_commit` | `src/dir/commit.rs` | all 3 orphan tests (with SOFTTAIL tag) |
| `lfs_dir_fetch` | `src/dir/fetch.rs` | all 3 orphan tests |
| `lfs_alloc_ckpoint` | `src/block_alloc/alloc.rs` | one_orphan, mkconsistent_one_orphan |
| `lfs_fs_preporphans` | `src/fs/` | all 3 orphan tests |
| `lfs_gstate_hasorphans` | `src/gstate.rs` or similar | all 3 orphan tests |
| `lfs_pair_tole32` | `src/types.rs` or similar | one_orphan, mkconsistent_one_orphan |
| `LFS_TYPE_SOFTTAIL` | `src/lfs_type.rs` | one_orphan, mkconsistent_one_orphan |
| `lfs_mktag` / `LFS_MKATTRS` equiv | `src/types.rs` | one_orphan, mkconsistent_one_orphan |

### Test helper module

Create `src/test_internals.rs` (gated behind `#[cfg(test)]`):

```rust
#[cfg(test)]
pub mod test_internals {
    pub use crate::dir::commit::{lfs_dir_alloc, lfs_dir_commit};
    pub use crate::dir::fetch::lfs_dir_fetch;
    pub use crate::block_alloc::alloc::lfs_alloc_ckpoint;
    // ... etc
}
```

Or add `#[cfg(test)]` pub to the existing functions. Prefer whichever is less invasive; the key constraint is that integration tests (`tests/test_orphans.rs`) can import these.

**Note:** Integration tests can only access `pub` items from the crate's public API. So the re-exports must be `pub` at the crate root level, gated by `#[cfg(test)]`.

## Test implementations

### test_orphans_normal

Reference: `test_orphans.toml:1-60`

1. Format, mount, mkdir `parent`, `parent/orphan`, `parent/child`, remove `parent/orphan`, unmount
2. Mount, open `parent/child`, get `dir.m.pair[0]` (the block number)
3. Read the entire block via BD read callback
4. Find the end of valid data (scan backwards past ERASE_VALUE bytes)
5. Corrupt the last 3 bytes of the commit (overwrite CRC) with `BLOCK_SIZE`
6. Erase and reprogram the block
7. Mount — `parent/orphan` should be `LFS_ERR_NOENT`, `parent/child` should exist, `lfs_fs_size` = 8
8. Mount again — mkdir `parent/otherchild` should trigger deorphan, `lfs_fs_size` still 8

Guard: `if PROG_SIZE <= 0x3fe` (only works with one CRC per commit)

### test_orphans_one_orphan

Reference: `test_orphans.toml:92-127`

1. Format, mount
2. `lfs_alloc_ckpoint`, `lfs_dir_alloc` to create an orphan mdir
3. `lfs_dir_commit` the orphan (empty attrs)
4. `lfs_fs_preporphans(+1)` to mark FS as having orphans
5. `lfs_dir_fetch` root `[0, 1]`, `lfs_pair_tole32(orphan.pair)`
6. `lfs_dir_commit` root with `LFS_MKATTRS({SOFTTAIL, 0x3ff, 8}, orphan.pair)`
7. Assert `lfs_gstate_hasorphans`
8. Unmount, mount, assert hasorphans
9. `lfs_fs_forceconsistency` — assert no more orphans
10. Unmount

### test_orphans_mkconsistent_one_orphan

Reference: `test_orphans.toml:164-204`

Same as `test_orphans_one_orphan` but uses `lfs_fs_mkconsistent` instead of `lfs_fs_forceconsistency`, then remounts to verify orphans are still gone.

## Validate

```bash
cargo test -p littlefs-rust-core test_orphans_normal
cargo test -p littlefs-rust-core test_orphans_one_orphan
cargo test -p littlefs-rust-core test_orphans_mkconsistent_one_orphan
```
