//! File and filesystem info. Per lfs.h struct lfs_info, lfs_fsinfo, lfs_attr, lfs_file_config.

use crate::types::lfs_size_t;
use core::ffi::c_void;

/// Per lfs.h struct lfs_info
#[repr(C)]
pub struct LfsInfo {
    pub type_: u8,
    pub size: lfs_size_t,
    pub name: [u8; 256], // LFS_NAME_MAX+1
}

/// Per lfs.h struct lfs_fsinfo
#[repr(C)]
pub struct LfsFsinfo {
    pub disk_version: u32,
    pub block_size: lfs_size_t,
    pub block_count: lfs_size_t,
    pub name_max: lfs_size_t,
    pub file_max: lfs_size_t,
    pub attr_max: lfs_size_t,
}

/// Per lfs.h struct lfs_attr
#[repr(C)]
pub struct LfsAttr {
    pub type_: u8,
    pub buffer: *mut c_void,
    pub size: lfs_size_t,
}

/// Per lfs.h struct lfs_file_config
#[repr(C)]
pub struct LfsFileConfig {
    pub buffer: *mut c_void,
    pub attrs: *mut LfsAttr,
    pub attr_count: lfs_size_t,
}
