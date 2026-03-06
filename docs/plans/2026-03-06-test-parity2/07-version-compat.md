# Phase 7: Version Compatibility Tests

## Scope

Implement all 17 `test_compat` cases. This is the largest phase. Unblocks 17 tests.

## Background

The upstream `test_compat.toml` tests three things:

1. **Forward compatibility** (7 tests): Format with a "previous" version (`lfsp`), mount/read/write with the current version (`lfs`)
2. **Backward compatibility** (7 tests): Format with current `lfs`, mount/read/write with `lfsp`
3. **Version edge cases** (3 tests): Tamper with superblock version fields, verify mount accepts/rejects correctly

### Upstream self-test mode

The C code handles the case where `lfsp` is not a separate library by aliasing `lfsp_*` to `lfs_*`. In this mode, both "versions" are the same implementation — the tests still exercise format/mount/read/write across mount cycles, they just don't test actual cross-version behavior. The comment in the TOML says:

> If lfsp is not linked, and LFSP is not defined, these tests will alias the relevant lfs types/functions as necessary so at least the tests can themselves be tested locally.

We will use this same approach: `lfsp_*` operations use the same `lp_littlefs` crate. This validates the test logic and verifies that format-mount-read/write works end-to-end. Real cross-version testing can be added later by linking `lp-littlefs-c-align` or a previous-version crate as `lfsp`.

## Implementation

### 1. Test helper aliases

In `test_compat.rs`, define type aliases and wrapper functions so the test code reads naturally:

```rust
use lp_littlefs::{
    Lfs, LfsConfig, LfsDir, LfsFile, LfsInfo,
    lfs_format, lfs_mount, lfs_unmount,
    lfs_mkdir, lfs_dir_open, lfs_dir_read, lfs_dir_close,
    lfs_file_open, lfs_file_write, lfs_file_read, lfs_file_seek, lfs_file_close,
    lfs_fs_stat,
    LFS_O_RDONLY, LFS_O_WRONLY, LFS_O_CREAT, LFS_O_EXCL, LFS_SEEK_SET,
    LFS_TYPE_REG, LFS_TYPE_DIR,
};

// "Previous version" aliases — same implementation in self-test mode
type LfspT = Lfs;
fn lfsp_format(lfs: *mut LfspT, cfg: *const LfsConfig) -> i32 { lfs_format(lfs, cfg) }
fn lfsp_mount(lfs: *mut LfspT, cfg: *const LfsConfig) -> i32 { lfs_mount(lfs, cfg) }
// ... etc for all lfsp_* functions
```

Also define:

```rust
const LFSP_DISK_VERSION: u32 = LFS_DISK_VERSION;
const LFSP_DISK_VERSION_MAJOR: u32 = LFS_DISK_VERSION_MAJOR;
```

### 2. Forward compat tests (7 cases)

All guarded by `LFS_DISK_VERSION_MAJOR == LFSP_DISK_VERSION_MAJOR` (always true in self-test mode).

| Case | C reference | Summary |
|------|-------------|---------|
| `forward_mount` | toml:62-89 | `lfsp_format` + `lfsp_mount` + `lfsp_unmount`, then `lfs_mount` + `lfs_fs_stat` (check disk_version) + `lfs_unmount` |
| `forward_read_dirs` | toml:92-147 | `lfsp` creates 5 dirs, `lfs` lists them |
| `forward_read_files` | toml:150-237 | `lfsp` creates 5 files with PRNG data (SIZE=[4,32,512,8192], CHUNK=4), `lfs` lists + reads + verifies |
| `forward_read_files_in_dirs` | toml:240-354 | `lfsp` creates dirs+files, `lfs` lists + reads |
| `forward_write_dirs` | toml:357-419 | `lfsp` creates 5 dirs, `lfs` creates 5 more, lists all 10 |
| `forward_write_files` | toml:422-541 | `lfsp` writes first half of files, `lfs` writes second half, reads all |
| `forward_write_files_in_dirs` | toml:544-690 | Same with nested dirs |

### 3. Backward compat tests (7 cases)

All guarded by `LFS_DISK_VERSION == LFSP_DISK_VERSION` (always true in self-test mode).

Same pattern as forward but reversed: `lfs` creates, `lfsp` reads/writes.

### 4. Version edge cases (3 cases)

These require internal APIs to tamper with the superblock version field.

| Case | Needs | Summary |
|------|-------|---------|
| `major_incompat` | `lfs_dir_fetch`, `lfs_dir_commit`, `LfsSuperblock`, `lfs_superblock_tole32` | Bump major version +1, expect mount → `LFS_ERR_INVAL` |
| `minor_incompat` | same | Bump minor version +1, expect mount → `LFS_ERR_INVAL` |
| `minor_bump` | same + `lfs_fs_stat` | Downgrade minor, mount works, read works, write triggers minor bump |

These use the same `#[cfg(test)]` pub re-exports from Phase 4. Additional APIs needed:

| Function | Location |
|----------|----------|
| `LfsSuperblock` | `src/lfs_superblock.rs` |
| `lfs_superblock_tole32` | `src/lfs_superblock.rs` |
| `lfs_dir_fetch` | `src/dir/fetch.rs` (already needed by Phase 4) |
| `lfs_dir_commit` | `src/dir/commit.rs` (already needed by Phase 4) |

### 5. Parameterization

The file-based tests have `defines.SIZE = [4, 32, 512, 8192]` and `defines.CHUNK = 2 or 4`. Use `#[rstest]` with `#[values]`:

```rust
#[rstest]
fn test_compat_forward_read_files(
    #[values(4, 32, 512, 8192)] size: u32,
) {
    const CHUNK: u32 = 4;
    // ...
}
```

## Dependencies

- Phase 4 (orphan internals) must be done first — shares the `#[cfg(test)]` re-export mechanism
- Phase 2 (infrastructure from old plan) — `test_prng`, `write_prng_file`, `verify_prng_file` are used in file-based compat tests

## Tests unblocked

All 17 `test_compat_*` tests. Replace `#[ignore = "stub: requires version compat infrastructure"]` and `todo!()` with implementations.

## Validate

```bash
cargo test -p lp-littlefs-core test_compat
# All 17+ (parameterized) cases should pass

# Verify no stubs remain
cargo test -p lp-littlefs-core -- --list --ignored 2>&1 | grep -c "compat"
# Expected: 0
```
