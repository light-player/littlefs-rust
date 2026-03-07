//! Safe Rust API for the LittleFS embedded filesystem.
//!
//! Built on [`littlefs-rust-core`](https://crates.io/crates/littlefs-rust-core), a
//! function-by-function Rust port of the
//! [C littlefs](https://github.com/littlefs-project/littlefs). No C toolchain required.
//!
//! # Quick start
//!
//! ```rust
//! use littlefs_rust::{Config, Filesystem, RamStorage};
//!
//! let mut storage = RamStorage::new(512, 128);
//! let config = Config::new(512, 128);
//!
//! Filesystem::format(&mut storage, &config).unwrap();
//! let fs = Filesystem::mount(storage, config).map_err(|(e, _)| e).unwrap();
//!
//! fs.write_file("/hello.txt", b"Hello, littlefs!").unwrap();
//! let data = fs.read_to_vec("/hello.txt").unwrap();
//! assert_eq!(data, b"Hello, littlefs!");
//!
//! fs.unmount().unwrap();
//! ```
//!
//! # Architecture
//!
//! The crate uses interior mutability ([`RefCell`](core::cell::RefCell)) so that
//! [`Filesystem`] methods take `&self`. Each operation borrows the internal state only
//! for the duration of one core call, then releases it. This enables multiple open
//! files, interleaved file and directory operations, and reading files while iterating
//! directories — all without conflict.
//!
//! [`File`] and [`ReadDir`] hold a shared reference to the [`Filesystem`] and implement
//! [`Drop`] for RAII close.

#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(feature = "alloc")]
extern crate alloc;

mod config;
mod dir;
mod error;
mod file;
mod filesystem;
mod metadata;
mod ram;
mod storage;

pub use config::Config;
pub use dir::ReadDir;
pub use error::Error;
pub use file::File;
pub use filesystem::Filesystem;
pub use metadata::{DirEntry, FileType, Metadata, OpenFlags, SeekFrom};
pub use ram::RamStorage;
pub use storage::Storage;
