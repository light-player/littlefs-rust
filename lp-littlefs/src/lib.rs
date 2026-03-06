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
#[cfg(feature = "alloc")]
mod lfs_alloc_module;

mod error;
mod file;
mod fs;
mod lfs_config;
mod lfs_gstate;
mod lfs_info;
mod lfs_superblock;
pub mod lfs_type;
#[cfg(test)]
mod test;
#[macro_use]
mod macros;
mod tag;
mod types;
mod util;

use core::ffi::c_void;

pub use crate::dir::LfsDir;
pub use crate::error::{
    LFS_ERR_CORRUPT, LFS_ERR_EXIST, LFS_ERR_INVAL, LFS_ERR_IO, LFS_ERR_ISDIR, LFS_ERR_NAMETOOLONG,
    LFS_ERR_NOATTR, LFS_ERR_NOENT, LFS_ERR_NOMEM, LFS_ERR_NOSPC, LFS_ERR_NOTDIR,
};
pub use crate::file::LfsFile;
pub use crate::fs::Lfs;
pub use crate::lfs_config::LfsConfig;
pub use crate::lfs_info::{LfsAttr, LfsFileConfig, LfsInfo};

// Test helpers for integration tests (bypass, traverse isolation).
#[doc(hidden)]
pub use crate::dir::traverse::TraverseTestOut;
#[doc(hidden)]
pub use crate::fs::format::{
    test_format_minimal_superblock, test_traverse_filter_gets_superblock_after_push,
    test_traverse_format_attrs,
};
pub use crate::lfs_info::LfsFsinfo;

// Internal APIs exposed for evil/corruption tests that need to manipulate
// metadata directly (lfs_init, lfs_dir_fetch, lfs_dir_commit, etc.).
#[doc(hidden)]
pub use crate::dir::commit::lfs_dir_commit;
#[doc(hidden)]
pub use crate::dir::fetch::lfs_dir_fetch;
#[doc(hidden)]
pub use crate::dir::traverse::lfs_dir_get;
#[doc(hidden)]
pub use crate::dir::LfsMdir;
#[doc(hidden)]
pub use crate::file::lfs_ctz::lfs_ctz_fromle32;
#[doc(hidden)]
pub use crate::file::lfs_ctz::LfsCtz;
#[doc(hidden)]
pub use crate::fs::init::{lfs_deinit, lfs_init};
#[doc(hidden)]
pub use crate::fs::superblock::lfs_fs_prepmove;
#[doc(hidden)]
pub use crate::tag::{lfs_mattr, lfs_mktag};
use crate::types::{lfs_block_t, lfs_off_t, lfs_size_t, lfs_soff_t, lfs_ssize_t};
#[doc(hidden)]
pub use crate::util::{lfs_pair_fromle32, lfs_pair_tole32, lfs_tole32};

/// Format a block device with littlefs.
/// Per lfs.h lfs_format. Calls lfs_format_ (lfs.c:4391).
#[inline(never)]
pub fn lfs_format(lfs: *mut Lfs, config: *const LfsConfig) -> i32 {
    crate::lfs_trace!("lfs_format({:p}, {:p})", lfs, config);
    let err = crate::fs::lfs_format_(lfs, config);
    crate::lfs_trace!("lfs_format -> {}", err);
    err
}

/// Mount a littlefs.
/// Per lfs.h lfs_mount. Calls lfs_mount_ (lfs.c:4482).
#[inline(never)]
pub fn lfs_mount(lfs: *mut Lfs, config: *const LfsConfig) -> i32 {
    crate::lfs_trace!("lfs_mount({:p}, {:p})", lfs, config);
    crate::fs::lfs_mount_(lfs, config)
}

/// Unmount a littlefs.
/// Per lfs.h lfs_unmount. Calls lfs_unmount_ (lfs.c:4647).
#[inline(never)]
pub fn lfs_unmount(lfs: *mut Lfs) -> i32 {
    crate::fs::lfs_unmount_(lfs)
}

/// Remove a file or directory. Per lfs.h lfs_remove (lfs.c:6193-6195).
#[inline(never)]
pub fn lfs_remove(lfs: *mut Lfs, path: *const u8) -> i32 {
    crate::fs::remove::lfs_remove_(lfs, path)
}

/// Rename or move a file or directory. Per lfs.h lfs_rename (lfs.c:6227-6231).
#[inline(never)]
pub fn lfs_rename(lfs: *mut Lfs, oldpath: *const u8, newpath: *const u8) -> i32 {
    crate::fs::rename::lfs_rename_(lfs, oldpath, newpath)
}

/// Find info about a file or directory. Per lfs.h lfs_stat (lfs.c:6263-6267).
#[inline(never)]
pub fn lfs_stat(lfs: *mut Lfs, path: *const u8, info: *mut LfsInfo) -> i32 {
    crate::fs::stat::lfs_stat_(lfs, path, info)
}

/// Get a custom attribute. Per lfs.h lfs_getattr (lfs.c:6090-6105).
#[inline(never)]
pub fn lfs_getattr(
    lfs: *mut Lfs,
    path: *const u8,
    r#type: u8,
    buffer: *mut c_void,
    size: lfs_size_t,
) -> lfs_ssize_t {
    crate::fs::attr::lfs_getattr_(lfs, path, r#type, buffer, size)
}

/// Set custom attributes. Per lfs.h lfs_setattr (lfs.c:6471-6475).
#[inline(never)]
pub fn lfs_setattr(
    lfs: *mut Lfs,
    path: *const u8,
    r#type: u8,
    buffer: *const c_void,
    size: lfs_size_t,
) -> i32 {
    crate::fs::attr::lfs_setattr_(lfs, path, r#type, buffer, size)
}

/// Remove a custom attribute. Per lfs.h lfs_removeattr (lfs.c:6487-6491).
#[inline(never)]
pub fn lfs_removeattr(lfs: *mut Lfs, path: *const u8, r#type: u8) -> i32 {
    crate::fs::attr::lfs_removeattr_(lfs, path, r#type)
}

/// Open a file. Per lfs.h lfs_file_open (lfs.c:6140-6146).
#[inline(never)]
pub fn lfs_file_open(lfs: *mut Lfs, file: *mut LfsFile, path: *const u8, flags: i32) -> i32 {
    crate::file::ops::lfs_file_open_(lfs, file, path as *const i8, flags)
}

/// Open a file with extra configuration. Per lfs.h lfs_file_opencfg (lfs.c:6193-6197).
#[inline(never)]
pub fn lfs_file_opencfg(
    lfs: *mut Lfs,
    file: *mut LfsFile,
    path: *const u8,
    flags: i32,
    config: *const LfsFileConfig,
) -> i32 {
    crate::file::ops::lfs_file_opencfg_(lfs, file, path as *const i8, flags, config)
}

/// Close a file. Per lfs.h lfs_file_close (lfs.c:6227-6231).
#[inline(never)]
pub fn lfs_file_close(lfs: *mut Lfs, file: *mut LfsFile) -> i32 {
    crate::file::ops::lfs_file_close_(lfs, file)
}

/// Synchronize a file on storage. Per lfs.h lfs_file_sync (lfs.c:6263-6267).
#[inline(never)]
pub fn lfs_file_sync(lfs: *mut Lfs, file: *mut LfsFile) -> i32 {
    crate::file::ops::lfs_file_sync_(lfs, file)
}

/// Read data from file. Per lfs.h lfs_file_read (lfs.c:6210-6224).
#[inline(never)]
pub fn lfs_file_read(
    lfs: *mut Lfs,
    file: *mut LfsFile,
    buffer: *mut c_void,
    size: lfs_size_t,
) -> lfs_ssize_t {
    crate::file::ops::lfs_file_read_(lfs, file, buffer, size)
}

/// Write data to file. Per lfs.h lfs_file_write (lfs.c:6228-6242).
#[inline(never)]
pub fn lfs_file_write(
    lfs: *mut Lfs,
    file: *mut LfsFile,
    buffer: *const c_void,
    size: lfs_size_t,
) -> lfs_ssize_t {
    crate::file::ops::lfs_file_write_(lfs, file, buffer, size)
}

/// Change the position of the file. Per lfs.h lfs_file_seek (lfs.c:6246-6260).
#[inline(never)]
pub fn lfs_file_seek(
    lfs: *mut Lfs,
    file: *mut LfsFile,
    off: lfs_soff_t,
    whence: i32,
) -> lfs_soff_t {
    crate::file::ops::lfs_file_seek_(lfs, file, off, whence)
}

/// Truncate the size of the file. Per lfs.h lfs_file_truncate (lfs.c:6471-6475).
#[inline(never)]
pub fn lfs_file_truncate(lfs: *mut Lfs, file: *mut LfsFile, size: lfs_off_t) -> i32 {
    crate::file::ops::lfs_file_truncate_(lfs, file, size)
}

/// Return the position of the file. Per lfs.h lfs_file_tell.
#[inline(never)]
pub fn lfs_file_tell(_lfs: *mut Lfs, file: *mut LfsFile) -> lfs_soff_t {
    crate::file::ops::lfs_file_tell_(core::ptr::null(), file)
}

/// Change the position to the beginning of the file. Per lfs.h lfs_file_rewind (lfs.c:6487-6491).
#[inline(never)]
pub fn lfs_file_rewind(lfs: *mut Lfs, file: *mut LfsFile) -> i32 {
    crate::file::ops::lfs_file_rewind_(lfs, file)
}

/// Return the size of the file. Per lfs.h lfs_file_size (lfs.c:6495-6499).
#[inline(never)]
pub fn lfs_file_size(_lfs: *mut Lfs, file: *mut LfsFile) -> lfs_soff_t {
    crate::file::ops::lfs_file_size_(core::ptr::null(), file)
}

/// Create a directory. Per lfs.h lfs_mkdir (lfs.c:6503-6507).
#[inline(never)]
pub fn lfs_mkdir(lfs: *mut Lfs, path: *const u8) -> i32 {
    crate::fs::mkdir::lfs_mkdir_(lfs, path)
}

/// Open a directory. Per lfs.h lfs_dir_open (lfs.c:6511-6515).
#[inline(never)]
pub fn lfs_dir_open(lfs: *mut Lfs, dir: *mut LfsDir, path: *const u8) -> i32 {
    crate::dir::open::lfs_dir_open_(lfs, dir, path)
}

/// Close a directory. Per lfs.h lfs_dir_close.
#[inline(never)]
pub fn lfs_dir_close(lfs: *mut Lfs, dir: *mut LfsDir) -> i32 {
    crate::dir::open::lfs_dir_close_(lfs, dir)
}

/// Read an entry in the directory. Per lfs.h lfs_dir_read.
#[inline(never)]
pub fn lfs_dir_read(lfs: *mut Lfs, dir: *mut LfsDir, info: *mut LfsInfo) -> i32 {
    crate::dir::open::lfs_dir_read_(lfs, dir, info)
}

/// Change the position of the directory. Per lfs.h lfs_dir_seek.
#[inline(never)]
pub fn lfs_dir_seek(lfs: *mut Lfs, dir: *mut LfsDir, off: lfs_off_t) -> i32 {
    todo!("lfs_dir_seek")
}

/// Return the position of the directory. Per lfs.h lfs_dir_tell (lfs.c:6400-6412).
#[inline(never)]
pub fn lfs_dir_tell(lfs: *mut Lfs, dir: *mut LfsDir) -> lfs_soff_t {
    todo!("lfs_dir_tell")
}

/// Change the position to the beginning of the directory. Per lfs.h lfs_dir_rewind.
#[inline(never)]
pub fn lfs_dir_rewind(lfs: *mut Lfs, dir: *mut LfsDir) -> i32 {
    todo!("lfs_dir_rewind")
}

/// Find on-disk info about the filesystem. Per lfs.h lfs_fs_stat (lfs.c:6449-6453).
#[inline(never)]
pub fn lfs_fs_stat(lfs: *mut Lfs, fsinfo: *mut LfsFsinfo) -> i32 {
    crate::fs::lfs_fs_stat_(lfs, fsinfo)
}

/// Find the current size of the filesystem. Per lfs.h lfs_fs_size (lfs.c:6449-6453).
#[inline(never)]
pub fn lfs_fs_size(lfs: *mut Lfs) -> lfs_ssize_t {
    crate::fs::stat::lfs_fs_size_(lfs)
}

/// Callback type for lfs_fs_traverse. Per lfs.h int (*cb)(void*, lfs_block_t).
pub type LfsTraverseCb = unsafe extern "C" fn(data: *mut c_void, block: lfs_block_t) -> i32;

/// Traverse through all blocks in use by the filesystem. Per lfs.h lfs_fs_traverse.
#[inline(never)]
pub fn lfs_fs_traverse(lfs: *mut Lfs, cb: LfsTraverseCb, data: *mut c_void) -> i32 {
    crate::fs::traverse::lfs_fs_traverse_(lfs, Some(cb), data, false)
}

/// Attempt to make the filesystem consistent. Per lfs.h lfs_fs_mkconsistent (lfs.c:6479-6483).
#[inline(never)]
pub fn lfs_fs_mkconsistent(lfs: *mut Lfs) -> i32 {
    crate::fs::consistent::lfs_fs_mkconsistent_(lfs)
}

/// Attempt any janitorial work. Per lfs.h lfs_fs_gc (lfs.c:6495-6499).
#[inline(never)]
pub fn lfs_fs_gc(lfs: *mut Lfs) -> i32 {
    crate::fs::consistent::lfs_fs_gc_(lfs)
}

/// Force consistency (deorphan, demove, desuperblock). For testing.
#[doc(hidden)]
pub fn lfs_fs_forceconsistency(lfs: *mut Lfs) -> i32 {
    crate::fs::superblock::lfs_fs_forceconsistency(lfs)
}

/// Prepend orphan count delta to gstate. For testing power-loss paths.
#[doc(hidden)]
pub fn lfs_fs_preporphans(lfs: *mut Lfs, orphans: i8) -> i32 {
    crate::fs::superblock::lfs_fs_preporphans(lfs, orphans)
}

/// True if gstate has pending orphans. For testing.
#[doc(hidden)]
pub unsafe fn lfs_fs_hasorphans(lfs: *const Lfs) -> bool {
    crate::lfs_gstate::lfs_gstate_hasorphans(&(*lfs).gstate)
}

/// Grow the filesystem to a new size. Per lfs.h lfs_fs_grow (lfs.c:6511-6515).
#[inline(never)]
pub fn lfs_fs_grow(lfs: *mut Lfs, block_count: lfs_size_t) -> i32 {
    todo!("lfs_fs_grow")
}
