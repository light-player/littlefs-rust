# lp-littlefs-compat

Cross-implementation compatibility tests that verify `lp-littlefs-core` (Rust) and the upstream C
littlefs (via [`littlefs2-sys`](https://crates.io/crates/littlefs2-sys)) produce interoperable
on-disk formats.

## Rationale

A pure Rust port of a filesystem is only useful if it can read and write the same format as the
original. Unit tests within `lp-littlefs-core` exercise the Rust code against itself, but cannot
catch subtle format-level divergences — a CRC mismatch in a tag, a differently-ordered directory
entry, an inline file threshold off by one. This crate catches those bugs by having one
implementation write and the other read back.

## How it works

### Shared storage

Both implementations operate on the same in-memory byte array. `SharedStorage` owns a `Vec<u8>`
behind `UnsafeCell` and provides block device callbacks for both the C (`littlefs2_sys::lfs_config`)
and Rust (`lp_littlefs_core::LfsConfig`) configuration structs. A `TestGeometry` struct describes
block size, block count, read/prog size, cache size, and lookahead size.

```
SharedStorage (Vec<u8>)
    ├── build_c_config()    → littlefs2_sys::lfs_config    (C callbacks)
    └── build_rust_env()    → RustEnv { LfsConfig, buffers } (Rust callbacks)
```

### Wrapper modules

- **`c_impl`** — functions that call `littlefs2_sys::lfs_*` to format, mount, mkdir,
  create/read/write files, list directories, rename, remove, unmount.
- **`rust_impl`** — equivalent functions calling `lp_littlefs_core::lfs_*` with the same signatures
  and semantics.

The two modules are concrete, not trait-based. The C and Rust APIs have different struct types; a
unifying trait would add indirection for no benefit.

### Test directions

Tests run in both directions:

- **Forward** (C creates, Rust reads): C formats the device, creates directories and files,
  unmounts. Rust mounts and verifies everything is readable and correct.
- **Backward** (Rust creates, C reads): Rust formats, creates content, unmounts. C mounts and
  verifies.

## Tests

### `test_operations.rs` — operation-level compat

Fine-grained tests for individual operations across implementations:

- Format and mount (C format / Rust mount, Rust format / C mount)
- Mkdir and file creation (both orderings)
- Rename and remove
- File content (inline and CTZ-sized files)
- Nested directory structures
- Directory removal
- CRC/layout verification at different prog sizes

### `test_compat.rs` — upstream forward/backward compat

Mirrors the `test_compat.toml` tests from the upstream C littlefs test suite. Parameterized over
file sizes (4, 32, 512, 8192 bytes) and chunk counts:

- Mount after cross-format
- Read directories created by the other implementation
- Read files created by the other implementation
- Read files inside directories
- Write new directories/files on a volume created by the other implementation
- Write files inside directories

## Build requirements

Requires a C compiler and clang (for bindgen) to build `littlefs2-sys`. On macOS arm64, the existing
`.cargo/config.toml` handles the bindgen workaround.

## Running

```bash
cargo test -p lp-littlefs-compat
```

## Dependencies

- `lp-littlefs-core` — the Rust LittleFS implementation under test
- `littlefs2-sys` — C littlefs bindings (with `malloc` feature for buffer allocation)
- `rstest` — test parameterization
