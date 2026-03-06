# Phase 1: Crate Scaffolding

## Goal

Create the `lp-littlefs` crate with foundational types: `Storage` trait, `Config`, `Error`, `RamStorage`. No filesystem operations yet — just the types that everything else builds on.

## Steps

### 1. Create crate and add to workspace

Create `lp-littlefs/` with `Cargo.toml`:

```toml
[package]
name = "lp-littlefs"
version = "0.1.0"
edition = "2021"
description = "Safe Rust API for the LittleFS embedded filesystem"
license = "BSD-3-Clause"
repository = "https://github.com/light-player/lp-littlefs"
readme = "README.md"

[lib]
name = "lp_littlefs"
path = "src/lib.rs"

[dependencies]
lp-littlefs-core = { path = "../lp-littlefs-core" }
bitflags = "2"

[dev-dependencies]
env_logger = "0.10"

[features]
default = ["alloc"]
alloc = ["lp-littlefs-core/alloc"]
std = ["alloc"]
log = ["lp-littlefs-core/log"]
```

Add to workspace root `Cargo.toml`:

```toml
[workspace]
resolver = "2"
members = ["lp-littlefs-core", "lp-littlefs", "lp-littlefs-compat"]
```

### 2. Error enum (src/error.rs)

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Error {
    Io,
    Corrupt,
    NoEntry,
    Exists,
    NotDir,
    IsDir,
    NotEmpty,
    Invalid,
    NoSpace,
    NoMemory,
    NoAttribute,
    NameTooLong,
}
```

- `impl Display for Error`
- `impl From<i32> for Error` — maps `LFS_ERR_*` constants; panics on unknown codes (internal bug, not user-facing)
- `fn from_lfs_result(code: i32) -> Result<(), Error>` — helper: 0 → `Ok(())`, negative → `Err`
- `fn from_lfs_size(code: i32) -> Result<u32, Error>` — helper: non-negative → `Ok(n as u32)`, negative → `Err`
- `#[cfg(feature = "std")] impl std::error::Error for Error`

### 3. Storage trait (src/storage.rs)

```rust
pub trait Storage {
    fn read(&mut self, block: u32, offset: u32, buf: &mut [u8]) -> Result<(), Error>;
    fn write(&mut self, block: u32, offset: u32, data: &[u8]) -> Result<(), Error>;
    fn erase(&mut self, block: u32) -> Result<(), Error>;
    fn sync(&mut self) -> Result<(), Error> { Ok(()) }
}
```

### 4. Config struct (src/config.rs)

```rust
pub struct Config {
    pub block_size: u32,
    pub block_count: u32,
    pub read_size: u32,
    pub prog_size: u32,
    pub block_cycles: i32,
    pub cache_size: u32,
    pub lookahead_size: u32,
    pub name_max: u32,
    pub file_max: u32,
    pub attr_max: u32,
}

impl Config {
    pub fn new(block_size: u32, block_count: u32) -> Self {
        Self {
            block_size,
            block_count,
            read_size: 16,
            prog_size: 16,
            block_cycles: -1,
            cache_size: 0,       // 0 = use block_size
            lookahead_size: 0,   // 0 = use block_size
            name_max: 255,
            file_max: i32::MAX as u32,
            attr_max: 1022,
        }
    }
}
```

Add an internal `resolve_cache_size(&self) -> u32` method that returns `cache_size` if non-zero, else `block_size` (same for `lookahead_size`). Used when building `LfsConfig` in phase 2.

### 5. Metadata types (src/metadata.rs)

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileType {
    File,
    Dir,
}

pub struct Metadata {
    pub file_type: FileType,
    pub size: u32,
    pub name: String,
}

pub struct DirEntry {
    pub name: String,
    pub file_type: FileType,
    pub size: u32,
}
```

OpenFlags and SeekFrom:

```rust
bitflags! {
    pub struct OpenFlags: u32 {
        const READ    = 0x1;
        const WRITE   = 0x2;
        const CREATE  = 0x100;
        const EXCL    = 0x200;
        const TRUNC   = 0x400;
        const APPEND  = 0x800;
    }
}

pub enum SeekFrom {
    Start(u32),
    Current(i32),
    End(i32),
}
```

### 6. RamStorage (src/ram.rs)

```rust
pub struct RamStorage {
    data: Vec<u8>,
    block_size: u32,
    block_count: u32,
}

impl RamStorage {
    pub fn new(block_size: u32, block_count: u32) -> Self;
    pub fn data(&self) -> &[u8];
}

impl Storage for RamStorage { ... }
```

Erase fills with `0xFF`. Read/write copy to/from the backing `Vec`. Bounds-checked.

### 7. README.md (lp-littlefs/README.md)

This is the crate-level README, rendered on crates.io via `readme = "README.md"` in `Cargo.toml`. It should contain:

- **What it is** — safe Rust API over a pure-Rust littlefs implementation; no C toolchain needed
- **Quick usage example** — format, mount, write, read (the `ram_hello` example from phase 5)
- **Architecture overview** — `lp-littlefs-core` (faithful C port) vs `lp-littlefs` (safe wrapper); `Storage` trait for plugging in block devices
- **Design notes** — `RefCell` interior mutability, borrow-per-call pattern, why multiple open files work, RAII close via `Drop`
- **Feature flags** — `alloc` (default), `std`, `log`
- **Relationship to upstream** — on-disk format compatible with C littlefs, link to upstream repo and spec

Pull the relevant content from `docs/plans/2026-03-06-safe-wrapper/00-design.md`. Keep it focused on what a user of the crate needs to know, not implementation phase details.

### 8. lib.rs

Crate-level doc comment (`//!`) with a one-paragraph synopsis and a short usage example. Use `#![doc = include_str!("../README.md")]` if the README is concise enough, otherwise write a shorter `//!` block and let the README carry the full story.

```rust
//! Safe Rust API for the LittleFS embedded filesystem.
//!
//! Built on [`lp-littlefs-core`], a function-by-function Rust port of the
//! [C littlefs](https://github.com/littlefs-project/littlefs). No C toolchain required.
//!
//! # Quick start
//!
//! ```rust
//! use lp_littlefs::{Config, Filesystem, RamStorage};
//!
//! let mut storage = RamStorage::new(512, 128);
//! let config = Config::new(512, 128);
//!
//! Filesystem::format(&mut storage, &config).unwrap();
//! let fs = Filesystem::mount(storage, config).unwrap();
//!
//! fs.write_file("/hello.txt", b"Hello, littlefs!").unwrap();
//! let data = fs.read_to_vec("/hello.txt").unwrap();
//! assert_eq!(data, b"Hello, littlefs!");
//!
//! fs.unmount().unwrap();
//! ```

extern crate alloc;

mod config;
mod error;
mod metadata;
mod ram;
mod storage;

pub use config::Config;
pub use error::Error;
pub use metadata::{DirEntry, FileType, Metadata, OpenFlags, SeekFrom};
pub use ram::RamStorage;
pub use storage::Storage;
```

Filesystem, File, ReadDir modules are stubbed as empty — filled in phases 2–4.

## Validate

```bash
cargo build -p lp-littlefs
cargo test -p lp-littlefs   # no tests yet, just compiles
```
