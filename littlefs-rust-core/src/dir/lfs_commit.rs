//! Commit state. Per lfs.c struct lfs_commit.

use crate::types::lfs_tag_t;
use crate::types::{lfs_block_t, lfs_off_t};

/// Per lfs.c struct lfs_commit
#[repr(C)]
pub struct LfsCommit {
    pub block: lfs_block_t,
    pub off: lfs_off_t,
    pub ptag: lfs_tag_t,
    pub crc: u32,
    pub begin: lfs_off_t,
    pub end: lfs_off_t,
}
