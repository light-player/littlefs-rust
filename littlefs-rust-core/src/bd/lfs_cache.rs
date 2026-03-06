//! Block cache. Per lfs.h lfs_cache_t.

use crate::types::lfs_block_t;

/// Per lfs.h typedef struct lfs_cache
#[repr(C)]
pub struct LfsCache {
    pub block: lfs_block_t,
    pub off: u32,
    pub size: u32,
    pub buffer: *mut u8,
}
