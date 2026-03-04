//! Initialization. Per lfs.c lfs_init, lfs_deinit.

use crate::bd::bd::lfs_cache_zero;
use crate::types::{LFS_ATTR_MAX, LFS_BLOCK_NULL, LFS_FILE_MAX, LFS_NAME_MAX};
use crate::util::{lfs_min, lfs_npw2};

/// Per lfs.c lfs_init (lines 4198-4369)
///
/// C:
/// ```c
/// static int lfs_init(lfs_t *lfs, const struct lfs_config *cfg) {
///     lfs->cfg = cfg;
///     lfs->block_count = cfg->block_count;  // May be 0
///     int err = 0;
///
/// #ifdef LFS_MULTIVERSION
///     // this driver only supports minor version < current minor version
///     LFS_ASSERT(!lfs->cfg->disk_version || (
///             (0xffff & (lfs->cfg->disk_version >> 16))
///                     == LFS_DISK_VERSION_MAJOR
///                 && (0xffff & (lfs->cfg->disk_version >> 0))
///                     <= LFS_DISK_VERSION_MINOR));
/// #endif
///
///     // check that bool is a truthy-preserving type
///     //
///     // note the most common reason for this failure is a before-c99 compiler,
///     // which littlefs currently does not support
///     LFS_ASSERT((bool)0x80000000);
///
///     // check that the required io functions are provided
///     LFS_ASSERT(lfs->cfg->read != NULL);
/// #ifndef LFS_READONLY
///     LFS_ASSERT(lfs->cfg->prog != NULL);
///     LFS_ASSERT(lfs->cfg->erase != NULL);
///     LFS_ASSERT(lfs->cfg->sync != NULL);
/// #endif
///
///     // validate that the lfs-cfg sizes were initiated properly before
///     // performing any arithmetic logics with them
///     LFS_ASSERT(lfs->cfg->read_size != 0);
///     LFS_ASSERT(lfs->cfg->prog_size != 0);
///     LFS_ASSERT(lfs->cfg->cache_size != 0);
///
///     // check that block size is a multiple of cache size is a multiple
///     // of prog and read sizes
///     LFS_ASSERT(lfs->cfg->cache_size % lfs->cfg->read_size == 0);
///     LFS_ASSERT(lfs->cfg->cache_size % lfs->cfg->prog_size == 0);
///     LFS_ASSERT(lfs->cfg->block_size % lfs->cfg->cache_size == 0);
///
///     // check that the block size is large enough to fit all ctz pointers
///     LFS_ASSERT(lfs->cfg->block_size >= 128);
///     // this is the exact calculation for all ctz pointers, if this fails
///     // and the simpler assert above does not, math must be broken
///     LFS_ASSERT(4*lfs_npw2(0xffffffff / (lfs->cfg->block_size-2*4))
///             <= lfs->cfg->block_size);
///
///     // block_cycles = 0 is no longer supported.
///     //
///     // block_cycles is the number of erase cycles before littlefs evicts
///     // metadata logs as a part of wear leveling. Suggested values are in the
///     // range of 100-1000, or set block_cycles to -1 to disable block-level
///     // wear-leveling.
///     LFS_ASSERT(lfs->cfg->block_cycles != 0);
///
///     // check that compact_thresh makes sense
///     //
///     // metadata can't be compacted below block_size/2, and metadata can't
///     // exceed a block_size
///     LFS_ASSERT(lfs->cfg->compact_thresh == 0
///             || lfs->cfg->compact_thresh >= lfs->cfg->block_size/2);
///     LFS_ASSERT(lfs->cfg->compact_thresh == (lfs_size_t)-1
///             || lfs->cfg->compact_thresh <= lfs->cfg->block_size);
///
///     // check that metadata_max is a multiple of read_size and prog_size,
///     // and a factor of the block_size
///     LFS_ASSERT(!lfs->cfg->metadata_max
///             || lfs->cfg->metadata_max % lfs->cfg->read_size == 0);
///     LFS_ASSERT(!lfs->cfg->metadata_max
///             || lfs->cfg->metadata_max % lfs->cfg->prog_size == 0);
///     LFS_ASSERT(!lfs->cfg->metadata_max
///             || lfs->cfg->block_size % lfs->cfg->metadata_max == 0);
///
///     // setup read cache
///     if (lfs->cfg->read_buffer) {
///         lfs->rcache.buffer = lfs->cfg->read_buffer;
///     } else {
///         lfs->rcache.buffer = lfs_malloc(lfs->cfg->cache_size);
///         if (!lfs->rcache.buffer) {
///             err = LFS_ERR_NOMEM;
///             goto cleanup;
///         }
///     }
///
///     // setup program cache
///     if (lfs->cfg->prog_buffer) {
///         lfs->pcache.buffer = lfs->cfg->prog_buffer;
///     } else {
///         lfs->pcache.buffer = lfs_malloc(lfs->cfg->cache_size);
///         if (!lfs->pcache.buffer) {
///             err = LFS_ERR_NOMEM;
///             goto cleanup;
///         }
///     }
///
///     // zero to avoid information leaks
///     lfs_cache_zero(lfs, &lfs->rcache);
///     lfs_cache_zero(lfs, &lfs->pcache);
///
///     // setup lookahead buffer, note mount finishes initializing this after
///     // we establish a decent pseudo-random seed
///     LFS_ASSERT(lfs->cfg->lookahead_size > 0);
///     if (lfs->cfg->lookahead_buffer) {
///         lfs->lookahead.buffer = lfs->cfg->lookahead_buffer;
///     } else {
///         lfs->lookahead.buffer = lfs_malloc(lfs->cfg->lookahead_size);
///         if (!lfs->lookahead.buffer) {
///             err = LFS_ERR_NOMEM;
///             goto cleanup;
///         }
///     }
///
///     // check that the size limits are sane
///     LFS_ASSERT(lfs->cfg->name_max <= LFS_NAME_MAX);
///     lfs->name_max = lfs->cfg->name_max;
///     if (!lfs->name_max) {
///         lfs->name_max = LFS_NAME_MAX;
///     }
///
///     LFS_ASSERT(lfs->cfg->file_max <= LFS_FILE_MAX);
///     lfs->file_max = lfs->cfg->file_max;
///     if (!lfs->file_max) {
///         lfs->file_max = LFS_FILE_MAX;
///     }
///
///     LFS_ASSERT(lfs->cfg->attr_max <= LFS_ATTR_MAX);
///     lfs->attr_max = lfs->cfg->attr_max;
///     if (!lfs->attr_max) {
///         lfs->attr_max = LFS_ATTR_MAX;
///     }
///
///     LFS_ASSERT(lfs->cfg->metadata_max <= lfs->cfg->block_size);
///
///     LFS_ASSERT(lfs->cfg->inline_max == (lfs_size_t)-1
///             || lfs->cfg->inline_max <= lfs->cfg->cache_size);
///     LFS_ASSERT(lfs->cfg->inline_max == (lfs_size_t)-1
///             || lfs->cfg->inline_max <= lfs->attr_max);
///     LFS_ASSERT(lfs->cfg->inline_max == (lfs_size_t)-1
///             || lfs->cfg->inline_max <= ((lfs->cfg->metadata_max)
///                 ? lfs->cfg->metadata_max
///                 : lfs->cfg->block_size)/8);
///     lfs->inline_max = lfs->cfg->inline_max;
///     if (lfs->inline_max == (lfs_size_t)-1) {
///         lfs->inline_max = 0;
///     } else if (lfs->inline_max == 0) {
///         lfs->inline_max = lfs_min(
///                 lfs->cfg->cache_size,
///                 lfs_min(
///                     lfs->attr_max,
///                     ((lfs->cfg->metadata_max)
///                         ? lfs->cfg->metadata_max
///                         : lfs->cfg->block_size)/8));
///     }
///
///     // setup default state
///     lfs->root[0] = LFS_BLOCK_NULL;
///     lfs->root[1] = LFS_BLOCK_NULL;
///     lfs->mlist = NULL;
///     lfs->seed = 0;
///     lfs->gdisk = (lfs_gstate_t){0};
///     lfs->gstate = (lfs_gstate_t){0};
///     lfs->gdelta = (lfs_gstate_t){0};
/// #ifdef LFS_MIGRATE
///     lfs->lfs1 = NULL;
/// #endif
///
///     return 0;
///
/// cleanup:
///     lfs_deinit(lfs);
///     return err;
/// }
/// ```
pub fn lfs_init(lfs: *mut super::lfs::Lfs, cfg: *const crate::lfs_config::LfsConfig) -> i32 {
    unsafe {
        let lfs = &mut *lfs;
        let cfg = &*cfg;

        // check that bool is a truthy-preserving type (C: (bool)0x80000000)
        crate::lfs_assert!(0x8000_0000u32 != 0);

        // check that the required io functions are provided
        crate::lfs_assert!(cfg.read.is_some());
        crate::lfs_assert!(cfg.prog.is_some());
        crate::lfs_assert!(cfg.erase.is_some());
        crate::lfs_assert!(cfg.sync.is_some());

        // validate that the lfs-cfg sizes were initiated properly
        crate::lfs_assert!(cfg.read_size != 0);
        crate::lfs_assert!(cfg.prog_size != 0);
        crate::lfs_assert!(cfg.cache_size != 0);

        // check that block size is a multiple of cache size is a multiple of prog and read sizes
        crate::lfs_assert!(cfg.cache_size.is_multiple_of(cfg.read_size));
        crate::lfs_assert!(cfg.cache_size.is_multiple_of(cfg.prog_size));
        crate::lfs_assert!(cfg.block_size.is_multiple_of(cfg.cache_size));

        // check that the block size is large enough to fit all ctz pointers
        crate::lfs_assert!(cfg.block_size >= 128);
        crate::lfs_assert!(
            4 * lfs_npw2(0xffff_ffffu32 / (cfg.block_size - 2 * 4)) <= cfg.block_size,
            "ctz pointer math"
        );

        // block_cycles = 0 is no longer supported
        crate::lfs_assert!(cfg.block_cycles != 0);

        // check that compact_thresh makes sense
        crate::lfs_assert!(cfg.compact_thresh == 0 || cfg.compact_thresh >= cfg.block_size / 2);
        crate::lfs_assert!(cfg.compact_thresh == u32::MAX || cfg.compact_thresh <= cfg.block_size);

        // check that metadata_max is a multiple of read_size and prog_size, and a factor of block_size
        crate::lfs_assert!(cfg.metadata_max == 0 || cfg.metadata_max.is_multiple_of(cfg.read_size));
        crate::lfs_assert!(cfg.metadata_max == 0 || cfg.metadata_max.is_multiple_of(cfg.prog_size));
        crate::lfs_assert!(
            cfg.metadata_max == 0 || cfg.block_size.is_multiple_of(cfg.metadata_max)
        );

        lfs.cfg = cfg;
        lfs.block_count = cfg.block_count;

        lfs.rcache.buffer = cfg.read_buffer as *mut u8;
        lfs.pcache.buffer = cfg.prog_buffer as *mut u8;
        lfs_cache_zero(lfs, &mut lfs.rcache);
        lfs_cache_zero(lfs, &mut lfs.pcache);

        crate::lfs_assert!(cfg.lookahead_size > 0);
        lfs.lookahead.buffer = cfg.lookahead_buffer as *mut u8;

        crate::lfs_assert!(cfg.name_max <= LFS_NAME_MAX as u32);
        lfs.name_max = if cfg.name_max != 0 {
            cfg.name_max
        } else {
            LFS_NAME_MAX as u32
        };
        crate::lfs_assert!(cfg.file_max <= LFS_FILE_MAX as u32);
        lfs.file_max = if cfg.file_max != 0 {
            cfg.file_max
        } else {
            LFS_FILE_MAX as u32
        };
        crate::lfs_assert!(cfg.attr_max <= LFS_ATTR_MAX as u32);
        lfs.attr_max = if cfg.attr_max != 0 {
            cfg.attr_max
        } else {
            LFS_ATTR_MAX as u32
        };

        crate::lfs_assert!(cfg.metadata_max == 0 || cfg.metadata_max <= cfg.block_size);
        let metadata_max = if cfg.metadata_max != 0 {
            cfg.metadata_max
        } else {
            cfg.block_size
        };
        crate::lfs_assert!(cfg.inline_max == u32::MAX || cfg.inline_max <= cfg.cache_size);
        crate::lfs_assert!(cfg.inline_max == u32::MAX || cfg.inline_max <= lfs.attr_max);
        crate::lfs_assert!(cfg.inline_max == u32::MAX || cfg.inline_max <= metadata_max / 8);
        lfs.inline_max = if cfg.inline_max == u32::MAX {
            0
        } else if cfg.inline_max == 0 {
            lfs_min(cfg.cache_size, lfs_min(lfs.attr_max, metadata_max / 8))
        } else {
            cfg.inline_max
        };

        lfs.root = [LFS_BLOCK_NULL, LFS_BLOCK_NULL];
        lfs.mlist = core::ptr::null_mut();
        lfs.seed = 0;
        lfs.gstate = crate::lfs_gstate::LfsGstate {
            tag: 0,
            pair: [0, 0],
        };
        lfs.gdisk = crate::lfs_gstate::LfsGstate {
            tag: 0,
            pair: [0, 0],
        };
        lfs.gdelta = crate::lfs_gstate::LfsGstate {
            tag: 0,
            pair: [0, 0],
        };
    }
    0
}

/// Per lfs.c lfs_deinit (lines 4371-4389)
///
/// C:
/// ```c
/// static int lfs_deinit(lfs_t *lfs) {
///     // free allocated memory
///     if (!lfs->cfg->read_buffer) {
///         lfs_free(lfs->rcache.buffer);
///     }
///
///     if (!lfs->cfg->prog_buffer) {
///         lfs_free(lfs->pcache.buffer);
///     }
///
///     if (!lfs->cfg->lookahead_buffer) {
///         lfs_free(lfs->lookahead.buffer);
///     }
///
///     return 0;
/// }
///
///
///
/// ```
pub fn lfs_deinit(_lfs: *mut super::lfs::Lfs) -> i32 {
    // With provided buffers (read_buffer, prog_buffer, lookahead_buffer), no free needed
    0
}
