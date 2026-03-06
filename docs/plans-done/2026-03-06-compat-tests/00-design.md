# C Compatibility Tests — littlefs-rust-compat

Replace `littlefs-rust-c-align` with a cleaned-up `littlefs-rust-compat` crate whose job is testing that `littlefs-rust` and the C littlefs produce interoperable on-disk formats.

## Motivation

The existing `littlefs-rust-c-align` crate proved its value by catching real format-level bugs (directory entries disappearing across mount cycles, CTZ write issues). But the crate is currently broken — it depends on a high-level wrapper API (`LittleFs`, `Config`, `BlockDevice` trait, etc.) that no longer exists. The `littlefs-rust` public API is now a C-style function interface (`lfs_format`, `lfs_mount`, raw pointer args, `i32` returns).

Meanwhile, the test-parity2 plan (phase 7) proposes 14 upstream `test_compat` forward/backward tests as self-test aliases inside `littlefs-rust` — where `lfsp_*` just calls `lfs_*`. That approach adds near-zero coverage: "format with lfs, mount with lfs, read with lfs" is already tested hundreds of times across the existing suite. The self-test alias pattern makes sense in the upstream C codebase where the test runner can optionally link a separate `lfsp` library, but in Rust there is no equivalent mechanism.

A separate compat crate using `littlefs2-sys` as the C implementation gives those 14 tests real teeth: C formats, Rust reads (and vice versa), exercising actual cross-implementation interop.

## Scope

- Rename and refactor `littlefs-rust-c-align` into `littlefs-rust-compat`
- Rewrite to use the current `littlefs-rust` C-style API
- Port existing operation-level compat tests
- Add the 14 upstream `test_compat` forward/backward tests
- Reduce `littlefs-rust/tests/test_compat.rs` to the 3 version edge cases that belong there

## What stays in `littlefs-rust`

The 3 version edge cases (`major_incompat`, `minor_incompat`, `minor_bump`) test how `littlefs-rust` handles superblock version fields internally. They don't need a second implementation — they need internal APIs (`lfs_dir_fetch`, `lfs_dir_commit`, `LfsSuperblock`). These stay in `littlefs-rust/tests/test_compat.rs` and are covered by test-parity2 phase 7 (reduced to just those 3 cases).

## Architecture

### Shared storage

Both implementations operate on the same in-memory byte array. A `SharedStorage` struct (renamed from `AlignStorage`) owns a `Vec<u8>` behind `UnsafeCell` and provides:

- C function pointer callbacks for `littlefs2-sys::lfs_config` (read/prog/erase/sync)
- C function pointer callbacks for `littlefs_rust::LfsConfig` (same signature shape)
- Config builders for each side that wire up the callbacks and point at the shared storage

The `littlefs_rust::LfsConfig` and `littlefs2_sys::lfs_config` have compatible callback signatures (`unsafe extern "C" fn(...) -> i32`), but differ in struct layout (field types, field order). `SharedStorage` builds each separately from a common `TestGeometry` describing block_size, block_count, etc.

### Wrapper modules

- `c_impl`: Functions that call `littlefs2_sys::lfs_*` to format, mount, mkdir, create files, read dirs, read files, write files, unmount. Each function takes `&SharedStorage` + `&TestGeometry` (or a built config) and returns `Result<T, i32>`.
- `rust_impl`: Equivalent functions calling `littlefs_rust::lfs_*`. Same signatures, same semantics, different underlying implementation.

The two modules are intentionally concrete (not trait-based). The C and Rust APIs have different struct types and ergonomics; a unifying trait would add a layer of indirection for no current benefit. If a third implementation is added later, that's the time to extract a trait.

### Test structure

```
littlefs-rust-compat/
├── Cargo.toml
├── src/
│   ├── lib.rs
│   ├── storage.rs       # SharedStorage + TestGeometry
│   ├── c_impl.rs        # littlefs2-sys wrappers
│   └── rust_impl.rs     # littlefs-rust wrappers
└── tests/
    ├── test_operations.rs    # Ported from c-align: mkdir, rename, remove, content, nested dirs
    └── test_compat.rs        # Upstream forward/backward compat (14 cases)
```

### Dependencies

```toml
[package]
name = "littlefs-rust-compat"
version = "0.1.0"
edition = "2021"

[dependencies]
littlefs-rust = { path = "../littlefs-rust" }
littlefs2-sys = { version = "0.3", features = ["malloc"] }

[dev-dependencies]
env_logger = "0.10"
rstest = "0.26"
```

## Phases

| Phase | File | Description |
|-------|------|-------------|
| [01](01-scaffolding.md) | Crate structure, storage, config builders | New crate, `SharedStorage`, config builders for both sides |
| [02](02-port-operations.md) | `c_impl`, `rust_impl`, `test_operations.rs` | Port existing c-align tests to current API |
| [03](03-upstream-compat.md) | `test_compat.rs` | Add 14 upstream forward/backward compat tests |

After phase 3, update test-parity2 phase 7 to cover only the 3 version edge cases in `littlefs-rust/tests/test_compat.rs`.

## Build requirements

- C compiler (for littlefs C sources compiled by `littlefs2-sys`)
- clang (for bindgen)
- Existing `.cargo/config.toml` handles arm64 macOS bindgen workaround

## Validate

```bash
cargo test -p littlefs-rust-core-compat
```
