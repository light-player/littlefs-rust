# lp-littlefs

Pure Rust implementation of the [LittleFS](https://github.com/littlefs-project/littlefs) embedded filesystem. On-disk format compatible with upstream LittleFS for interoperability. No C dependencies, no bindgen — builds on any Rust target.

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

Most users should depend on `lp-littlefs`, which provides a safe Rust API:

```toml
lp-littlefs = { git = "https://github.com/light-player/lp-littlefs", branch = "main" }
```

Implement the `Storage` trait for your block device, then format, mount, and go. See the [lp-littlefs README](lp-littlefs/README.md) for full documentation.

If you need the low-level C-style API directly (for FFI interop, custom wrappers, or testing), depend on `lp-littlefs-core`:

```toml
lp-littlefs-core = { git = "https://github.com/light-player/lp-littlefs", branch = "main" }
```

## Workspace

| Crate | Description |
|-------|-------------|
| [`lp-littlefs`](lp-littlefs/) | Safe Rust API — `Filesystem`, `File`, `ReadDir`, `Storage` trait |
| [`lp-littlefs-core`](lp-littlefs-core/) | C-faithful port — raw pointer API matching upstream C signatures |
| [`lp-littlefs-compat`](lp-littlefs-compat/) | Cross-implementation compatibility tests using `littlefs2-sys` |

### lp-littlefs

The safe wrapper crate. Provides `Filesystem<S: Storage>`, `File`, `ReadDir` (iterator), `Config`, `Error`, and convenience methods like `read_to_vec` and `write_file`. Multiple open files are supported simultaneously. `File` and `ReadDir` implement `Drop` for RAII close.

### lp-littlefs-core

A function-by-function translation of `reference/lfs.c` into Rust. Exposes a C-style API (`lfs_format`, `lfs_mount`, `lfs_file_open`, etc.) using raw pointers and `i32` return codes. The translation is kept close to the C source so it can be mechanically verified against upstream and bug fixes can be ported easily.

### lp-littlefs-compat

Compatibility tests verifying the Rust port produces byte-identical on-disk images to the C implementation. Uses `littlefs2-sys` for the C side. Tests run in both directions: C formats → Rust reads, and Rust formats → C reads.

## Why lp-littlefs

Existing Rust options ([littlefs2](https://crates.io/crates/littlefs2), [littlefs2-sys](https://crates.io/crates/littlefs2-sys)) depend on the C library and require a C toolchain for every target. This adds build complexity, especially when cross-compiling for embedded targets like RISC-V.

lp-littlefs is pure Rust — no C compiler, no bindgen, no cross-compilation toolchain issues. Created for [LightPlayer](https://github.com/light-player/lightplayer), an embedded LED effects system running on ESP32.

## Development

### One-time setup

```bash
./dev-init.sh
```

Installs Rust (stable + rustfmt), the `thumbv6m-none-eabi` target for `no_std` checks, and cargo-deny. Requires [just](https://github.com/casey/just) to run the CI recipe locally.

### Pre-commit

```bash
just fci
```

### Running tests

```bash
cargo test -p lp-littlefs          # safe wrapper tests
cargo test -p lp-littlefs-core     # core tests
cargo test -p lp-littlefs-compat   # C compatibility tests
```

## Upstream and reference

- [littlefs-project/littlefs](https://github.com/littlefs-project/littlefs) — original C implementation
- [DESIGN.md](https://github.com/littlefs-project/littlefs/blob/master/DESIGN.md) — design and rationale
- [SPEC.md](https://github.com/littlefs-project/littlefs/blob/master/SPEC.md) — on-disk format specification

## Prior art

- [littlefs2](https://crates.io/crates/littlefs2) — idiomatic Rust API wrapping the C library via FFI (requires C toolchain)
- [littlefs2-sys](https://crates.io/crates/littlefs2-sys) — low-level C bindings, compiles littlefs C source
- [chamelon](https://github.com/yomimono/chamelon) — pure OCaml implementation, interoperable with littlefs

## Versioning

Releases are automated via [release-plz](https://release-plz.dev/) using conventional commits:

- `fix:` → patch
- `feat:` → minor
- `feat!:` or `BREAKING CHANGE` → major

## How it was made

lp-littlefs was primarily authored by the Cursor IDE AI, directed by a human engineer.

## Status

The C-to-Rust core translation is functionally complete — format, mount, file I/O, directories, attributes, rename, remove, traverse, grow, and power-loss recovery all pass upstream-derived tests. The safe wrapper API (`lp-littlefs`) is implemented and tested.
