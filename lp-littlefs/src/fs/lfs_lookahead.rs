//! Lookahead buffer. Per lfs.h struct lfs_lookahead.

use crate::types::lfs_block_t;

/// Per lfs.h struct lfs_lookahead
#[repr(C)]
pub struct LfsLookahead {
    pub start: lfs_block_t,
    pub size: lfs_block_t,
    pub next: lfs_block_t,
    pub ckpoint: lfs_block_t,
    pub buffer: *mut u8,
}
