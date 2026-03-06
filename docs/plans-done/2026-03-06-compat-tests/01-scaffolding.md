# Phase 1: Crate Scaffolding

## Goal

Create `littlefs-rust-compat` with shared storage and config builders for both the C (`littlefs2-sys`) and Rust (`littlefs-rust`) implementations. No tests yet — just the infrastructure.

## Steps

### 1. Create crate directory and Cargo.toml

Create `littlefs-rust-compat/` alongside `littlefs-rust/`. Add to workspace members in root `Cargo.toml`.

```toml
[package]
name = "littlefs-rust-compat"
version = "0.1.0"
edition = "2021"
description = "C ↔ Rust compatibility tests for littlefs-rust"
license = "BSD-3-Clause"

[dependencies]
littlefs-rust = { path = "../littlefs-rust" }
littlefs2-sys = { version = "0.3", features = ["malloc"] }

[dev-dependencies]
env_logger = "0.10"
rstest = "0.26"
```

### 2. SharedStorage (src/storage.rs)

Refactored from `littlefs-rust-c-align/src/storage.rs`. Key changes:

- Rename `AlignStorage` → `SharedStorage`
- Remove the `BlockDevice` trait impl (trait no longer exists)
- Add `build_rust_config` that returns `littlefs_rust::LfsConfig` with callbacks pointing at the shared storage
- Keep `build_c_config` (renamed from `build_lfs_config`) returning `littlefs2_sys::lfs_config`
- Both config builders accept a `&TestGeometry` for the common parameters

```rust
pub struct TestGeometry {
    pub block_size: u32,
    pub block_count: u32,
    pub read_size: u32,
    pub prog_size: u32,
    pub cache_size: u32,
    pub lookahead_size: u32,
}
```

Default: `block_size=512, block_count=128, read_size=16, prog_size=16, cache_size=512, lookahead_size=512` (matches upstream test defaults).

The `LfsConfig` callback signatures in `littlefs-rust` are:
```rust
type lfs_read_t = unsafe extern "C" fn(*const LfsConfig, u32, u32, *mut u8, u32) -> i32;
type lfs_prog_t = unsafe extern "C" fn(*const LfsConfig, u32, u32, *const u8, u32) -> i32;
type lfs_erase_t = unsafe extern "C" fn(*const LfsConfig, u32) -> i32;
type lfs_sync_t = unsafe extern "C" fn(*const LfsConfig) -> i32;
```

The `littlefs2-sys` callbacks have the same shape but reference `littlefs2_sys::lfs_config` instead. Both store a `context` pointer. `SharedStorage` sets context to `&self` in both configs.

Since `littlefs_rust::LfsConfig` manages its own read/prog/lookahead buffers via pointers, and `littlefs2-sys` with the `malloc` feature allocates them internally when null, the config builders should:
- For `build_c_config`: pass null buffers (let malloc handle it)
- For `build_rust_config`: allocate owned buffers and return them alongside the config in a wrapper struct

```rust
pub struct RustEnv {
    pub config: littlefs_rust::LfsConfig,
    _read_buf: Vec<u8>,
    _prog_buf: Vec<u8>,
    _lookahead_buf: Vec<u8>,
}
```

### 3. lib.rs

```rust
pub mod c_impl;
pub mod rust_impl;
pub mod storage;
```

### 4. Stub c_impl.rs and rust_impl.rs

Empty modules with doc comments. Implementation comes in phase 2.

### 5. Verify build

```bash
cargo build -p littlefs-rust-core-compat
```

Both `littlefs2-sys` (C compilation + bindgen) and `littlefs-rust` must link cleanly.

## Validate

```bash
cargo build -p littlefs-rust-core-compat
cargo test -p littlefs-rust-core-compat  # no tests yet, just verify it compiles
```
