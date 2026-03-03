//! Error codes. Per lfs.h enum lfs_error.
//! Negative values allow positive return values (e.g. bytes read).

#![allow(non_camel_case_types)]

pub const LFS_ERR_OK: i32 = 0;
pub const LFS_ERR_IO: i32 = -5;
pub const LFS_ERR_CORRUPT: i32 = -84;
pub const LFS_ERR_NOENT: i32 = -2;
pub const LFS_ERR_EXIST: i32 = -17;
pub const LFS_ERR_NOTDIR: i32 = -20;
pub const LFS_ERR_ISDIR: i32 = -21;
pub const LFS_ERR_NOTEMPTY: i32 = -39;
pub const LFS_ERR_BADF: i32 = -9;
pub const LFS_ERR_FBIG: i32 = -27;
pub const LFS_ERR_INVAL: i32 = -22;
pub const LFS_ERR_NOSPC: i32 = -28;
pub const LFS_ERR_NOMEM: i32 = -12;
pub const LFS_ERR_NOATTR: i32 = -61;
pub const LFS_ERR_NAMETOOLONG: i32 = -36;
