//! Block device operations. Per lfs.c lfs_bd_read, lfs_bd_prog, lfs_bd_crc, etc.

use crate::bd::LfsCache;
use crate::types::{lfs_block_t, lfs_off_t, lfs_size_t};

/// Per lfs.c lfs_cache_drop (lines 31-36)
///
/// C:
/// ```c
/// static inline void lfs_cache_drop(lfs_t *lfs, lfs_cache_t *rcache) {
///     // do not zero, cheaper if cache is readonly or only going to be
///     // written with identical data (during relocates)
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

/// Per lfs.c lfs_cache_zero (lines 38-42)
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

/// Per lfs.c lfs_bd_read (lines 44-126)
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
///
///     while (size > 0) {
///         lfs_size_t diff = size;
///
///         if (pcache && block == pcache->block &&
///                 off < pcache->off + pcache->size) {
///             if (off >= pcache->off) {
///                 // is already in pcache?
///                 diff = lfs_min(diff, pcache->size - (off-pcache->off));
///                 memcpy(data, &pcache->buffer[off-pcache->off], diff);
///
///                 data += diff;
///                 off += diff;
///                 size -= diff;
///                 continue;
///             }
///
///             // pcache takes priority
///             diff = lfs_min(diff, pcache->off-off);
///         }
///
///         if (block == rcache->block &&
///                 off < rcache->off + rcache->size) {
///             if (off >= rcache->off) {
///                 // is already in rcache?
///                 diff = lfs_min(diff, rcache->size - (off-rcache->off));
///                 memcpy(data, &rcache->buffer[off-rcache->off], diff);
///
///                 data += diff;
///                 off += diff;
///                 size -= diff;
///                 continue;
///             }
///
///             // rcache takes priority
///             diff = lfs_min(diff, rcache->off-off);
///         }
///
///         if (size >= hint && off % lfs->cfg->read_size == 0 &&
///                 size >= lfs->cfg->read_size) {
///             // bypass cache?
///             diff = lfs_aligndown(diff, lfs->cfg->read_size);
///             int err = lfs->cfg->read(lfs->cfg, block, off, data, diff);
///             LFS_ASSERT(err <= 0);
///             if (err) {
///                 return err;
///             }
///
///             data += diff;
///             off += diff;
///             size -= diff;
///             continue;
///         }
///
///         // load to cache, first condition can no longer fail
///         LFS_ASSERT(!lfs->block_count || block < lfs->block_count);
///         rcache->block = block;
///         rcache->off = lfs_aligndown(off, lfs->cfg->read_size);
///         rcache->size = lfs_min(
///                 lfs_min(
///                     lfs_alignup(off+hint, lfs->cfg->read_size),
///                     lfs->cfg->block_size)
///                 - rcache->off,
///                 lfs->cfg->cache_size);
///         int err = lfs->cfg->read(lfs->cfg, rcache->block,
///                 rcache->off, rcache->buffer, rcache->size);
///         LFS_ASSERT(err <= 0);
///         if (err) {
///             return err;
///         }
///     }
///
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

/// Per lfs.c lfs_bd_cmp (lines 128-154)
///
/// C:
/// ```c
/// static int lfs_bd_cmp(lfs_t *lfs,
///         const lfs_cache_t *pcache, lfs_cache_t *rcache, lfs_size_t hint,
///         lfs_block_t block, lfs_off_t off,
///         const void *buffer, lfs_size_t size) {
///     const uint8_t *data = buffer;
///     lfs_size_t diff = 0;
///
///     for (lfs_off_t i = 0; i < size; i += diff) {
///         uint8_t dat[8];
///
///         diff = lfs_min(size-i, sizeof(dat));
///         int err = lfs_bd_read(lfs,
///                 pcache, rcache, hint-i,
///                 block, off+i, &dat, diff);
///         if (err) {
///             return err;
///         }
///
///         int res = memcmp(dat, data + i, diff);
///         if (res) {
///             return res < 0 ? LFS_CMP_LT : LFS_CMP_GT;
///         }
///     }
///
///     return LFS_CMP_EQ;
/// }
/// ```
pub fn lfs_bd_cmp(
    _lfs: *const core::ffi::c_void,
    _pcache: *const LfsCache,
    _rcache: *mut LfsCache,
    _hint: lfs_size_t,
    _block: lfs_block_t,
    _off: lfs_off_t,
    _buffer: *const u8,
    _size: lfs_size_t,
) -> i32 {
    todo!("lfs_bd_cmp")
}

/// Per lfs.c lfs_bd_crc (lines 155-175)
///
/// C:
/// ```c
/// static int lfs_bd_crc(lfs_t *lfs,
///         const lfs_cache_t *pcache, lfs_cache_t *rcache, lfs_size_t hint,
///         lfs_block_t block, lfs_off_t off, lfs_size_t size, uint32_t *crc) {
///     lfs_size_t diff = 0;
///
///     for (lfs_off_t i = 0; i < size; i += diff) {
///         uint8_t dat[8];
///         diff = lfs_min(size-i, sizeof(dat));
///         int err = lfs_bd_read(lfs,
///                 pcache, rcache, hint-i,
///                 block, off+i, &dat, diff);
///         if (err) {
///             return err;
///         }
///
///         *crc = lfs_crc(*crc, &dat, diff);
///     }
///
///     return 0;
/// }
/// ```
pub fn lfs_bd_crc(
    _lfs: *const core::ffi::c_void,
    _pcache: *const LfsCache,
    _rcache: *mut LfsCache,
    _hint: lfs_size_t,
    _block: lfs_block_t,
    _off: lfs_off_t,
    _size: lfs_size_t,
    _crc: *mut u32,
) -> i32 {
    todo!("lfs_bd_crc")
}

/// Per lfs.c lfs_bd_flush (lines 177-210)
///
/// C:
/// ```c
/// #ifndef LFS_READONLY
/// static int lfs_bd_flush(lfs_t *lfs,
///         lfs_cache_t *pcache, lfs_cache_t *rcache, bool validate) {
///     if (pcache->block != LFS_BLOCK_NULL && pcache->block != LFS_BLOCK_INLINE) {
///         LFS_ASSERT(pcache->block < lfs->block_count);
///         lfs_size_t diff = lfs_alignup(pcache->size, lfs->cfg->prog_size);
///         int err = lfs->cfg->prog(lfs->cfg, pcache->block,
///                 pcache->off, pcache->buffer, diff);
///         LFS_ASSERT(err <= 0);
///         if (err) {
///             return err;
///         }
///
///         if (validate) {
///             // check data on disk
///             lfs_cache_drop(lfs, rcache);
///             int res = lfs_bd_cmp(lfs,
///                     NULL, rcache, diff,
///                     pcache->block, pcache->off, pcache->buffer, diff);
///             if (res < 0) {
///                 return res;
///             }
///
///             if (res != LFS_CMP_EQ) {
///                 return LFS_ERR_CORRUPT;
///             }
///         }
///
///         lfs_cache_zero(lfs, pcache);
///     }
///
///     return 0;
/// }
/// #endif
/// ```
pub fn lfs_bd_flush(
    _lfs: *const core::ffi::c_void,
    _pcache: *mut LfsCache,
    _rcache: *mut LfsCache,
    _validate: bool,
) -> i32 {
    todo!("lfs_bd_flush")
}

/// Per lfs.c lfs_bd_sync (lines 213-226)
///
/// C:
/// ```c
/// #ifndef LFS_READONLY
/// static int lfs_bd_sync(lfs_t *lfs,
///         lfs_cache_t *pcache, lfs_cache_t *rcache, bool validate) {
///     lfs_cache_drop(lfs, rcache);
///
///     int err = lfs_bd_flush(lfs, pcache, rcache, validate);
///     if (err) {
///         return err;
///     }
///
///     err = lfs->cfg->sync(lfs->cfg);
///     LFS_ASSERT(err <= 0);
///     return err;
/// }
/// #endif
/// ```
pub fn lfs_bd_sync(
    _lfs: *const core::ffi::c_void,
    _pcache: *mut LfsCache,
    _rcache: *mut LfsCache,
    _validate: bool,
) -> i32 {
    todo!("lfs_bd_sync")
}

/// Per lfs.c lfs_bd_prog (lines 228-274)
///
/// C:
/// ```c
/// #ifndef LFS_READONLY
/// static int lfs_bd_prog(lfs_t *lfs,
///         lfs_cache_t *pcache, lfs_cache_t *rcache, bool validate,
///         lfs_block_t block, lfs_off_t off,
///         const void *buffer, lfs_size_t size) {
///     const uint8_t *data = buffer;
///     LFS_ASSERT(block == LFS_BLOCK_INLINE || block < lfs->block_count);
///     LFS_ASSERT(off + size <= lfs->cfg->block_size);
///
///     while (size > 0) {
///         if (block == pcache->block &&
///                 off >= pcache->off &&
///                 off < pcache->off + lfs->cfg->cache_size) {
///             // already fits in pcache?
///             lfs_size_t diff = lfs_min(size,
///                     lfs->cfg->cache_size - (off-pcache->off));
///             memcpy(&pcache->buffer[off-pcache->off], data, diff);
///
///             data += diff;
///             off += diff;
///             size -= diff;
///
///             pcache->size = lfs_max(pcache->size, off - pcache->off);
///             if (pcache->size == lfs->cfg->cache_size) {
///                 // eagerly flush out pcache if we fill up
///                 int err = lfs_bd_flush(lfs, pcache, rcache, validate);
///                 if (err) {
///                     return err;
///                 }
///             }
///
///             continue;
///         }
///
///         // pcache must have been flushed, either by programming and
///         // entire block or manually flushing the pcache
///         LFS_ASSERT(pcache->block == LFS_BLOCK_NULL);
///
///         // prepare pcache, first condition can no longer fail
///         pcache->block = block;
///         pcache->off = lfs_aligndown(off, lfs->cfg->prog_size);
///         pcache->size = 0;
///     }
///
///     return 0;
/// }
/// #endif
/// ```
pub fn lfs_bd_prog(
    _lfs: *const core::ffi::c_void,
    _pcache: *mut LfsCache,
    _rcache: *mut LfsCache,
    _validate: bool,
    _block: lfs_block_t,
    _off: lfs_off_t,
    _buffer: *const u8,
    _size: lfs_size_t,
) -> i32 {
    todo!("lfs_bd_prog")
}

/// Per lfs.c lfs_bd_erase (lines 277-282)
///
/// C:
/// ```c
/// #ifndef LFS_READONLY
/// static int lfs_bd_erase(lfs_t *lfs, lfs_block_t block) {
///     LFS_ASSERT(block < lfs->block_count);
///     int err = lfs->cfg->erase(lfs->cfg, block);
///     LFS_ASSERT(err <= 0);
///     return err;
/// }
/// #endif
/// ```
pub fn lfs_bd_erase(_lfs: *const core::ffi::c_void, _block: lfs_block_t) -> i32 {
    todo!("lfs_bd_erase")
}
