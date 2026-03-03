//! Metadata directory. Per lfs.h lfs_mdir_t.

use crate::types::lfs_block_t;

/// Per lfs.h typedef struct lfs_mdir
#[repr(C)]
pub struct LfsMdir {
    pub pair: [lfs_block_t; 2],
    pub rev: u32,
    pub off: u32,
    pub etag: u32,
    pub count: u16,
    pub erased: bool,
    pub split: bool,
    pub tail: [lfs_block_t; 2],
}
