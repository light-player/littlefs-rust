# Phase 3: File Operations

## Goal

Implement `File<'a, S>` with open, read, write, seek, tell, size, sync, truncate, close, and `Drop`.

## Internal types

### FileAllocation

```rust
pub(crate) struct FileAllocation {
    file: MaybeUninit<lp_littlefs_core::LfsFile>,
    cache: Vec<u8>,
    file_config: lp_littlefs_core::LfsFileConfig,
}
```

Owns the core `LfsFile` state and its cache buffer. The `LfsFileConfig` wires the cache buffer pointer. This is heap-allocated via `Box` so it has a stable address — the core's mlist can safely point to `file`.

### File

```rust
pub struct File<'a, S: Storage> {
    fs: &'a Filesystem<S>,
    alloc: Box<FileAllocation>,
    closed: bool,
}
```

`'a` ties the file's lifetime to the filesystem. The borrow checker prevents files from outliving the filesystem.

## Open

```rust
impl<S: Storage> Filesystem<S> {
    pub fn open(&self, path: &str, flags: OpenFlags) -> Result<File<'_, S>, Error> {
        let mut alloc = Box::new(FileAllocation::new(self.cache_size()));
        // Wire alloc.file_config.buffer = alloc.cache.as_mut_ptr()
        let path_bytes = null_terminate(path);
        {
            let mut inner = self.inner.borrow_mut();
            let rc = lp_littlefs_core::lfs_file_opencfg(
                inner.lfs.as_mut_ptr(),
                alloc.file.as_mut_ptr(),
                path_bytes.as_ptr(),
                flags.bits() as i32,
                &alloc.file_config as *const _,
            );
            from_lfs_result(rc)?;
        } // borrow released
        Ok(File { fs: self, alloc, closed: false })
    }
}
```

Uses `lfs_file_opencfg` (not `lfs_file_open`) so the wrapper controls buffer allocation. The `RefCell` borrow is held only for the `lfs_file_opencfg` call.

## File methods

All methods follow the same borrow pattern:

```rust
impl<S: Storage> File<'_, S> {
    pub fn read(&self, buf: &mut [u8]) -> Result<u32, Error> {
        let mut inner = self.fs.inner.borrow_mut();
        let rc = lp_littlefs_core::lfs_file_read(
            inner.lfs.as_mut_ptr(),
            self.alloc.file.as_mut_ptr(),  // safe: Box is heap-stable
            buf.as_mut_ptr() as *mut c_void,
            buf.len() as u32,
        );
        drop(inner); // explicit release
        from_lfs_size(rc)
    }
}
```

Note: `self.alloc.file.as_mut_ptr()` through a shared `&self` is sound because:
1. `MaybeUninit::as_mut_ptr()` returns a raw pointer without requiring `&mut`
2. The core's mlist needs this pointer to be stable, which `Box` guarantees
3. Exclusive access to the core state is provided by the `RefCell` borrow

### Methods to implement

| Method | Core call | Returns |
|--------|-----------|---------|
| `read(&self, buf)` | `lfs_file_read` | `Result<u32, Error>` (bytes read) |
| `write(&self, data)` | `lfs_file_write` | `Result<u32, Error>` (bytes written) |
| `seek(&self, pos: SeekFrom)` | `lfs_file_seek` | `Result<u32, Error>` (new position) |
| `tell(&self)` | `lfs_file_tell` | `u32` |
| `size(&self)` | `lfs_file_size` | `u32` |
| `sync(&self)` | `lfs_file_sync` | `Result<(), Error>` |
| `truncate(&self, size: u32)` | `lfs_file_truncate` | `Result<(), Error>` |

### close

```rust
    pub fn close(mut self) -> Result<(), Error> {
        self.closed = true;
        let mut inner = self.fs.inner.borrow_mut();
        let rc = lp_littlefs_core::lfs_file_close(
            inner.lfs.as_mut_ptr(),
            self.alloc.file.as_mut_ptr(),
        );
        from_lfs_result(rc)
    }
```

Consumes `self`. Sets `closed = true` so `Drop` is a no-op.

### Drop

```rust
impl<S: Storage> Drop for File<'_, S> {
    fn drop(&mut self) {
        if !self.closed {
            if let Ok(mut inner) = self.fs.inner.try_borrow_mut() {
                let _ = lp_littlefs_core::lfs_file_close(
                    inner.lfs.as_mut_ptr(),
                    self.alloc.file.as_mut_ptr(),
                );
            }
        }
    }
}
```

## Path helper

```rust
fn null_terminate(s: &str) -> Vec<u8> {
    let mut v: Vec<u8> = s.bytes().collect();
    v.push(0);
    v
}
```

Used wherever the core expects a null-terminated `*const u8` path.

## Tests

### test_write_read_roundtrip

Format, mount, open file with `CREATE | WRITE`, write bytes, close, open with `READ`, read back, verify contents match.

### test_multiple_open_files

Open two files simultaneously, write to both, close both, read both back.

### test_seek_tell_size

Write data, seek to various positions, verify `tell()` and `size()` return correct values.

### test_truncate

Write data, truncate to shorter length, verify size changed, read back truncated content.

### test_file_drop_closes

Open file, write, drop without explicit close. Re-mount, read file — data should be present (close syncs on drop).

### test_close_returns_error_info

Verify that explicit `close()` returns `Ok(())` on success.

### test_open_nonexistent_fails

Open a path that doesn't exist without `CREATE` → `Error::NoEntry`.

## Validate

```bash
cargo test -p lp-littlefs
```
