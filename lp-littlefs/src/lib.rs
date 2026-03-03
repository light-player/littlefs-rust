//! Hand-translated LittleFS from C to Rust.
//!
//! Logic and architecture kept close to reference/lfs.c. Uses `unsafe` where needed.
//! Safe wrapper API deferred until core passes all tests.

#![no_std]
#![allow(clippy::too_many_arguments)]
#![allow(dead_code, unused)]

#[cfg(feature = "alloc")]
extern crate alloc;

mod bd;
mod block_alloc;
mod crc;
mod dir;
mod error;
mod file;
mod fs;
mod lfs_config;
mod lfs_gstate;
mod lfs_info;
mod lfs_superblock;
mod lfs_type;
mod tag;
mod types;
mod util;

use crate::fs::Lfs;
use crate::lfs_config::LfsConfig;

/// Format a block device with littlefs.
/// Per lfs.h lfs_format. Calls lfs_format_ (lfs.c:4391).
#[inline(never)]
pub fn lfs_format(lfs: *mut Lfs, config: *const LfsConfig) -> i32 {
    todo!("lfs_format")
}

/// Mount a littlefs.
/// Per lfs.h lfs_mount. Calls lfs_mount_ (lfs.c:4482).
#[inline(never)]
pub fn lfs_mount(lfs: *mut Lfs, config: *const LfsConfig) -> i32 {
    todo!("lfs_mount")
}

/// Unmount a littlefs.
/// Per lfs.h lfs_unmount. Calls lfs_unmount_ (lfs.c:4647).
#[inline(never)]
pub fn lfs_unmount(lfs: *mut Lfs) -> i32 {
    todo!("lfs_unmount")
}
