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
pub fn lfs_alloc_drop(lfs: *mut Lfs) {
    unsafe {
        (*lfs).lookahead.size = 0;
        (*lfs).lookahead.next = 0;
        lfs_alloc_ckpoint(lfs);
    }
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
/// Callback wrapper for lfs_fs_traverse_: C expects (void* data, block), we pass lfs as data.
unsafe extern "C" fn lfs_alloc_lookahead_cb(
    data: *mut core::ffi::c_void,
    block: lfs_block_t,
) -> i32 {
    lfs_alloc_lookahead(data as *mut Lfs, block)
}

pub fn lfs_alloc_lookahead(p: *mut Lfs, block: lfs_block_t) -> i32 {
    unsafe {
        let lfs = &mut *p;
        // off = ((block - start) + block_count) % block_count
        let off = (block.wrapping_sub(lfs.lookahead.start)).wrapping_add(lfs.block_count)
            % lfs.block_count;

        if off < lfs.lookahead.size {
            let buf = lfs.lookahead.buffer;
            if !buf.is_null() {
                // buffer[off/8] |= 1 << (off%8)
                let byte_idx = (off / 8) as usize;
                let bit = 1u8 << (off % 8);
                *buf.add(byte_idx) |= bit;
            }
        }
        0
    }
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
pub fn lfs_alloc_scan(lfs: *mut Lfs) -> i32 {
    use crate::fs::traverse::lfs_fs_traverse_;
    use crate::util::lfs_min;

    unsafe {
        let lfs_ref = &mut *lfs;
        let cfg = lfs_ref.cfg.as_ref().expect("cfg");
        let buf = lfs_ref.lookahead.buffer;
        if buf.is_null() {
            return crate::error::LFS_ERR_NOSPC;
        }

        // move lookahead buffer to the first unused block
        lfs_ref.lookahead.start =
            (lfs_ref.lookahead.start + lfs_ref.lookahead.next) % lfs_ref.block_count;
        lfs_ref.lookahead.next = 0;
        // note we limit the lookahead buffer to at most the amount of blocks
        // checkpointed, this prevents the math in lfs_alloc from underflowing
        lfs_ref.lookahead.size = lfs_min(8 * cfg.lookahead_size, lfs_ref.lookahead.ckpoint);

        // find mask of free blocks from tree
        core::ptr::write_bytes(buf, 0, cfg.lookahead_size as usize);

        let err = lfs_fs_traverse_(
            lfs,
            Some(lfs_alloc_lookahead_cb),
            lfs as *mut core::ffi::c_void,
            true,
        );
        if err != 0 {
            lfs_alloc_drop(lfs);
            return err;
        }
        0
    }
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
        let buf = lfs.lookahead.buffer;
        if buf.is_null() {
            return LFS_ERR_NOSPC;
        }

        loop {
            // scan our lookahead buffer for free blocks
            while lfs.lookahead.next < lfs.lookahead.size {
                if (*buf.add((lfs.lookahead.next / 8) as usize)) & (1u8 << (lfs.lookahead.next % 8))
                    == 0
                {
                    // found a free block
                    *block = (lfs.lookahead.start + lfs.lookahead.next) % lfs.block_count;

                    // eagerly find next free block to maximize how many blocks
                    // lfs_alloc_ckpoint makes available for scanning
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

            // In order to keep our block allocator from spinning forever when our
            // filesystem is full, we mark points where there are no in-flight
            // allocations with a checkpoint before starting a set of allocations.
            // If we've looked at all blocks since the last checkpoint, we report
            // the filesystem as out of storage.
            if lfs.lookahead.ckpoint == 0 {
                crate::lfs_trace!(
                    "No more free space 0x{:08x} (ckpoint==0)",
                    (lfs.lookahead.start + lfs.lookahead.next) % lfs.block_count
                );
                return LFS_ERR_NOSPC;
            }

            // No blocks in our lookahead buffer, we need to scan the filesystem for
            // unused blocks in the next lookahead window.
            let err = lfs_alloc_scan(lfs);
            if err != 0 {
                crate::lfs_trace!(
                    "lfs_alloc NOSPC: alloc_scan returned {} start={} next={}",
                    err,
                    lfs.lookahead.start,
                    lfs.lookahead.next
                );
                return err;
            }
        }
    }
}
