# littlefs-rust

[![CI](https://github.com/light-player/littlefs-rust/actions/workflows/ci.yml/badge.svg)](https://github.com/light-player/littlefs-rust/actions/workflows/ci.yml)
[![License](https://img.shields.io/badge/license-BSD--3--Clause-blue.svg)](LICENSE)

Pure Rust port of the [LittleFS](https://github.com/littlefs-project/littlefs) embedded filesystem.
On-disk format compatible with upstream LittleFS for interoperability. No C dependencies, no
bindgen — builds on any Rust target.

> **Not yet on crates.io.** Use git dependencies for now (see [Getting started](#getting-started)).
> A crates.io release is planned after validation on real hardware.

### Basic example

```rust
fn main() {
    use littlefs_rust::{Config, Filesystem, RamStorage};

    let mut storage = RamStorage::new(512, 128);
    let config = Config::new(512, 128);

    Filesystem::format(&mut storage, &config).unwrap();
    let fs = Filesystem::mount(storage, config).unwrap();

    fs.write_file("/hello.txt", b"Hello, littlefs!").unwrap();
}
```

### Example programs

- [littlefs-rust/examples/ram_hello.rs](littlefs-rust/examples/ram_hello.rs)
- [littlefs-rust/examples/ram_tree.rs](littlefs-rust/examples/ram_tree.rs)

## Getting started

Most users should depend on `littlefs-rust`, which provides a safe Rust API:

```toml
littlefs-rust = { git = "https://github.com/light-player/littlefs-rust", branch = "main" }
```

Implement the `Storage` trait for your block device, then format, mount, and go. See
the [littlefs-rust README](littlefs-rust/README.md) for full documentation.

If you need the low-level C-style API directly (for FFI interop, custom wrappers, or testing),
depend on `littlefs-rust-core`:

```toml
littlefs-rust-core = { git = "https://github.com/light-player/littlefs-rust", branch = "main" }
```

## Workspace

| Crate                                       | Description                                                                                                                   |
|---------------------------------------------|-------------------------------------------------------------------------------------------------------------------------------|
| [`littlefs-rust`](littlefs-rust/)               | Safe Rust API — `Filesystem`, `File`, `ReadDir`, `Storage` trait                                                              |
| [`littlefs-rust-core`](littlefs-rust-core/)     | C-faithful port — raw pointer API matching upstream C signatures                                                              |
| [`littlefs-rust-compat`](littlefs-rust-compat/) | Cross-implementation compatibility tests using `littlefs2-sys` (optional; requires C toolchain, excluded from default builds) |

### littlefs-rust

The safe wrapper crate. Provides `Filesystem<S: Storage>`, `File`, `ReadDir` (iterator), `Config`,
`Error`, and convenience methods like `read_to_vec` and `write_file`. Multiple open files are
supported simultaneously. `File` and `ReadDir` implement `Drop` for RAII close.

### littlefs-rust-core

A function-by-function translation of `reference/lfs.c` into Rust. Exposes a C-style API (
`lfs_format`, `lfs_mount`, `lfs_file_open`, etc.) using raw pointers and `i32` return codes. The
translation is kept close to the C source so it can be mechanically verified against upstream and
bug fixes can be ported easily.

### littlefs-rust-compat

Compatibility tests verifying the Rust port produces byte-identical on-disk images to the C
implementation. Uses `littlefs2-sys` for the C side. Tests run in both directions: C formats → Rust
reads, and Rust formats → C reads. Optional: excluded from default builds; requires a C toolchain (
gcc, clang, libclang-dev). See [littlefs-rust-compat/README.md](littlefs-rust-compat/README.md) for
build requirements.

## Why littlefs-rust

Existing Rust
options ([littlefs2](https://crates.io/crates/littlefs2), [littlefs2-sys](https://crates.io/crates/littlefs2-sys))
depend on the C library and require a C toolchain for every target. This adds build complexity,
especially when cross-compiling for embedded targets like RISC-V.

littlefs-rust is pure Rust — no C compiler, no bindgen, no cross-compilation toolchain issues. Created
for [LightPlayer](https://github.com/light-player/lightplayer), an embedded LED effects system
running on ESP32.

## Development

### One-time setup

```bash
./dev-init.sh
```

Installs Rust (stable + rustfmt), the `thumbv6m-none-eabi` target for `no_std` checks, cargo-deny,
and sets up `reference/` (symlink to upstream littlefs at the commit in `.upstream`). Requires
[just](https://github.com/casey/just) to run the CI recipe locally. See [CONTRIBUTING.md](CONTRIBUTING.md)
for the upstream sync workflow.

### Pre-commit

```bash
just fci
```

### Running tests

```bash
cargo test -p littlefs-rust          # safe wrapper tests
cargo test -p littlefs-rust-core     # core tests
just compat                       # C compatibility tests (recommended; handles macOS arm64)
# Or: cargo test -p littlefs-rust-compat  # requires C toolchain
```

Compat tests require a C toolchain. Use `just compat` from the repo root — it sets the bindgen
workaround on macOS arm64 automatically.
See [littlefs-rust-compat/README.md](littlefs-rust-compat/README.md) for details.

## Upstream and reference

- [littlefs-project/littlefs](https://github.com/littlefs-project/littlefs) — original C
  implementation. Tracked commit in `.upstream`; run `just upstream-sync` to populate `reference/`.
- [DESIGN.md](https://github.com/littlefs-project/littlefs/blob/master/DESIGN.md) — design and
  rationale
- [SPEC.md](https://github.com/littlefs-project/littlefs/blob/master/SPEC.md) — on-disk format
  specification

## Prior art

- [littlefs2](https://crates.io/crates/littlefs2) — idiomatic Rust API wrapping the C library via
  FFI (requires C toolchain)
- [littlefs2-sys](https://crates.io/crates/littlefs2-sys) — low-level C bindings, compiles littlefs
  C source
- [chamelon](https://github.com/yomimono/chamelon) — pure OCaml implementation, interoperable with
  littlefs

## Versioning

Releases are automated via [release-plz](https://release-plz.dev/) using conventional commits:

- `fix:` → patch
- `feat:` → minor
- `feat!:` or `BREAKING CHANGE` → major

## How it was made

littlefs-rust was primarily authored by the Cursor IDE AI, directed by a human engineer.

## Status

The C-to-Rust core translation is functionally complete — format, mount, file I/O, directories,
attributes, rename, remove, traverse, grow, and power-loss recovery all pass upstream-derived tests.
The safe wrapper API (`littlefs-rust`) is implemented and tested.
