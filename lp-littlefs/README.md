# lp-littlefs

Safe Rust API for the [LittleFS](https://github.com/littlefs-project/littlefs) embedded filesystem.

Built on [lp-littlefs-core](https://crates.io/crates/lp-littlefs-core), a function-by-function
Rust port of the C littlefs. **No C toolchain required** — pure Rust, builds on any target.

On-disk format is compatible with upstream LittleFS for interoperability.

## Quick start

```rust
use lp_littlefs::{Config, Filesystem, RamStorage};

let mut storage = RamStorage::new(512, 128);
let config = Config::new(512, 128);

Filesystem::format(&mut storage, &config).unwrap();
let fs = Filesystem::mount(storage, config).unwrap();

fs.write_file("/hello.txt", b"Hello, littlefs!").unwrap();
let data = fs.read_to_vec("/hello.txt").unwrap();
assert_eq!(data, b"Hello, littlefs!");

fs.unmount().unwrap();
```

## Usage

Implement the `Storage` trait for your block device:

```rust
use lp_littlefs::{Storage, Error};

struct MyFlash { /* ... */ }

impl Storage for MyFlash {
    fn read(&mut self, block: u32, offset: u32, buf: &mut [u8]) -> Result<(), Error> {
        // read from flash
        Ok(())
    }

    fn write(&mut self, block: u32, offset: u32, data: &[u8]) -> Result<(), Error> {
        // write to flash
        Ok(())
    }

    fn erase(&mut self, block: u32) -> Result<(), Error> {
        // erase block
        Ok(())
    }

    // sync() has a default no-op implementation
}
```

Then format, mount, and use the filesystem:

```rust
use lp_littlefs::{Config, Filesystem, OpenFlags};

let config = Config::new(4096, 256); // 4KB blocks, 256 blocks = 1MB

Filesystem::format(&mut flash, &config)?;
let fs = Filesystem::mount(flash, config)?;

// Convenience methods
fs.write_file("/data.bin", &payload)?;
let data = fs.read_to_vec("/data.bin")?;

// Directory operations
fs.mkdir("/logs")?;
for entry in fs.list_dir("/")? {
    println!("{} ({:?})", entry.name, entry.file_type);
}

// Fine-grained file access
let file = fs.open("/log.txt", OpenFlags::WRITE | OpenFlags::CREATE | OpenFlags::APPEND)?;
file.write(b"new log line\n")?;
file.close()?;

fs.unmount()?;
```

## Architecture

This crate is a safe wrapper around `lp-littlefs-core`, which is a faithful function-by-function
translation of the C littlefs. The wrapper handles all the unsafe ceremony: raw pointers,
`MaybeUninit`, integer error codes, null-terminated paths, and `unsafe extern "C"` callbacks.

### Interior mutability and multiple open files

`Filesystem` wraps its internal state in a `RefCell`. Each operation borrows the `RefCell` only for
the duration of one core call, then releases it immediately. This means multiple `File` and `ReadDir`
handles can coexist, and you can interleave file operations with directory iteration:

```rust
let mut dir = fs.read_dir("/")?;
while let Some(entry) = dir.next() {
    let entry = entry?;
    // This works — the RefCell borrow was released after dir.next()
    let data = fs.read_to_vec(&format!("/{}", entry.name))?;
}
```

### RAII close

`File` and `ReadDir` implement `Drop`, which closes the underlying handle automatically.
Use explicit `close()` when you need to handle errors from the close operation.

### Heap allocation

The crate requires `alloc`. Internal buffers (caches, lookahead) and file/directory allocations are
heap-allocated via `Box` and `Vec`. This provides stable addresses for the core's internal pointer
structures without requiring `Pin` or self-referential struct tricks.

## Running examples

From the workspace root:

```bash
cargo run -p lp-littlefs --example ram_hello
cargo run -p lp-littlefs --example ram_tree
```

`ram_hello` writes and reads a file; `ram_tree` demonstrates directory creation and listing.

## Feature flags

| Feature | Default | Description |
|---------|---------|-------------|
| `alloc` | yes | Required. Enables `Vec`, `Box`, and convenience methods. |
| `std`   | no  | Enables `std::error::Error` impl on `Error`. |
| `log`   | no  | Passes through to `lp-littlefs-core` for trace logging. |

## Prior art

- [littlefs2](https://crates.io/crates/littlefs2) — idiomatic Rust API wrapping the C library via FFI (requires C toolchain)
- [littlefs2-sys](https://crates.io/crates/littlefs2-sys) — low-level C bindings

## License

BSD-3-Clause (same as upstream littlefs)
