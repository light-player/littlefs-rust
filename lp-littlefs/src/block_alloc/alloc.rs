//! Block allocator. Per lfs.c lfs_alloc, lfs_alloc_scan, lfs_alloc_lookahead, etc.

use crate::fs::Lfs;
use crate::types::lfs_block_t;

/// Per lfs.c lfs_alloc_ckpoint (lines 614-616)
///
/// C:
/// ```c
/// static void lfs_alloc_ckpoint(lfs_t *lfs) {
///     lfs->lookahead.ckpoint = lfs->block_count;
/// }
/// ```
pub fn lfs_alloc_ckpoint(lfs: *mut Lfs) {
    unsafe {
        let lfs = &mut *lfs;
        lfs.lookahead.ckpoint = lfs.block_count;
    }
}

/// Per lfs.c lfs_alloc_drop (lines 620-624)
///
/// C:
/// ```c
/// static void lfs_alloc_drop(lfs_t *lfs) {
///     lfs->lookahead.size = 0;
///     lfs->lookahead.next = 0;
///     lfs_alloc_ckpoint(lfs);
/// }
/// ```
pub fn lfs_alloc_drop(_lfs: *mut Lfs) {
    todo!("lfs_alloc_drop")
}

/// Per lfs.c lfs_alloc_lookahead (lines 627-637)
///
/// C:
/// ```c
/// #ifndef LFS_READONLY
/// static int lfs_alloc_lookahead(void *p, lfs_block_t block) {
///     lfs_t *lfs = (lfs_t*)p;
///     lfs_block_t off = ((block - lfs->lookahead.start)
///             + lfs->block_count) % lfs->block_count;
///
///     if (off < lfs->lookahead.size) {
///         lfs->lookahead.buffer[off / 8] |= 1U << (off % 8);
///     }
///
///     return 0;
/// }
/// #endif
/// ```
pub fn lfs_alloc_lookahead(_p: *mut Lfs, _block: lfs_block_t) -> i32 {
    todo!("lfs_alloc_lookahead")
}

/// Per lfs.c lfs_alloc_scan (lines 641-663)
///
/// C:
/// ```c
/// #ifndef LFS_READONLY
/// static int lfs_alloc_scan(lfs_t *lfs) {
///     // move lookahead buffer to the first unused block
///     //
///     // note we limit the lookahead buffer to at most the amount of blocks
///     // checkpointed, this prevents the math in lfs_alloc from underflowing
///     lfs->lookahead.start = (lfs->lookahead.start + lfs->lookahead.next)
///             % lfs->block_count;
///     lfs->lookahead.next = 0;
///     lfs->lookahead.size = lfs_min(
///             8*lfs->cfg->lookahead_size,
///             lfs->lookahead.ckpoint);
///
///     // find mask of free blocks from tree
///     memset(lfs->lookahead.buffer, 0, lfs->cfg->lookahead_size);
///     int err = lfs_fs_traverse_(lfs, lfs_alloc_lookahead, lfs, true);
///     if (err) {
///         lfs_alloc_drop(lfs);
///         return err;
///     }
///
///     return 0;
/// }
/// #endif
/// ```
pub fn lfs_alloc_scan(_lfs: *mut Lfs) -> i32 {
    todo!("lfs_alloc_scan")
}

/// Per lfs.c lfs_alloc (lines 666-716)
///
/// C:
/// ```c
/// #ifndef LFS_READONLY
/// static int lfs_alloc(lfs_t *lfs, lfs_block_t *block) {
///     while (true) {
///         // scan our lookahead buffer for free blocks
///         while (lfs->lookahead.next < lfs->lookahead.size) {
///             if (!(lfs->lookahead.buffer[lfs->lookahead.next / 8]
///                     & (1U << (lfs->lookahead.next % 8)))) {
///                 // found a free block
///                 *block = (lfs->lookahead.start + lfs->lookahead.next)
///                         % lfs->block_count;
///
///                 // eagerly find next free block to maximize how many blocks
///                 // lfs_alloc_ckpoint makes available for scanning
///                 while (true) {
///                     lfs->lookahead.next += 1;
///                     lfs->lookahead.ckpoint -= 1;
///
///                     if (lfs->lookahead.next >= lfs->lookahead.size
///                             || !(lfs->lookahead.buffer[lfs->lookahead.next / 8]
///                                 & (1U << (lfs->lookahead.next % 8)))) {
///                         return 0;
///                     }
///                 }
///             }
///
///             lfs->lookahead.next += 1;
///             lfs->lookahead.ckpoint -= 1;
///         }
///
///         // In order to keep our block allocator from spinning forever when our
///         // filesystem is full, we mark points where there are no in-flight
///         // allocations with a checkpoint before starting a set of allocations.
///         //
///         // If we've looked at all blocks since the last checkpoint, we report
///         // the filesystem as out of storage.
///         //
///         if (lfs->lookahead.ckpoint <= 0) {
///             LFS_ERROR("No more free space 0x%"PRIx32,
///                     (lfs->lookahead.start + lfs->lookahead.next)
///                         % lfs->block_count);
///             return LFS_ERR_NOSPC;
///         }
///
///         // No blocks in our lookahead buffer, we need to scan the filesystem for
///         // unused blocks in the next lookahead window.
///         int err = lfs_alloc_scan(lfs);
///         if(err) {
///             return err;
///         }
///     }
/// }
/// #endif
/// ```
pub fn lfs_alloc(lfs: *mut Lfs, block: *mut lfs_block_t) -> i32 {
    use crate::error::LFS_ERR_NOSPC;

    unsafe {
        let lfs = &mut *lfs;
        let cfg = &*lfs.cfg;
        let buf = lfs.lookahead.buffer;
        if buf.is_null() {
            return LFS_ERR_NOSPC;
        }

        loop {
            while lfs.lookahead.next < lfs.lookahead.size {
                let byte_idx = (lfs.lookahead.next / 8) as usize;
                let bit_mask = 1u8 << (lfs.lookahead.next % 8);
                let used = (*buf.add(byte_idx)) & bit_mask != 0;

                if !used {
                    *block = (lfs.lookahead.start + lfs.lookahead.next) % lfs.block_count;

                    loop {
                        lfs.lookahead.next += 1;
                        lfs.lookahead.ckpoint = lfs.lookahead.ckpoint.wrapping_sub(1);

                        if lfs.lookahead.next >= lfs.lookahead.size {
                            return 0;
                        }
                        let next_byte = (lfs.lookahead.next / 8) as usize;
                        let next_bit = 1u8 << (lfs.lookahead.next % 8);
                        if (*buf.add(next_byte)) & next_bit == 0 {
                            return 0;
                        }
                    }
                }

                lfs.lookahead.next += 1;
                lfs.lookahead.ckpoint = lfs.lookahead.ckpoint.wrapping_sub(1);
            }

            if lfs.lookahead.ckpoint == 0 {
                crate::lfs_error!(
                    "No more free space 0x{:08x}",
                    (lfs.lookahead.start + lfs.lookahead.next) % lfs.block_count
                );
                return LFS_ERR_NOSPC;
            }

            let err = lfs_alloc_scan(lfs);
            if err != 0 {
                return err;
            }
        }
    }
}
