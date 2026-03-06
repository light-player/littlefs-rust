# lp-littlefs

Pure Rust port of the [LittleFS](https://github.com/littlefs-project/littlefs) embedded filesystem.

On-disk format compatible with upstream LittleFS for interoperability. Hand-translated from the reference C implementation, preserving the original logic and control flow. No C dependencies, no bindgen.

## Architecture

### Workspace

The project is a Cargo workspace with two crates:

| Crate                | Purpose                                             |
| -------------------- | --------------------------------------------------- |
| `lp-littlefs-core`   | The filesystem implementation (`#![no_std]`)        |
| `lp-littlefs-compat` | C-to-Rust compatibility tests using `littlefs2-sys` |

### lp-littlefs-core

The core crate is a direct translation of `reference/lfs.c` into Rust. It exposes a C-style public API (`lfs_format`, `lfs_mount`, `lfs_file_open`, etc.) using raw pointers and `i32` return codes, matching the upstream C signatures. A safe wrapper API is planned but deferred until the core passes all upstream tests.

Internal modules mirror the structure of the C source:

| Module        | Responsibility                                                                           |
| ------------- | ---------------------------------------------------------------------------------------- |
| `bd`          | Block device read/prog/erase/sync with read and program caching                          |
| `block_alloc` | Block allocator (lookahead bitmap)                                                       |
| `crc`         | CRC-32 with 16-entry lookup table, matching `lfs_util.c`                                 |
| `dir`         | Metadata pairs: commit, fetch, find, open, traverse                                      |
| `file`        | File operations and CTZ skip-list data structure                                         |
| `fs`          | High-level filesystem: format, mount, mkdir, remove, rename, stat, attrs, grow, traverse |
| `tag`         | Tag encoding/decoding for the metadata log                                               |
| `lfs_config`  | Block device configuration with function-pointer callbacks                               |
| `lfs_gstate`  | Global state tracking (orphans, moves)                                                   |

Key types:

- `Lfs` — filesystem handle (config, caches, root pair, global state, lookahead)
- `LfsFile` — open file (position, CTZ list, inline cache)
- `LfsDir` — open directory (position within metadata pair chain)
- `LfsConfig` — block device callbacks and geometry

### lp-littlefs-compat

Compatibility tests that verify the Rust port produces byte-identical on-disk images to the C implementation. Uses `littlefs2-sys` for the C side and a `SharedStorage` backed by a single `Vec<u8>` so both implementations operate on the same block device.

Tests run in both directions:

- **Forward**: C formats/writes, Rust mounts and reads back
- **Backward**: Rust formats/writes, C mounts and reads back

### `no_std` and feature flags

The core crate is `#![no_std]`. The `alloc` feature (on by default) enables heap-backed `lfs_file_open`; without it, only `lfs_file_opencfg` (caller-provided buffers) is available.

| Feature        | Default | Description                                      |
| -------------- | ------- | ------------------------------------------------ |
| `alloc`        | yes     | Enables `alloc` crate for `lfs_file_open`        |
| `loop_limits`  | yes     | Iteration caps to detect infinite loops          |
| `std`          | no      | Standard library support                         |
| `log`          | no      | Logging via the `log` crate                      |
| `readonly`     | no      | Omit all write operations                        |
| `no_malloc`    | no      | Disable `lfs_file_open`; `lfs_file_opencfg` only |
| `multiversion` | no      | On-disk version selection                        |
| `shrink`       | no      | Filesystem shrink support                        |
| `slow_tests`   | no      | Power-loss and long-running tests                |

### Test infrastructure

Integration tests in `lp-littlefs-core/tests/` cover allocation, attributes, bad blocks, directories, file operations, entries, path handling, seek, truncation, moves, orphans, relocations, superblocks, exhaustion, interspersed operations, and power loss.

Test helpers (`tests/common/`):

- `RamStorage` — in-memory block device (erase fills `0xff`, prog copies bytes)
- `BadBlockRamStorage` — configurable bad-block simulation
- `WearLevelingBd` — per-block erase cycle tracking
- `PowerLossCtx` — write-count based power-loss injection with `Noop` (atomic progs) and `Ooo` (out-of-order revert) behaviors
- `run_powerloss_linear`, `run_powerloss_log`, `run_powerloss_exhaustive` — strategies for varying the fail point
- Deterministic PRNG matching the C test suite's `TEST_PRNG`

### Error handling

Functions return negative `i32` error codes (`LFS_ERR_IO`, `LFS_ERR_CORRUPT`, `LFS_ERR_NOENT`, etc.) and positive values for byte counts, matching the C convention.

## Development

### One-time setup

You can run the setup script to ensure a clean development environment, or read
the script and do it manually.

```bash
./dev-init.sh
```

Installs Rust (stable + rustfmt), the `thumbv6m-none-eabi` target for `no_std` checks, and cargo-deny.
Requires [just](https://github.com/casey/just) to run the CI recipe locally.

### Pre-commit

Run the "fix and check" recipe to format, fix, and check the code before committing.

```bash
just fci
```

## Upstream and reference

- [littlefs-project/littlefs](https://github.com/littlefs-project/littlefs) — original C implementation
- [DESIGN.md](https://github.com/littlefs-project/littlefs/blob/master/DESIGN.md) — design and rationale
- [SPEC.md](https://github.com/littlefs-project/littlefs/blob/master/SPEC.md) — on-disk format specification

## Prior art

- [littlefs2](https://crates.io/crates/littlefs2) — idiomatic Rust API wrapping the C library via FFI (requires C toolchain)
- [littlefs2-sys](https://crates.io/crates/littlefs2-sys) — low-level C bindings, compiles littlefs C source
- [chamelon](https://github.com/yomimono/chamelon) — pure OCaml implementation, interoperable with littlefs; inspiration for clean-room ports

## Why lp-littlefs

Existing Rust options (`littlefs2`, `littlefs2-sys`) depend on the C library and thus require a C
toolchain for every target.

This adds build and dev complexity to every downstream project, especially when cross-compiling.
The creator of
[LightPlayer](https://github.com/light-player/lightplayer), an embedded LED effects app, ran
into problems when adding littlefs to his project, and was thusly motivated to create a pure-rust
port of littlefs.

## Versioning

Releases are automated via [release-plz](https://release-plz.dev/) using conventional commits:

- `fix:` → patch (1.0.0 → 1.0.1)
- `feat:` → minor (1.0.0 → 1.1.0)
- `feat!:` or `BREAKING CHANGE` → major (1.0.0 → 2.0.0)

## Usage

Add as a git dependency:

```toml
lp-littlefs-core = { git = "https://github.com/light-player/lp-littlefs", branch = "main" }
```

## How it was made

`lp-littlefs` was primarily authored by the Cursor IDE and the Composer 1.5 model,
directed by a human engineer.

## Status

The C-to-Rust translation is functionally complete — format, mount, file I/O, directories, attributes, rename, remove, traverse, grow, and power-loss recovery all work and pass upstream-derived tests. A safe, idiomatic Rust wrapper API has not been built yet; the current public API mirrors the C function signatures with raw pointers.
