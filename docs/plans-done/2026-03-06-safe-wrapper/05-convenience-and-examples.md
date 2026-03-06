# Phase 5: Convenience Methods and Examples

## Goal

Add high-level convenience methods (`read_to_vec`, `write_file`, `fs_size`, `gc`) and two runnable examples demonstrating the API.

## Convenience methods

### read_to_vec

```rust
impl<S: Storage> Filesystem<S> {
    pub fn read_to_vec(&self, path: &str) -> Result<Vec<u8>, Error> {
        let file = self.open(path, OpenFlags::READ)?;
        let size = file.size() as usize;
        let mut buf = vec![0u8; size];
        let n = file.read(&mut buf)?;
        buf.truncate(n as usize);
        // file closed via Drop
        Ok(buf)
    }
}
```

### write_file

```rust
    pub fn write_file(&self, path: &str, data: &[u8]) -> Result<(), Error> {
        let file = self.open(path, OpenFlags::WRITE | OpenFlags::CREATE | OpenFlags::TRUNC)?;
        let mut offset = 0;
        while offset < data.len() {
            let n = file.write(&data[offset..])? as usize;
            offset += n;
        }
        // file closed via Drop
        Ok(())
    }
```

Writes all data. Loops in case of partial writes (shouldn't happen in practice with littlefs, but defensive).

### fs_size

```rust
    pub fn fs_size(&self) -> Result<u32, Error> {
        let mut inner = self.inner.borrow_mut();
        let rc = littlefs_rust_core::lfs_fs_size(inner.lfs.as_mut_ptr());
        from_lfs_size(rc)
    }
```

### gc

```rust
    pub fn gc(&self) -> Result<(), Error> {
        let mut inner = self.inner.borrow_mut();
        let rc = littlefs_rust_core::lfs_fs_gc(inner.lfs.as_mut_ptr());
        from_lfs_result(rc)
    }
```

## Examples

### examples/ram_hello.rs

Minimal example: format, mount, write a file, read it back, print contents.

```rust
use littlefs_rust::{Config, Filesystem, OpenFlags, RamStorage};

fn main() {
    let block_size = 512;
    let block_count = 128;
    let mut storage = RamStorage::new(block_size, block_count);
    let config = Config::new(block_size, block_count);

    Filesystem::format(&mut storage, &config).expect("format failed");
    let fs = Filesystem::mount(storage, config).expect("mount failed");

    fs.write_file("/hello.txt", b"Hello, littlefs!").expect("write failed");

    let data = fs.read_to_vec("/hello.txt").expect("read failed");
    println!("{}", core::str::from_utf8(&data).unwrap());

    let storage = fs.unmount().expect("unmount failed");
    println!("Used {} bytes of storage", storage.data().len());
}
```

### examples/ram_tree.rs

Directory operations: create a tree, list it, rename, remove.

```rust
use littlefs_rust::{Config, FileType, Filesystem, RamStorage};

fn main() {
    let mut storage = RamStorage::new(512, 128);
    let config = Config::new(512, 128);

    Filesystem::format(&mut storage, &config).expect("format failed");
    let fs = Filesystem::mount(storage, config).expect("mount failed");

    // Build a directory tree
    fs.mkdir("/docs").expect("mkdir docs");
    fs.mkdir("/docs/drafts").expect("mkdir drafts");
    fs.write_file("/docs/readme.txt", b"Read me").expect("write readme");
    fs.write_file("/docs/drafts/notes.txt", b"Draft notes").expect("write notes");

    // List root
    println!("/ contents:");
    for entry in fs.list_dir("/").expect("list /") {
        println!("  {} ({:?}, {} bytes)", entry.name, entry.file_type, entry.size);
    }

    // List nested dir
    println!("\n/docs contents:");
    for entry in fs.list_dir("/docs").expect("list /docs") {
        println!("  {} ({:?}, {} bytes)", entry.name, entry.file_type, entry.size);
    }

    // Rename
    fs.rename("/docs/readme.txt", "/docs/README.txt").expect("rename");
    println!("\nAfter rename, /docs contains:");
    for entry in fs.list_dir("/docs").expect("list /docs") {
        println!("  {}", entry.name);
    }

    // Stat
    let meta = fs.stat("/docs/README.txt").expect("stat");
    println!("\n/docs/README.txt: {:?}, {} bytes", meta.file_type, meta.size);

    // Remove
    fs.remove("/docs/drafts/notes.txt").expect("remove file");
    fs.remove("/docs/drafts").expect("remove dir");
    println!("\nAfter removing drafts, /docs contains:");
    for entry in fs.list_dir("/docs").expect("list /docs") {
        println!("  {}", entry.name);
    }

    // Filesystem info
    let size = fs.fs_size().expect("fs_size");
    println!("\nFilesystem uses {} blocks", size);

    fs.unmount().expect("unmount");
}
```

## Tests

### test_read_to_vec

Write known data, read back with `read_to_vec`, verify match.

### test_read_to_vec_empty

Read an empty file, verify returns empty vec.

### test_write_file_overwrites

Write data, then write different data to same path, verify only new data present.

### test_write_file_creates_new

Write to a path that doesn't exist, verify file created.

### test_fs_size

Format and mount, check `fs_size` returns a small value. Write files, verify `fs_size` increases.

## Validate

```bash
cargo test -p littlefs-rust
cargo run -p littlefs-rust --example ram_hello
cargo run -p littlefs-rust --example ram_tree
```
