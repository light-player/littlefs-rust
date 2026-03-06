//! C ↔ Rust compatibility tests for lp-littlefs.
//!
//! Tests that lp-littlefs and the C littlefs (via littlefs2-sys) produce
//! interoperable on-disk formats.

pub mod c_impl;
pub mod rust_impl;
pub mod storage;
