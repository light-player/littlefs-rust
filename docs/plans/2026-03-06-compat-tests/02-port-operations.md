# Phase 2: Port Operation-Level Tests

## Goal

Implement `c_impl` and `rust_impl` wrapper modules using the current APIs, then port the existing `lp-littlefs-c-align` operation-level tests.

## Steps

### 1. c_impl.rs

Port from `lp-littlefs-c-align/src/c_lfs.rs`. Each function calls `littlefs2_sys::lfs_*` and returns `Result<T, i32>`. Functions to implement:

| Function | What it does |
|----------|-------------|
| `format` | `lfs_format` + `lfs_mount` + `lfs_unmount` |
| `mount_dir_names(path)` | Mount, list dir entries (skip `.`/`..`), unmount |
| `mount_read_file(path)` | Mount, open, read full content, close, unmount |
| `format_mkdir_unmount(name)` | Format, mount, mkdir, unmount |
| `format_mkdir_file_unmount(dir, file)` | Format, mount, mkdir, create file, unmount |
| `format_file_mkdir_unmount(file, dir)` | Same but reversed order |
| `format_create_three_unmount()` | Format, create "aaa", "zzz", "mmm" |
| `format_create_rename_unmount(old, new)` | Format, create, rename, unmount |
| `format_create_remove_unmount(path)` | Format, create, remove, unmount |
| `format_create_write_unmount(path, data)` | Format, create, write content, close, unmount |
| `format_nested_dir_file_unmount(parent, child, file)` | Format, mkdir chain, create file, unmount |
| `format_mkdir_file_rmdir_unmount(dir, file)` | Format, mkdir, create, remove file, rmdir, unmount |
| `mount_mkdir_expect_exist(path)` | Mount, mkdir, expect `LFS_ERR_EXIST` |

The code will be structurally identical to the current `c_lfs.rs` — it's already using `littlefs2_sys` directly. The only change is that `SharedStorage` config builder replaces `AlignStorage::build_lfs_config`.

### 2. rust_impl.rs

Port from `lp-littlefs-c-align/src/rust_lfs.rs`. Same function set as `c_impl`, but calls `lp_littlefs::lfs_*` (C-style API with raw pointers, `*const u8` paths, `i32` returns). This is a significant rewrite since the old code used the now-removed high-level wrapper.

Pattern for each function:
```rust
pub fn format(storage: &SharedStorage, geo: &TestGeometry) -> Result<(), i32> {
    let mut env = storage.build_rust_env(geo);
    let mut lfs = MaybeUninit::<lp_littlefs::Lfs>::zeroed();
    // set config.context to &storage
    env.config.context = storage as *const SharedStorage as *mut c_void;

    let err = lp_littlefs::lfs_format(lfs.as_mut_ptr(), &env.config);
    if err != 0 { return Err(err); }

    let err = lp_littlefs::lfs_mount(lfs.as_mut_ptr(), &env.config);
    if err != 0 { return Err(err); }

    let err = lp_littlefs::lfs_unmount(lfs.as_mut_ptr());
    if err != 0 { return Err(err); }

    Ok(())
}
```

Paths need null termination (`CString` or manual `push(0)`).

### 3. test_operations.rs

Port all tests from `lp-littlefs-c-align/tests/align_tests.rs`. Each test:
1. Creates `SharedStorage` + `TestGeometry` (default geometry)
2. Calls `c_impl::*` or `rust_impl::*`
3. Asserts results

Tests to port (preserving existing `#[ignore]` annotations where bugs remain):

| Test | Direction | Checks |
|------|-----------|--------|
| `c_format_rust_mount_root` | C → Rust | Rust mounts C-formatted empty FS |
| `rust_format_c_mount_root` | Rust → C | C mounts Rust-formatted empty FS |
| `c_mkdir_file_rust_sees_both` | C → Rust | Rust sees both entries |
| `rust_mkdir_file_c_sees_both` | Rust → C | C sees both entries |
| `c_mkdir_file_rust_sees_both_reverse_order` | C → Rust | Order independence |
| `c_insert_before_rust_sees_all` | C → Rust | Insert-before ordering |
| `rust_insert_before_c_sees_all` | Rust → C | Insert-before ordering |
| `c_mkdir_remount_exist` | C → C | Remount persistence |
| `c_rename_rust_sees_new_name` | C → Rust | Rename visibility |
| `rust_rename_c_sees_new_name` | Rust → C | (may still be ignored) |
| `c_remove_rust_sees_gone` | C → Rust | Remove visibility |
| `rust_remove_c_sees_gone` | Rust → C | Remove visibility |
| `c_write_rust_reads_content_inline` | C → Rust | Inline file content |
| `rust_write_c_reads_content_inline` | Rust → C | Inline file content |
| `c_write_rust_reads_content_ctz` | C → Rust | CTZ file content |
| `rust_write_c_reads_content_ctz` | Rust → C | (may still be ignored) |
| `c_nested_dir_rust_sees` | C → Rust | Nested dir hierarchy |
| `rust_nested_dir_c_sees` | Rust → C | Nested dir hierarchy |
| `c_rmdir_rust_sees_gone` | C → Rust | rmdir persistence |
| `rust_rmdir_c_sees_gone` | Rust → C | (may still be ignored) |
| `format_layout_prog_size_16` | Rust → C | prog_size=16 compat |
| `format_layout_prog_size_64` | C → Rust | prog_size=64 compat |

Re-evaluate the `#[ignore]` annotations — some bugs from the old c-align era may now be fixed.

## Validate

```bash
cargo test -p lp-littlefs-core-compat test_operations
# All non-ignored tests pass
```
