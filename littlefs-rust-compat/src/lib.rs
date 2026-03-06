//! C ↔ Rust compatibility tests for littlefs-rust-core.
//!
//! Tests that littlefs-rust-core and the C littlefs (via littlefs2-sys) produce
//! interoperable on-disk formats.

pub mod c_impl;
pub mod rust_impl;
pub mod storage;
