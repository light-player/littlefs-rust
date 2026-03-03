//! Block device operations. Per lfs.c lfs_bd_read, lfs_bd_prog, lfs_bd_crc, etc.

use crate::bd::LfsCache;
use crate::types::{lfs_block_t, lfs_off_t, lfs_size_t};

/// Per lfs.c lfs_cache_drop (lines 33-37)
///
/// C:
/// ```c
/// static inline void lfs_cache_drop(lfs_t *lfs, lfs_cache_t *rcache) {
///     (void)lfs;
///     rcache->block = LFS_BLOCK_NULL;
/// }
/// ```
#[inline(always)]
pub fn lfs_cache_drop(_lfs: *const core::ffi::c_void, rcache: *mut LfsCache) {
    unsafe {
        (*rcache).block = crate::types::LFS_BLOCK_NULL;
    }
}

/// Per lfs.c lfs_cache_zero (lines 40-44)
///
/// C:
/// ```c
/// static inline void lfs_cache_zero(lfs_t *lfs, lfs_cache_t *pcache) {
///     // zero to avoid information leak
///     memset(pcache->buffer, 0xff, lfs->cfg->cache_size);
///     pcache->block = LFS_BLOCK_NULL;
/// }
/// ```
#[inline(always)]
pub fn lfs_cache_zero(_lfs: *const core::ffi::c_void, _pcache: *mut LfsCache) {
    todo!("lfs_cache_zero")
}

/// Per lfs.c lfs_bd_read (lines 46-127)
///
/// C:
/// ```c
/// static int lfs_bd_read(lfs_t *lfs,
///         const lfs_cache_t *pcache, lfs_cache_t *rcache, lfs_size_t hint,
///         lfs_block_t block, lfs_off_t off,
///         void *buffer, lfs_size_t size) {
///     uint8_t *data = buffer;
///     if (off+size > lfs->cfg->block_size
///             || (lfs->block_count && block >= lfs->block_count)) {
///         return LFS_ERR_CORRUPT;
///     }
///     while (size > 0) {
///         // check pcache, rcache, bypass or load to rcache
///         // ... (see lfs.c:56-125)
///     }
///     return 0;
/// }
/// ```
pub fn lfs_bd_read(
    _lfs: *const core::ffi::c_void,
    _pcache: *const LfsCache,
    _rcache: *mut LfsCache,
    _hint: lfs_size_t,
    _block: lfs_block_t,
    _off: lfs_off_t,
    _buffer: *mut u8,
    _size: lfs_size_t,
) -> i32 {
    todo!("lfs_bd_read")
}
