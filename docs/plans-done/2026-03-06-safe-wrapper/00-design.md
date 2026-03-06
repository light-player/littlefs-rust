# Safe Wrapper — littlefs-rust

Add a safe, idiomatic Rust wrapper crate (`littlefs-rust`) on top of the C-faithful core (
`littlefs-rust-core`). Primary consumer is the LightPlayer ESP32 firmware. Secondary audience is the
open-source embedded Rust community.

## Motivation

`littlefs-rust-core` is a function-by-function port of the C littlefs. Its API is deliberately C-like:
raw pointers, `MaybeUninit`, integer error codes, `unsafe extern "C"` callbacks. This is the right
design for the core — it keeps the translation verifiable against upstream and makes porting bug
fixes straightforward.

But application code shouldn't interact with this API directly. The LightPlayer firmware needs a
safe filesystem interface (`LpFs` trait) backed by littlefs on flash. The broader Rust embedded
community needs something they can drop into a project without writing `unsafe` boilerplate.

A wrapper crate in this repo provides both, while keeping the core pristine.

## Current State

- `littlefs-rust-core/` — C-faithful port, passing upstream test suite
- `littlefs-rust-compat/` — C ↔ Rust interop tests using `littlefs2-sys`
- No safe wrapper exists yet

## Design

### Repo structure

```
(workspace root)
├── Cargo.toml                        # members: littlefs-rust-core, littlefs-rust, littlefs-rust-compat
├── littlefs-rust-core/                 # unchanged — faithful C port
├── littlefs-rust/                      # new: safe wrapper
│   ├── Cargo.toml
│   ├── src/
│   │   ├── lib.rs
│   │   ├── storage.rs                # Storage trait
│   │   ├── config.rs                 # Config struct
│   │   ├── error.rs                  # Error enum
│   │   ├── filesystem.rs             # Filesystem<S>
│   │   ├── file.rs                   # File<'a, S>
│   │   ├── dir.rs                    # ReadDir<'a, S>, DirEntry
│   │   ├── metadata.rs               # Metadata, FileType, OpenFlags
│   │   └── ram.rs                    # RamStorage for tests/examples
│   ├── tests/
│   │   └── integration.rs
│   └── examples/
│       ├── ram_hello.rs              # format, write, read
│       └── ram_tree.rs               # mkdir, list, remove
├── littlefs-rust-compat/              # unchanged
└── ...
```

### Core architecture: RefCell + borrow-per-call

The wrapper uses interior mutability (`RefCell`) so that `Filesystem` methods take `&self`. Each
operation borrows the `RefCell` only for the duration of that single core call, then releases it
immediately. This enables multiple open files, interleaved file and directory operations, and
reading files while iterating directories — all without conflict.

```
Filesystem<S: Storage>
├── inner: RefCell<FsInner<S>>
│   └── FsInner { lfs: Lfs, config: LfsConfig, storage: S, caches... }
│
├── open(path, flags) -> File<'_, S>
├── read_dir(path) -> ReadDir<'_, S>
├── read_to_vec(path) -> Vec<u8>
├── write_file(path, data) -> ()
├── ...

File<'a, S: Storage>
├── fs: &'a Filesystem<S>           # shared borrow — ties lifetime to filesystem
├── alloc: Box<FileAllocation>       # owned, heap-stable (mlist can point here)
├── closed: bool
├── read/write/seek/tell/size/sync/truncate
└── Drop calls close (best-effort)

ReadDir<'a, S: Storage>
├── fs: &'a Filesystem<S>
├── alloc: Box<DirAllocation>
├── implements Iterator<Item = Result<DirEntry, Error>>
└── Drop calls dir_close (best-effort)
```

**Why this works for multiple open files:** `File` holds `&'a Filesystem<S>` (shared borrow), not
`&mut`. Multiple `File`s and `ReadDir`s can coexist because they all share the filesystem reference.
Each individual operation (`read`, `write`, `next`) does `self.fs.inner.borrow_mut()` for the
duration of one core call, then drops the borrow. Between calls the `RefCell` is free.

**Why there is no re-entrancy panic:** User code runs between borrows, never during. `DirEntry` is
fully owned (`String` name, `FileType`, `u32` size) — no borrows leak from the core. The only way to
trigger a panic would be a `Storage` implementation that called back into the `Filesystem`, which is
a bug in the storage impl.

**RAII close:** `File` and `ReadDir` implement `Drop`, which calls close with `try_borrow_mut` (
never panics). Explicit `close()` is available when the caller wants to handle errors.

**Lifetime safety:** `File<'a, S>` borrows `&'a Filesystem<S>`, so the compiler guarantees all files
and dirs are dropped before the filesystem. No dangling mlist pointers.

### Storage trait

```rust
pub trait Storage {
    fn read(&mut self, block: u32, offset: u32, buf: &mut [u8]) -> Result<(), Error>;
    fn write(&mut self, block: u32, offset: u32, data: &[u8]) -> Result<(), Error>;
    fn erase(&mut self, block: u32) -> Result<(), Error>;
    fn sync(&mut self) -> Result<(), Error> { Ok(()) }
}
```

Block/offset model matches how flash hardware works. No typenum, no generic-array. Default `sync`
returns `Ok(())`.

Internally, the wrapper generates `unsafe extern "C" fn` trampolines that bridge `Storage` trait
calls to `LfsConfig` callbacks. The `LfsConfig.context` pointer points to the `Storage` inside
`FsInner`.

### Config

```rust
pub struct Config {
    pub block_size: u32,
    pub block_count: u32,
    pub read_size: u32,        // default: 16
    pub prog_size: u32,        // default: 16
    pub block_cycles: i32,     // default: -1 (disabled)
    pub cache_size: u32,       // default: 0 (= block_size)
    pub lookahead_size: u32,   // default: 0 (= block_size)
    pub name_max: u32,         // default: 255
    pub file_max: u32,         // default: i32::MAX
    pub attr_max: u32,         // default: 1022
}

impl Config {
    pub fn new(block_size: u32, block_count: u32) -> Self;
}
```

`new()` fills sensible defaults. Users override fields directly. No builder pattern needed.

### Error

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

1:1 mapping from `LFS_ERR_*`. Implements `Display`. Implements `std::error::Error` behind `std`
feature.

### Public API surface

**Filesystem lifecycle:**

- `Filesystem::format(storage, config)` — format a device (borrows storage)
- `Filesystem::mount(storage, config)` — mount, takes ownership of storage
- `Filesystem::unmount(self)` — explicit unmount, returns storage
- `Drop` — best-effort unmount

**File I/O:**

- `fs.open(path, flags) -> File` — open with `OpenFlags`
- `file.read(buf) -> u32` — read bytes
- `file.write(data) -> u32` — write bytes
- `file.seek(SeekFrom) -> u32` — seek
- `file.tell() -> u32`
- `file.size() -> u32`
- `file.sync()` — flush to storage
- `file.truncate(size)` — truncate
- `file.close(self)` — explicit close returning `Result`
- `Drop` — best-effort close

**Convenience:**

- `fs.read_to_vec(path) -> Vec<u8>` — open, read all, close
- `fs.write_file(path, data)` — open with `CREAT|TRUNC`, write, close

**Path operations:**

- `fs.mkdir(path)`
- `fs.remove(path)` — remove file or directory
- `fs.rename(from, to)`
- `fs.stat(path) -> Metadata`
- `fs.exists(path) -> bool`

**Directory listing:**

- `fs.read_dir(path) -> ReadDir` — returns iterator
- `fs.list_dir(path) -> Vec<DirEntry>` — convenience, collects entries
- `ReadDir` implements `Iterator<Item = Result<DirEntry, Error>>`
- `.` and `..` filtered out

**FS-level:**

- `fs.fs_size() -> u32`
- `fs.gc()`

### Types

```rust
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileType { File, Dir }

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

### RamStorage

```rust
pub struct RamStorage {
    ...
}

impl RamStorage {
    pub fn new(block_size: u32, block_count: u32) -> Self;
}

impl Storage for RamStorage { ... }
```

Included for tests, examples, and anyone trying the crate without hardware.

### Feature flags

```toml
[features]
default = ["alloc"]
alloc = []                        # Vec-based convenience methods, RamStorage, internal cache alloc
std = ["alloc"]                   # std::error::Error, etc.
log = ["littlefs-rust-core/log"]   # pass through logging
```

`alloc` is required for this crate. Without it, the borrow-per-call + `Box<FileAllocation>` design
doesn't work. A hypothetical `no_alloc` mode (user-provided buffers) is out of scope for v1.

## Out of scope for v1

- **Custom attributes** (`getattr`/`setattr`) — firmware doesn't use them
- **`no_alloc` mode** — requires different API surface (user-provided buffers)
- **`grow`/`shrink`** — niche, easy to add later
- **Traverse** (`lfs_fs_traverse`) — diagnostic/defrag tool, not normal FS ops
- **Object-safe traits** (`DynFilesystem`, `DynStorage`) — `LpFs` abstraction lives in lightplayer,
  not here

## Phases

| Phase                                | Description                    | Deliverable                                                                                               |
|--------------------------------------|--------------------------------|-----------------------------------------------------------------------------------------------------------|
| [01](01-crate-scaffolding.md)        | Crate scaffolding              | `Cargo.toml`, module stubs, `Storage` trait, `Config`, `Error`, `RamStorage`                              |
| [02](02-filesystem-lifecycle.md)     | Filesystem lifecycle           | `Filesystem::format`, `mount`, `unmount`, `Drop`; trampoline wiring                                       |
| [03](03-file-ops.md)                 | File operations                | `File`, `OpenFlags`, `open`, `read`, `write`, `seek`, `tell`, `size`, `sync`, `truncate`, `close`, `Drop` |
| [04](04-dir-and-path-ops.md)         | Directory and path ops         | `ReadDir`, `DirEntry`, `mkdir`, `remove`, `rename`, `stat`, `exists`, `read_dir`, `list_dir`              |
| [05](05-convenience-and-examples.md) | Convenience methods + examples | `read_to_vec`, `write_file`, `fs_size`, `gc`; `ram_hello` + `ram_tree` examples                           |

## Validate

```bash
cargo test -p littlefs-rust
cargo run -p littlefs-rust --example ram_hello
cargo run -p littlefs-rust --example ram_tree
```
