# Phase 4: Directory and Path Operations

## Goal

Implement `ReadDir<'a, S>` (iterator), `DirEntry`, and path operations: `mkdir`, `remove`, `rename`, `stat`, `exists`, `read_dir`, `list_dir`.

## Internal types

### DirAllocation

```rust
pub(crate) struct DirAllocation {
    dir: MaybeUninit<littlefs_rust_core::LfsDir>,
}
```

Simpler than `FileAllocation` — directories don't need a separate cache buffer. Heap-allocated via `Box` for stable mlist pointers.

### ReadDir

```rust
pub struct ReadDir<'a, S: Storage> {
    fs: &'a Filesystem<S>,
    alloc: Box<DirAllocation>,
    closed: bool,
}
```

Same lifetime pattern as `File`.

## ReadDir as Iterator

```rust
impl<S: Storage> Iterator for ReadDir<'_, S> {
    type Item = Result<DirEntry, Error>;

    fn next(&mut self) -> Option<Self::Item> {
        let mut inner = self.fs.inner.borrow_mut();
        let mut info = MaybeUninit::<littlefs_rust_core::LfsInfo>::zeroed();
        let rc = littlefs_rust_core::lfs_dir_read(
            inner.lfs.as_mut_ptr(),
            self.alloc.dir.as_mut_ptr(),
            info.as_mut_ptr(),
        );
        drop(inner); // release borrow before processing result

        match rc {
            0 => None, // end of directory
            n if n < 0 => Some(Err(Error::from(n))),
            _ => {
                let info = unsafe { info.assume_init() };
                let entry = dir_entry_from_info(&info);
                // Skip "." and ".."
                if entry.name == "." || entry.name == ".." {
                    self.next() // recurse to skip
                } else {
                    Some(Ok(entry))
                }
            }
        }
    }
}
```

Borrow is held only for the `lfs_dir_read` call. Between iterations, the `RefCell` is free. This means callers can interleave directory iteration with file operations:

```rust
let mut dir = fs.read_dir("/")?;
while let Some(entry) = dir.next() {
    let entry = entry?;
    let data = fs.read_to_vec(&entry.name)?; // no conflict
}
```

### close and Drop

Same pattern as `File`:

```rust
impl<S: Storage> ReadDir<'_, S> {
    pub fn close(mut self) -> Result<(), Error> {
        self.closed = true;
        let mut inner = self.fs.inner.borrow_mut();
        let rc = littlefs_rust_core::lfs_dir_close(
            inner.lfs.as_mut_ptr(),
            self.alloc.dir.as_mut_ptr(),
        );
        from_lfs_result(rc)
    }
}

impl<S: Storage> Drop for ReadDir<'_, S> {
    fn drop(&mut self) {
        if !self.closed {
            if let Ok(mut inner) = self.fs.inner.try_borrow_mut() {
                let _ = littlefs_rust_core::lfs_dir_close(
                    inner.lfs.as_mut_ptr(),
                    self.alloc.dir.as_mut_ptr(),
                );
            }
        }
    }
}
```

## Filesystem methods

### read_dir

```rust
impl<S: Storage> Filesystem<S> {
    pub fn read_dir(&self, path: &str) -> Result<ReadDir<'_, S>, Error> {
        let mut alloc = Box::new(DirAllocation::new());
        let path_bytes = null_terminate(path);
        {
            let mut inner = self.inner.borrow_mut();
            let rc = littlefs_rust_core::lfs_dir_open(
                inner.lfs.as_mut_ptr(),
                alloc.dir.as_mut_ptr(),
                path_bytes.as_ptr(),
            );
            from_lfs_result(rc)?;
        }
        Ok(ReadDir { fs: self, alloc, closed: false })
    }
}
```

### list_dir

```rust
    pub fn list_dir(&self, path: &str) -> Result<Vec<DirEntry>, Error> {
        let dir = self.read_dir(path)?;
        dir.collect()
    }
```

Convenience method. Opens, collects all entries, closes (via `Drop` when the iterator is consumed).

### mkdir

```rust
    pub fn mkdir(&self, path: &str) -> Result<(), Error> {
        let path_bytes = null_terminate(path);
        let mut inner = self.inner.borrow_mut();
        let rc = littlefs_rust_core::lfs_mkdir(inner.lfs.as_mut_ptr(), path_bytes.as_ptr());
        from_lfs_result(rc)
    }
```

### remove

```rust
    pub fn remove(&self, path: &str) -> Result<(), Error> {
        let path_bytes = null_terminate(path);
        let mut inner = self.inner.borrow_mut();
        let rc = littlefs_rust_core::lfs_remove(inner.lfs.as_mut_ptr(), path_bytes.as_ptr());
        from_lfs_result(rc)
    }
```

### rename

```rust
    pub fn rename(&self, from: &str, to: &str) -> Result<(), Error> {
        let from_bytes = null_terminate(from);
        let to_bytes = null_terminate(to);
        let mut inner = self.inner.borrow_mut();
        let rc = littlefs_rust_core::lfs_rename(
            inner.lfs.as_mut_ptr(),
            from_bytes.as_ptr(),
            to_bytes.as_ptr(),
        );
        from_lfs_result(rc)
    }
```

### stat

```rust
    pub fn stat(&self, path: &str) -> Result<Metadata, Error> {
        let path_bytes = null_terminate(path);
        let mut info = MaybeUninit::<littlefs_rust_core::LfsInfo>::zeroed();
        let mut inner = self.inner.borrow_mut();
        let rc = littlefs_rust_core::lfs_stat(
            inner.lfs.as_mut_ptr(),
            path_bytes.as_ptr(),
            info.as_mut_ptr(),
        );
        drop(inner);
        from_lfs_result(rc)?;
        Ok(metadata_from_info(unsafe { &info.assume_init() }))
    }
```

### exists

```rust
    pub fn exists(&self, path: &str) -> bool {
        self.stat(path).is_ok()
    }
```

## Helper functions

```rust
fn dir_entry_from_info(info: &littlefs_rust_core::LfsInfo) -> DirEntry {
    let nul = info.name.iter().position(|&b| b == 0).unwrap_or(info.name.len());
    let name = core::str::from_utf8(&info.name[..nul])
        .unwrap_or("")
        .to_string();
    let file_type = if info.type_ == littlefs_rust_core::lfs_type::LFS_TYPE_DIR {
        FileType::Dir
    } else {
        FileType::File
    };
    DirEntry { name, file_type, size: info.size }
}

fn metadata_from_info(info: &littlefs_rust_core::LfsInfo) -> Metadata {
    let entry = dir_entry_from_info(info);
    Metadata { name: entry.name, file_type: entry.file_type, size: entry.size }
}
```

## Tests

### test_mkdir_and_list

Create directories, list root, verify entries.

### test_remove_file

Create file, remove it, verify `exists` returns false.

### test_remove_dir

Create dir, remove it, verify gone.

### test_rename

Create file, rename, verify old path gone, new path exists with same content.

### test_stat

Create file with known data, stat it, verify type and size.

### test_exists

Verify `exists` returns true for existing paths, false for nonexistent.

### test_read_dir_iterator

Create several files and dirs, iterate with `read_dir`, verify all entries found, `.` and `..` filtered.

### test_list_dir_convenience

Same as above but using `list_dir` convenience method.

### test_read_dir_interleaved_with_file_ops

Open a directory iterator, read a file during iteration, verify no panic.

### test_nested_dirs

Create nested directory structure, list recursively (manually via nested `read_dir` calls).

## Validate

```bash
cargo test -p littlefs-rust
```
