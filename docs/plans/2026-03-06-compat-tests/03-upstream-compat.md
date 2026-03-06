# Phase 3: Upstream Forward/Backward Compat Tests

## Goal

Implement the 14 upstream `test_compat` forward/backward tests from `test_compat.toml`, using `littlefs2-sys` as the "previous version" (`lfsp`) and `lp-littlefs` as the current version (`lfs`).

In upstream C, when `lfsp` is not a separate library, both sides alias to the same `lfs_*`. Here, each side is a genuinely different implementation, making these tests meaningful.

## Mapping

Upstream terminology → compat crate mapping:

| Upstream | Compat crate |
|----------|-------------|
| `lfsp_*` (previous version) | `c_impl::*` (littlefs2-sys) |
| `lfs_*` (current version) | `rust_impl::*` (lp-littlefs) |

Forward compat = C formats, Rust reads/writes.
Backward compat = Rust formats, C reads/writes.

## Forward compat tests (7)

### `test_compat_forward_mount`
- C: format + mount + unmount
- Rust: mount, `lfs_fs_stat` (check disk_version), unmount

### `test_compat_forward_read_dirs`
- C: format, mount, create 5 dirs (`"dir{i}"` for i=0..4), unmount
- Rust: mount, list root (expect 5 dirs), for each dir open + read (expect `.` and `..` only), unmount

### `test_compat_forward_read_files`
- C: format, mount, create 5 files with PRNG data (seed=i, size=SIZE, chunk=CHUNK), unmount
- Rust: mount, list root (expect 5 files), for each file read + verify PRNG content, unmount
- Parameterized: `SIZE=[4, 32, 512, 8192]`, `CHUNK=4`

### `test_compat_forward_read_files_in_dirs`
- C: format, mount, create 5 dirs, in each create a file with PRNG data, unmount
- Rust: mount, list root dirs, for each dir list files, read + verify content, unmount
- Parameterized: `SIZE=[4, 32, 512, 8192]`, `CHUNK=4`

### `test_compat_forward_write_dirs`
- C: format, mount, create dirs 0..4, unmount
- Rust: mount, create dirs 5..9, list root (expect 10 dirs), unmount

### `test_compat_forward_write_files`
- C: format, mount, create files 0..4 with PRNG data, unmount
- Rust: mount, create files 5..9 with PRNG data, list root (expect 10 files), read + verify all 10, unmount
- Parameterized: `SIZE=[4, 32, 512, 8192]`, `CHUNK=4`

### `test_compat_forward_write_files_in_dirs`
- C: format, mount, create dirs 0..4, file in each, unmount
- Rust: mount, create dirs 5..9, file in each, list + verify all 10 dirs + files, unmount
- Parameterized: `SIZE=[4, 32, 512, 8192]`, `CHUNK=4`

## Backward compat tests (7)

Same structure as forward, but reversed: Rust creates, C reads/writes.

### `test_compat_backward_mount`
- Rust: format + mount + unmount
- C: mount, `lfs_fs_stat` (check disk_version), unmount

### `test_compat_backward_read_dirs`
- Rust creates 5 dirs → C lists and verifies

### `test_compat_backward_read_files`
- Rust creates 5 files with PRNG data → C reads and verifies
- Parameterized: `SIZE=[4, 32, 512, 8192]`, `CHUNK=4`

### `test_compat_backward_read_files_in_dirs`
- Rust creates dirs + files → C reads and verifies
- Parameterized: `SIZE=[4, 32, 512, 8192]`, `CHUNK=4`

### `test_compat_backward_write_dirs`
- Rust creates dirs 0..4 → C creates dirs 5..9, lists all 10

### `test_compat_backward_write_files`
- Rust creates files 0..4 → C creates files 5..9, reads + verifies all 10
- Parameterized: `SIZE=[4, 32, 512, 8192]`, `CHUNK=4`

### `test_compat_backward_write_files_in_dirs`
- Rust creates dirs+files 0..4 → C creates dirs+files 5..9, verifies all
- Parameterized: `SIZE=[4, 32, 512, 8192]`, `CHUNK=4`

## Implementation

### Additional wrapper functions needed

Beyond phase 2's helpers, the upstream compat tests need:

| Function | Both sides |
|----------|-----------|
| `mount_create_dirs(names)` | Mount, mkdir for each name, unmount |
| `mount_create_files_prng(files, size, chunk, seed_base)` | Mount, create + write PRNG data, unmount |
| `mount_create_dirs_with_files_prng(...)` | Mount, mkdir + create file in each, unmount |
| `mount_verify_dirs(expected)` | Mount, list root, assert dirs match, unmount |
| `mount_verify_files_prng(files, size, chunk, seed_base)` | Mount, list root, read + verify PRNG, unmount |
| `mount_verify_dirs_with_files_prng(...)` | Mount, verify dir listing + file contents, unmount |
| `mount_fs_stat()` | Mount, `lfs_fs_stat`, return disk_version, unmount |

For the write tests, combined mount sessions are needed (e.g. "mount, create 5 more dirs, list all 10, unmount" in a single mount). This means the wrappers should also expose lower-level "already mounted" helpers, or the test itself can do the mount/unmount lifecycle directly.

### PRNG

Use the same `test_prng` xorshift32 as `lp-littlefs/tests/common/mod.rs`. Copy or factor out. Seed for file `i` is `i` (matching upstream).

### Parameterization

Use `#[rstest]` with `#[values]`:

```rust
#[rstest]
fn test_compat_forward_read_files(#[values(4, 32, 512, 8192)] size: u32) {
    const CHUNK: u32 = 4;
    // ...
}
```

## Effect on lp-littlefs/tests/test_compat.rs

After this phase, the 14 forward/backward stubs in `lp-littlefs/tests/test_compat.rs` should be removed (or replaced with a comment pointing to `lp-littlefs-compat`). The file retains only the 3 version edge cases (`major_incompat`, `minor_incompat`, `minor_bump`), which are implemented separately as part of test-parity2.

## Validate

```bash
cargo test -p lp-littlefs-compat test_compat
# 14 base cases + parameterized variants (10 tests have SIZE parameter = 4 variants each)
# Expected: ~54 test cases total (4 non-parameterized + 10 * 4 parameterized + 4 non-parameterized)

cargo test -p lp-littlefs-compat
# All compat + operation tests pass
```
