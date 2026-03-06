//! Superblock. Per lfs.h lfs_superblock_t.

use crate::types::lfs_size_t;
use crate::util::{lfs_fromle32, lfs_tole32};

/// Per lfs.h typedef struct lfs_superblock
#[repr(C)]
pub struct LfsSuperblock {
    pub version: u32,
    pub block_size: lfs_size_t,
    pub block_count: lfs_size_t,
    pub name_max: lfs_size_t,
    pub file_max: lfs_size_t,
    pub attr_max: lfs_size_t,
}

/// Per lfs.c lfs_superblock_fromle32 (lines 487-494)
///
/// C:
/// ```c
/// static inline void lfs_superblock_fromle32(lfs_superblock_t *superblock) {
///     superblock->version     = lfs_fromle32(superblock->version);
///     superblock->block_size  = lfs_fromle32(superblock->block_size);
///     superblock->block_count = lfs_fromle32(superblock->block_count);
///     superblock->name_max    = lfs_fromle32(superblock->name_max);
///     superblock->file_max    = lfs_fromle32(superblock->file_max);
///     superblock->attr_max    = lfs_fromle32(superblock->attr_max);
/// }
/// ```
#[inline(always)]
pub fn lfs_superblock_fromle32(sb: &mut LfsSuperblock) {
    sb.version = lfs_fromle32(sb.version);
    sb.block_size = lfs_fromle32(sb.block_size);
    sb.block_count = lfs_fromle32(sb.block_count);
    sb.name_max = lfs_fromle32(sb.name_max);
    sb.file_max = lfs_fromle32(sb.file_max);
    sb.attr_max = lfs_fromle32(sb.attr_max);
}

/// Per lfs.c lfs_superblock_tole32 (lines 497-505)
///
/// C:
/// ```c
/// static inline void lfs_superblock_tole32(lfs_superblock_t *superblock) {
///     superblock->version     = lfs_tole32(superblock->version);
///     superblock->block_size  = lfs_tole32(superblock->block_size);
///     superblock->block_count = lfs_tole32(superblock->block_count);
///     superblock->name_max    = lfs_tole32(superblock->name_max);
///     superblock->file_max    = lfs_tole32(superblock->file_max);
///     superblock->attr_max    = lfs_tole32(superblock->attr_max);
/// }
/// ```
#[inline(always)]
pub fn lfs_superblock_tole32(sb: &mut LfsSuperblock) {
    sb.version = lfs_tole32(sb.version);
    sb.block_size = lfs_tole32(sb.block_size);
    sb.block_count = lfs_tole32(sb.block_count);
    sb.name_max = lfs_tole32(sb.name_max);
    sb.file_max = lfs_tole32(sb.file_max);
    sb.attr_max = lfs_tole32(sb.attr_max);
}
