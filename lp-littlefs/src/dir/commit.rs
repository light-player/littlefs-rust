//! Directory commit. Per lfs.c lfs_dir_commit, lfs_dir_commitattr, lfs_dir_alloc, etc.

use crate::dir::LfsCommit;
use crate::dir::LfsMdir;
use crate::types::{lfs_block_t, lfs_size_t, lfs_tag_t};

/// Per lfs.c lfs_dir_commitprog (lines 1604-1618)
///
/// C:
/// ```c
/// static int lfs_dir_commitprog(lfs_t *lfs, struct lfs_commit *commit,
///         const void *buffer, lfs_size_t size) {
///     int err = lfs_bd_prog(lfs,
///             &lfs->pcache, &lfs->rcache, false,
///             commit->block, commit->off ,
///             (const uint8_t*)buffer, size);
///     if (err) {
///         return err;
///     }
///
///     commit->crc = lfs_crc(commit->crc, buffer, size);
///     commit->off += size;
///     return 0;
/// }
/// ```
pub fn lfs_dir_commitprog(
    _lfs: *const core::ffi::c_void,
    _commit: *mut LfsCommit,
    _buffer: *const core::ffi::c_void,
    _size: lfs_size_t,
) -> i32 {
    todo!("lfs_dir_commitprog")
}

/// Per lfs.c lfs_dir_commitattr (lines 1621-1666)
///
/// C:
/// ```c
/// static int lfs_dir_commitattr(lfs_t *lfs, struct lfs_commit *commit,
///         lfs_tag_t tag, const void *buffer) {
///     // check if we fit
///     lfs_size_t dsize = lfs_tag_dsize(tag);
///     if (commit->off + dsize > commit->end) {
///         return LFS_ERR_NOSPC;
///     }
///
///     // write out tag
///     lfs_tag_t ntag = lfs_tobe32((tag & 0x7fffffff) ^ commit->ptag);
///     int err = lfs_dir_commitprog(lfs, commit, &ntag, sizeof(ntag));
///     if (err) {
///         return err;
///     }
///
///     if (!(tag & 0x80000000)) {
///         // from memory
///         err = lfs_dir_commitprog(lfs, commit, buffer, dsize-sizeof(tag));
///         if (err) {
///             return err;
///         }
///     } else {
///         // from disk
///         const struct lfs_diskoff *disk = buffer;
///         for (lfs_off_t i = 0; i < dsize-sizeof(tag); i++) {
///             // rely on caching to make this efficient
///             uint8_t dat;
///             err = lfs_bd_read(lfs,
///                     NULL, &lfs->rcache, dsize-sizeof(tag)-i,
///                     disk->block, disk->off+i, &dat, 1);
///             if (err) {
///                 return err;
///             }
///
///             err = lfs_dir_commitprog(lfs, commit, &dat, 1);
///             if (err) {
///                 return err;
///             }
///         }
///     }
///
///     commit->ptag = tag & 0x7fffffff;
///     return 0;
/// }
/// ```
pub fn lfs_dir_commitattr(
    _lfs: *const core::ffi::c_void,
    _commit: *mut LfsCommit,
    _tag: lfs_tag_t,
    _buffer: *const core::ffi::c_void,
) -> i32 {
    todo!("lfs_dir_commitattr")
}

/// Per lfs.c lfs_dir_commitcrc (lines 1669-1812)
///
/// C:
/// ```c
/// static int lfs_dir_commitcrc(lfs_t *lfs, struct lfs_commit *commit) {
///     // align to program units
///     //
///     // this gets a bit complex as we have two types of crcs:
///     // - 5-word crc with fcrc to check following prog (middle of block)
///     // - 2-word crc with no following prog (end of block)
///     const lfs_off_t end = lfs_alignup(
///             lfs_min(commit->off + 5*sizeof(uint32_t), lfs->cfg->block_size),
///             lfs->cfg->prog_size);
///
///     lfs_off_t off1 = 0;
///     uint32_t crc1 = 0;
///
///     // create crc tags to fill up remainder of commit, note that
///     // padding is not crced, which lets fetches skip padding but
///     // makes committing a bit more complicated
///     while (commit->off < end) {
///         lfs_off_t noff = (
///                 lfs_min(end - (commit->off+sizeof(lfs_tag_t)), 0x3fe)
///                 + (commit->off+sizeof(lfs_tag_t)));
///         // too large for crc tag? need padding commits
///         if (noff < end) {
///             noff = lfs_min(noff, end - 5*sizeof(uint32_t));
///         }
///
///         // space for fcrc?
///         uint8_t eperturb = (uint8_t)-1;
///         if (noff >= end && noff <= lfs->cfg->block_size - lfs->cfg->prog_size) {
///             // first read the leading byte, this always contains a bit
///             // we can perturb to avoid writes that don't change the fcrc
///             int err = lfs_bd_read(lfs,
///                     NULL, &lfs->rcache, lfs->cfg->prog_size,
///                     commit->block, noff, &eperturb, 1);
///             if (err && err != LFS_ERR_CORRUPT) {
///                 return err;
///             }
///
///         #ifdef LFS_MULTIVERSION
///             // unfortunately fcrcs break mdir fetching < lfs2.1, so only write
///             // these if we're a >= lfs2.1 filesystem
///             if (lfs_fs_disk_version(lfs) <= 0x00020000) {
///                 // don't write fcrc
///             } else
///         #endif
///             {
///                 // find the expected fcrc, don't bother avoiding a reread
///                 // of the eperturb, it should still be in our cache
///                 struct lfs_fcrc fcrc = {
///                     .size = lfs->cfg->prog_size,
///                     .crc = 0xffffffff
///                 };
///                 err = lfs_bd_crc(lfs,
///                         NULL, &lfs->rcache, lfs->cfg->prog_size,
///                         commit->block, noff, fcrc.size, &fcrc.crc);
///                 if (err && err != LFS_ERR_CORRUPT) {
///                     return err;
///                 }
///
///                 lfs_fcrc_tole32(&fcrc);
///                 err = lfs_dir_commitattr(lfs, commit,
///                         LFS_MKTAG(LFS_TYPE_FCRC, 0x3ff, sizeof(struct lfs_fcrc)),
///                         &fcrc);
///                 if (err) {
///                     return err;
///                 }
///             }
///         }
///
///         // build commit crc
///         struct {
///             lfs_tag_t tag;
///             uint32_t crc;
///         } ccrc;
///         lfs_tag_t ntag = LFS_MKTAG(
///                 LFS_TYPE_CCRC + (((uint8_t)~eperturb) >> 7), 0x3ff,
///                 noff - (commit->off+sizeof(lfs_tag_t)));
///         ccrc.tag = lfs_tobe32(ntag ^ commit->ptag);
///         commit->crc = lfs_crc(commit->crc, &ccrc.tag, sizeof(lfs_tag_t));
///         ccrc.crc = lfs_tole32(commit->crc);
///
///         int err = lfs_bd_prog(lfs,
///                 &lfs->pcache, &lfs->rcache, false,
///                 commit->block, commit->off, &ccrc, sizeof(ccrc));
///         if (err) {
///             return err;
///         }
///
///         // keep track of non-padding checksum to verify
///         if (off1 == 0) {
///             off1 = commit->off + sizeof(lfs_tag_t);
///             crc1 = commit->crc;
///         }
///
///         commit->off = noff;
///         // perturb valid bit?
///         commit->ptag = ntag ^ ((0x80UL & ~eperturb) << 24);
///         // reset crc for next commit
///         commit->crc = 0xffffffff;
///
///         // manually flush here since we don't prog the padding, this confuses
///         // the caching layer
///         if (noff >= end || noff >= lfs->pcache.off + lfs->cfg->cache_size) {
///             // flush buffers
///             int err = lfs_bd_sync(lfs, &lfs->pcache, &lfs->rcache, false);
///             if (err) {
///                 return err;
///             }
///         }
///     }
///
///     // successful commit, check checksums to make sure
///     //
///     // note that we don't need to check padding commits, worst
///     // case if they are corrupted we would have had to compact anyways
///     lfs_off_t off = commit->begin;
///     uint32_t crc = 0xffffffff;
///     int err = lfs_bd_crc(lfs,
///             NULL, &lfs->rcache, off1+sizeof(uint32_t),
///             commit->block, off, off1-off, &crc);
///     if (err) {
///         return err;
///     }
///
///     // check non-padding commits against known crc
///     if (crc != crc1) {
///         return LFS_ERR_CORRUPT;
///     }
///
///     // make sure to check crc in case we happen to pick
///     // up an unrelated crc (frozen block?)
///     err = lfs_bd_crc(lfs,
///             NULL, &lfs->rcache, sizeof(uint32_t),
///             commit->block, off1, sizeof(uint32_t), &crc);
///     if (err) {
///         return err;
///     }
///
///     if (crc != 0) {
///         return LFS_ERR_CORRUPT;
///     }
///
///     return 0;
/// }
/// #endif
/// ```
pub fn lfs_dir_commitcrc(_lfs: *const core::ffi::c_void, _commit: *mut LfsCommit) -> i32 {
    todo!("lfs_dir_commitcrc")
}

/// Per lfs.c lfs_dir_alloc (lines 1815-1857)
///
/// C:
/// ```c
/// static int lfs_dir_alloc(lfs_t *lfs, lfs_mdir_t *dir) {
///     // allocate pair of dir blocks (backwards, so we write block 1 first)
///     for (int i = 0; i < 2; i++) {
///         int err = lfs_alloc(lfs, &dir->pair[(i+1)%2]);
///         if (err) {
///             return err;
///         }
///     }
///
///     // zero for reproducibility in case initial block is unreadable
///     dir->rev = 0;
///
///     // rather than clobbering one of the blocks we just pretend
///     // the revision may be valid
///     int err = lfs_bd_read(lfs,
///             NULL, &lfs->rcache, sizeof(dir->rev),
///             dir->pair[0], 0, &dir->rev, sizeof(dir->rev));
///     dir->rev = lfs_fromle32(dir->rev);
///     if (err && err != LFS_ERR_CORRUPT) {
///         return err;
///     }
///
///     // to make sure we don't immediately evict, align the new revision count
///     // to our block_cycles modulus, see lfs_dir_compact for why our modulus
///     // is tweaked this way
///     if (lfs->cfg->block_cycles > 0) {
///         dir->rev = lfs_alignup(dir->rev, ((lfs->cfg->block_cycles+1)|1));
///     }
///
///     // set defaults
///     dir->off = sizeof(dir->rev);
///     dir->etag = 0xffffffff;
///     dir->count = 0;
///     dir->tail[0] = LFS_BLOCK_NULL;
///     dir->tail[1] = LFS_BLOCK_NULL;
///     dir->erased = false;
///     dir->split = false;
///
///     // don't write out yet, let caller take care of that
///     return 0;
/// }
/// ```
pub fn lfs_dir_alloc(_lfs: *const core::ffi::c_void, _dir: *mut LfsMdir) -> i32 {
    todo!("lfs_dir_alloc")
}

/// Per lfs.c lfs_dir_drop (lines 1859-1878)
///
/// C:
/// ```c
/// static int lfs_dir_drop(lfs_t *lfs, lfs_mdir_t *dir, lfs_mdir_t *tail) {
///     // steal state
///     int err = lfs_dir_getgstate(lfs, tail, &lfs->gdelta);
///     if (err) {
///         return err;
///     }
///
///     // steal tail
///     lfs_pair_tole32(tail->tail);
///     err = lfs_dir_commit(lfs, dir, LFS_MKATTRS(
///             {LFS_MKTAG(LFS_TYPE_TAIL + tail->split, 0x3ff, 8), tail->tail}));
///     lfs_pair_fromle32(tail->tail);
///     if (err) {
///         return err;
///     }
///
///     return 0;
/// }
/// ```
pub fn lfs_dir_drop(
    _lfs: *const core::ffi::c_void,
    _dir: *mut LfsMdir,
    _tail: *const LfsMdir,
) -> i32 {
    todo!("lfs_dir_drop")
}

/// Per lfs.c lfs_dir_split (lines 1880-1913)
///
/// C:
/// ```c
/// static int lfs_dir_split(lfs_t *lfs,
///         lfs_mdir_t *dir, const struct lfs_mattr *attrs, int attrcount,
///         lfs_mdir_t *source, uint16_t split, uint16_t end) {
///     // create tail metadata pair
///     lfs_mdir_t tail;
///     int err = lfs_dir_alloc(lfs, &tail);
///     if (err) {
///         return err;
///     }
///
///     tail.split = dir->split;
///     tail.tail[0] = dir->tail[0];
///     tail.tail[1] = dir->tail[1];
///
///     // note we don't care about LFS_OK_RELOCATED
///     int res = lfs_dir_compact(lfs, &tail, attrs, attrcount, source, split, end);
///     if (res < 0) {
///         return res;
///     }
///
///     dir->tail[0] = tail.pair[0];
///     dir->tail[1] = tail.pair[1];
///     dir->split = true;
///
///     // update root if needed
///     if (lfs_pair_cmp(dir->pair, lfs->root) == 0 && split == 0) {
///         lfs->root[0] = tail.pair[0];
///         lfs->root[1] = tail.pair[1];
///     }
///
///     return 0;
/// }
/// ```
pub fn lfs_dir_split(
    _lfs: *const core::ffi::c_void,
    _dir: *mut LfsMdir,
    _attrs: *const core::ffi::c_void,
    _attrcount: i32,
) -> i32 {
    todo!("lfs_dir_split")
}

/// Per lfs.c lfs_dir_commit_size (lines 1915-1923)
///
/// C:
/// ```c
/// static int lfs_dir_commit_size(void *p, lfs_tag_t tag, const void *buffer) {
///     lfs_size_t *size = p;
///     (void)buffer;
///
///     *size += lfs_tag_dsize(tag);
///     return 0;
/// }
/// ```
pub fn lfs_dir_commit_size(
    _p: *mut core::ffi::c_void,
    _tag: lfs_tag_t,
    _buffer: *const core::ffi::c_void,
) -> i32 {
    todo!("lfs_dir_commit_size")
}

/// Per lfs.c lfs_dir_commit_commit (lines 1932-1936)
///
/// C:
/// ```c
/// static int lfs_dir_commit_commit(void *p, lfs_tag_t tag, const void *buffer) {
///     struct lfs_dir_commit_commit *commit = p;
///     return lfs_dir_commitattr(commit->lfs, commit->commit, tag, buffer);
/// }
/// ```
pub fn lfs_dir_commit_commit(
    _p: *mut core::ffi::c_void,
    _tag: lfs_tag_t,
    _buffer: *const core::ffi::c_void,
) -> i32 {
    todo!("lfs_dir_commit_commit")
}

/// Per lfs.c lfs_dir_needsrelocation (lines 1939-1949)
///
/// C:
/// ```c
/// static bool lfs_dir_needsrelocation(lfs_t *lfs, lfs_mdir_t *dir) {
///     // If our revision count == n * block_cycles, we should force a relocation,
///     // this is how littlefs wear-levels at the metadata-pair level. Note that we
///     // actually use (block_cycles+1)|1, this is to avoid two corner cases:
///     // 1. block_cycles = 1, which would prevent relocations from terminating
///     // 2. block_cycles = 2n, which, due to aliasing, would only ever relocate
///     //    one metadata block in the pair, effectively making this useless
///     return (lfs->cfg->block_cycles > 0
///             && ((dir->rev + 1) % ((lfs->cfg->block_cycles+1)|1) == 0));
/// }
/// ```
pub fn lfs_dir_needsrelocation(_lfs: *const core::ffi::c_void, _dir: *const LfsMdir) -> bool {
    todo!("lfs_dir_needsrelocation")
}

/// Per lfs.c lfs_dir_compact (lines 1952-2123)
///
/// C:
/// ```c
/// static int lfs_dir_compact(lfs_t *lfs,
///         lfs_mdir_t *dir, const struct lfs_mattr *attrs, int attrcount,
///         lfs_mdir_t *source, uint16_t begin, uint16_t end) {
///     // save some state in case block is bad
///     bool relocated = false;
///     bool tired = lfs_dir_needsrelocation(lfs, dir);
///
///     // increment revision count
///     dir->rev += 1;
///
///     // do not proactively relocate blocks during migrations, this
///     // can cause a number of failure states such: clobbering the
///     // v1 superblock if we relocate root, and invalidating directory
///     // pointers if we relocate the head of a directory. On top of
///     // this, relocations increase the overall complexity of
///     // lfs_migration, which is already a delicate operation.
/// #ifdef LFS_MIGRATE
///     if (lfs->lfs1) {
///         tired = false;
///     }
/// #endif
///
///     if (tired && lfs_pair_cmp(dir->pair, (const lfs_block_t[2]){0, 1}) != 0) {
///         // we're writing too much, time to relocate
///         goto relocate;
///     }
///
///     // begin loop to commit compaction to blocks until a compact sticks
///     while (true) {
///         {
///             // setup commit state
///             struct lfs_commit commit = {
///                 .block = dir->pair[1],
///                 .off = 0,
///                 .ptag = 0xffffffff,
///                 .crc = 0xffffffff,
///
///                 .begin = 0,
///                 .end = (lfs->cfg->metadata_max ?
///                     lfs->cfg->metadata_max : lfs->cfg->block_size) - 8,
///             };
///
///             // erase block to write to
///             int err = lfs_bd_erase(lfs, dir->pair[1]);
///             if (err) {
///                 if (err == LFS_ERR_CORRUPT) {
///                     goto relocate;
///                 }
///                 return err;
///             }
///
///             // write out header
///             dir->rev = lfs_tole32(dir->rev);
///             err = lfs_dir_commitprog(lfs, &commit,
///                     &dir->rev, sizeof(dir->rev));
///             dir->rev = lfs_fromle32(dir->rev);
///             if (err) {
///                 if (err == LFS_ERR_CORRUPT) {
///                     goto relocate;
///                 }
///                 return err;
///             }
///
///             // traverse the directory, this time writing out all unique tags
///             err = lfs_dir_traverse(lfs,
///                     source, 0, 0xffffffff, attrs, attrcount,
///                     LFS_MKTAG(0x400, 0x3ff, 0),
///                     LFS_MKTAG(LFS_TYPE_NAME, 0, 0),
///                     begin, end, -begin,
///                     lfs_dir_commit_commit, &(struct lfs_dir_commit_commit){
///                         lfs, &commit});
///             if (err) {
///                 if (err == LFS_ERR_CORRUPT) {
///                     goto relocate;
///                 }
///                 return err;
///             }
///
///             // commit tail, which may be new after last size check
///             if (!lfs_pair_isnull(dir->tail)) {
///                 lfs_pair_tole32(dir->tail);
///                 err = lfs_dir_commitattr(lfs, &commit,
///                         LFS_MKTAG(LFS_TYPE_TAIL + dir->split, 0x3ff, 8),
///                         dir->tail);
///                 lfs_pair_fromle32(dir->tail);
///                 if (err) {
///                     if (err == LFS_ERR_CORRUPT) {
///                         goto relocate;
///                     }
///                     return err;
///                 }
///             }
///
///             // bring over gstate?
///             lfs_gstate_t delta = {0};
///             if (!relocated) {
///                 lfs_gstate_xor(&delta, &lfs->gdisk);
///                 lfs_gstate_xor(&delta, &lfs->gstate);
///             }
///             lfs_gstate_xor(&delta, &lfs->gdelta);
///             delta.tag &= ~LFS_MKTAG(0, 0, 0x3ff);
///
///             err = lfs_dir_getgstate(lfs, dir, &delta);
///             if (err) {
///                 return err;
///             }
///
///             if (!lfs_gstate_iszero(&delta)) {
///                 lfs_gstate_tole32(&delta);
///                 err = lfs_dir_commitattr(lfs, &commit,
///                         LFS_MKTAG(LFS_TYPE_MOVESTATE, 0x3ff,
///                             sizeof(delta)), &delta);
///                 if (err) {
///                     if (err == LFS_ERR_CORRUPT) {
///                         goto relocate;
///                     }
///                     return err;
///                 }
///             }
///
///             // complete commit with crc
///             err = lfs_dir_commitcrc(lfs, &commit);
///             if (err) {
///                 if (err == LFS_ERR_CORRUPT) {
///                     goto relocate;
///                 }
///                 return err;
///             }
///
///             // successful compaction, swap dir pair to indicate most recent
///             LFS_ASSERT(commit.off % lfs->cfg->prog_size == 0);
///             lfs_pair_swap(dir->pair);
///             dir->count = end - begin;
///             dir->off = commit.off;
///             dir->etag = commit.ptag;
///             // update gstate
///             lfs->gdelta = (lfs_gstate_t){0};
///             if (!relocated) {
///                 lfs->gdisk = lfs->gstate;
///             }
///         }
///         break;
///
/// relocate:
///         // commit was corrupted, drop caches and prepare to relocate block
///         relocated = true;
///         lfs_cache_drop(lfs, &lfs->pcache);
///         if (!tired) {
///             LFS_DEBUG("Bad block at 0x%"PRIx32, dir->pair[1]);
///         }
///
///         // can't relocate superblock, filesystem is now frozen
///         if (lfs_pair_cmp(dir->pair, (const lfs_block_t[2]){0, 1}) == 0) {
///             LFS_WARN("Superblock 0x%"PRIx32" has become unwritable",
///                     dir->pair[1]);
///             return LFS_ERR_NOSPC;
///         }
///
///         // relocate half of pair
///         int err = lfs_alloc(lfs, &dir->pair[1]);
///         if (err && (err != LFS_ERR_NOSPC || !tired)) {
///             return err;
///         }
///
///         tired = false;
///         continue;
///     }
///
///     return relocated ? LFS_OK_RELOCATED : 0;
/// }
/// ```
pub fn lfs_dir_compact(
    _lfs: *const core::ffi::c_void,
    _dir: *mut LfsMdir,
    _attrs: *const core::ffi::c_void,
    _attrcount: i32,
    _source: *const LfsMdir,
    _begin: u16,
    _end: u16,
) -> i32 {
    todo!("lfs_dir_compact")
}

/// Per lfs.c lfs_dir_splittingcompact (lines 2125-2232)
///
/// C:
/// ```c
/// static int lfs_dir_splittingcompact(lfs_t *lfs, lfs_mdir_t *dir,
///         const struct lfs_mattr *attrs, int attrcount,
///         lfs_mdir_t *source, uint16_t begin, uint16_t end) {
///     while (true) {
///         // find size of first split, we do this by halving the split until
///         // the metadata is guaranteed to fit
///         //
///         // Note that this isn't a true binary search, we never increase the
///         // split size. This may result in poorly distributed metadata but isn't
///         // worth the extra code size or performance hit to fix.
///         lfs_size_t split = begin;
///         while (end - split > 1) {
///             lfs_size_t size = 0;
///             int err = lfs_dir_traverse(lfs,
///                     source, 0, 0xffffffff, attrs, attrcount,
///                     LFS_MKTAG(0x400, 0x3ff, 0),
///                     LFS_MKTAG(LFS_TYPE_NAME, 0, 0),
///                     split, end, -split,
///                     lfs_dir_commit_size, &size);
///             if (err) {
///                 return err;
///             }
///
///             // space is complicated, we need room for:
///             //
///             // - tail:         4+2*4 = 12 bytes
///             // - gstate:       4+3*4 = 16 bytes
///             // - move delete:  4     = 4 bytes
///             // - crc:          4+4   = 8 bytes
///             //                 total = 40 bytes
///             //
///             // And we cap at half a block to avoid degenerate cases with
///             // nearly-full metadata blocks.
///             //
///             lfs_size_t metadata_max = (lfs->cfg->metadata_max)
///                     ? lfs->cfg->metadata_max
///                     : lfs->cfg->block_size;
///             if (end - split < 0xff
///                     && size <= lfs_min(
///                         metadata_max - 40,
///                         lfs_alignup(
///                             metadata_max/2,
///                             lfs->cfg->prog_size))) {
///                 break;
///             }
///
///             split = split + ((end - split) / 2);
///         }
///
///         if (split == begin) {
///             // no split needed
///             break;
///         }
///
///         // split into two metadata pairs and continue
///         int err = lfs_dir_split(lfs, dir, attrs, attrcount,
///                 source, split, end);
///         if (err && err != LFS_ERR_NOSPC) {
///             return err;
///         }
///
///         if (err) {
///             // we can't allocate a new block, try to compact with degraded
///             // performance
///             LFS_WARN("Unable to split {0x%"PRIx32", 0x%"PRIx32"}",
///                     dir->pair[0], dir->pair[1]);
///             break;
///         } else {
///             end = split;
///         }
///     }
///
///     if (lfs_dir_needsrelocation(lfs, dir)
///             && lfs_pair_cmp(dir->pair, (const lfs_block_t[2]){0, 1}) == 0) {
///         // oh no! we're writing too much to the superblock,
///         // should we expand?
///         lfs_ssize_t size = lfs_fs_size_(lfs);
///         if (size < 0) {
///             return size;
///         }
///
///         // littlefs cannot reclaim expanded superblocks, so expand cautiously
///         //
///         // if our filesystem is more than ~88% full, don't expand, this is
///         // somewhat arbitrary
///         if (lfs->block_count - size > lfs->block_count/8) {
///             LFS_DEBUG("Expanding superblock at rev %"PRIu32, dir->rev);
///             int err = lfs_dir_split(lfs, dir, attrs, attrcount,
///                     source, begin, end);
///             if (err && err != LFS_ERR_NOSPC) {
///                 return err;
///             }
///
///             if (err) {
///                 // welp, we tried, if we ran out of space there's not much
///                 // we can do, we'll error later if we've become frozen
///                 LFS_WARN("Unable to expand superblock");
///             } else {
///                 // duplicate the superblock entry into the new superblock
///                 end = 1;
///             }
///         }
///     }
///
///     return lfs_dir_compact(lfs, dir, attrs, attrcount, source, begin, end);
/// }
/// ```
pub fn lfs_dir_splittingcompact(
    _lfs: *const core::ffi::c_void,
    _dir: *mut LfsMdir,
    _attrs: *const core::ffi::c_void,
    _attrcount: i32,
    _source: *const LfsMdir,
    _begin: u16,
    _end: u16,
) -> i32 {
    todo!("lfs_dir_splittingcompact")
}

/// Per lfs.c lfs_dir_relocatingcommit (lines 2234-2406)
///
/// C:
/// ```c
/// static int lfs_dir_relocatingcommit(lfs_t *lfs, lfs_mdir_t *dir,
///         const lfs_block_t pair[2],
///         const struct lfs_mattr *attrs, int attrcount,
///         lfs_mdir_t *pdir) {
///     int state = 0;
///
///     // calculate changes to the directory
///     bool hasdelete = false;
///     for (int i = 0; i < attrcount; i++) {
///         if (lfs_tag_type3(attrs[i].tag) == LFS_TYPE_CREATE) {
///             dir->count += 1;
///         } else if (lfs_tag_type3(attrs[i].tag) == LFS_TYPE_DELETE) {
///             LFS_ASSERT(dir->count > 0);
///             dir->count -= 1;
///             hasdelete = true;
///         } else if (lfs_tag_type1(attrs[i].tag) == LFS_TYPE_TAIL) {
///             dir->tail[0] = ((lfs_block_t*)attrs[i].buffer)[0];
///             dir->tail[1] = ((lfs_block_t*)attrs[i].buffer)[1];
///             dir->split = (lfs_tag_chunk(attrs[i].tag) & 1);
///             lfs_pair_fromle32(dir->tail);
///         }
///     }
///
///     // should we actually drop the directory block?
///     if (hasdelete && dir->count == 0) {
///         LFS_ASSERT(pdir);
///         int err = lfs_fs_pred(lfs, dir->pair, pdir);
///         if (err && err != LFS_ERR_NOENT) {
///             return err;
///         }
///
///         if (err != LFS_ERR_NOENT && pdir->split) {
///             state = LFS_OK_DROPPED;
///             goto fixmlist;
///         }
///     }
///
///     if (dir->erased && dir->count < 0xff) {
///         // try to commit
///         struct lfs_commit commit = {
///             .block = dir->pair[0],
///             .off = dir->off,
///             .ptag = dir->etag,
///             .crc = 0xffffffff,
///
///             .begin = dir->off,
///             .end = (lfs->cfg->metadata_max ?
///                 lfs->cfg->metadata_max : lfs->cfg->block_size) - 8,
///         };
///
///         // traverse attrs that need to be written out
///         lfs_pair_tole32(dir->tail);
///         int err = lfs_dir_traverse(lfs,
///                 dir, dir->off, dir->etag, attrs, attrcount,
///                 0, 0, 0, 0, 0,
///                 lfs_dir_commit_commit, &(struct lfs_dir_commit_commit){
///                     lfs, &commit});
///         lfs_pair_fromle32(dir->tail);
///         if (err) {
///             if (err == LFS_ERR_NOSPC || err == LFS_ERR_CORRUPT) {
///                 goto compact;
///             }
///             return err;
///         }
///
///         // commit any global diffs if we have any
///         lfs_gstate_t delta = {0};
///         lfs_gstate_xor(&delta, &lfs->gstate);
///         lfs_gstate_xor(&delta, &lfs->gdisk);
///         lfs_gstate_xor(&delta, &lfs->gdelta);
///         delta.tag &= ~LFS_MKTAG(0, 0, 0x3ff);
///         if (!lfs_gstate_iszero(&delta)) {
///             err = lfs_dir_getgstate(lfs, dir, &delta);
///             if (err) {
///                 return err;
///             }
///
///             lfs_gstate_tole32(&delta);
///             err = lfs_dir_commitattr(lfs, &commit,
///                     LFS_MKTAG(LFS_TYPE_MOVESTATE, 0x3ff,
///                         sizeof(delta)), &delta);
///             if (err) {
///                 if (err == LFS_ERR_NOSPC || err == LFS_ERR_CORRUPT) {
///                     goto compact;
///                 }
///                 return err;
///             }
///         }
///
///         // finalize commit with the crc
///         err = lfs_dir_commitcrc(lfs, &commit);
///         if (err) {
///             if (err == LFS_ERR_NOSPC || err == LFS_ERR_CORRUPT) {
///                 goto compact;
///             }
///             return err;
///         }
///
///         // successful commit, update dir
///         LFS_ASSERT(commit.off % lfs->cfg->prog_size == 0);
///         dir->off = commit.off;
///         dir->etag = commit.ptag;
///         // and update gstate
///         lfs->gdisk = lfs->gstate;
///         lfs->gdelta = (lfs_gstate_t){0};
///
///         goto fixmlist;
///     }
///
/// compact:
///     // fall back to compaction
///     lfs_cache_drop(lfs, &lfs->pcache);
///
///     state = lfs_dir_splittingcompact(lfs, dir, attrs, attrcount,
///             dir, 0, dir->count);
///     if (state < 0) {
///         return state;
///     }
///
///     goto fixmlist;
///
/// fixmlist:;
///     // this complicated bit of logic is for fixing up any active
///     // metadata-pairs that we may have affected
///     //
///     // note we have to make two passes since the mdir passed to
///     // lfs_dir_commit could also be in this list, and even then
///     // we need to copy the pair so they don't get clobbered if we refetch
///     // our mdir.
///     lfs_block_t oldpair[2] = {pair[0], pair[1]};
///     for (struct lfs_mlist *d = lfs->mlist; d; d = d->next) {
///         if (lfs_pair_cmp(d->m.pair, oldpair) == 0) {
///             d->m = *dir;
///             if (d->m.pair != pair) {
///                 for (int i = 0; i < attrcount; i++) {
///                     if (lfs_tag_type3(attrs[i].tag) == LFS_TYPE_DELETE &&
///                             d->id == lfs_tag_id(attrs[i].tag) &&
///                             d->type != LFS_TYPE_DIR) {
///                         d->m.pair[0] = LFS_BLOCK_NULL;
///                         d->m.pair[1] = LFS_BLOCK_NULL;
///                     } else if (lfs_tag_type3(attrs[i].tag) == LFS_TYPE_DELETE &&
///                             d->id > lfs_tag_id(attrs[i].tag)) {
///                         d->id -= 1;
///                         if (d->type == LFS_TYPE_DIR) {
///                             ((lfs_dir_t*)d)->pos -= 1;
///                         }
///                     } else if (lfs_tag_type3(attrs[i].tag) == LFS_TYPE_CREATE &&
///                             d->id >= lfs_tag_id(attrs[i].tag)) {
///                         d->id += 1;
///                         if (d->type == LFS_TYPE_DIR) {
///                             ((lfs_dir_t*)d)->pos += 1;
///                         }
///                     }
///                 }
///             }
///
///             while (d->id >= d->m.count && d->m.split) {
///                 // we split and id is on tail now
///                 if (lfs_pair_cmp(d->m.tail, lfs->root) != 0) {
///                     d->id -= d->m.count;
///                 }
///                 int err = lfs_dir_fetch(lfs, &d->m, d->m.tail);
///                 if (err) {
///                     return err;
///                 }
///             }
///         }
///     }
///
///     return state;
/// }
/// ```
pub fn lfs_dir_relocatingcommit(
    _lfs: *const core::ffi::c_void,
    _dir: *mut LfsMdir,
    _pair: *const [lfs_block_t; 2],
    _attrs: *const core::ffi::c_void,
    _attrcount: i32,
    _pdir: *const LfsMdir,
) -> i32 {
    todo!("lfs_dir_relocatingcommit")
}

/// Per lfs.c lfs_dir_orphaningcommit (lines 2408-2599)
///
/// C:
/// ```c
/// static int lfs_dir_orphaningcommit(lfs_t *lfs, lfs_mdir_t *dir,
///         const struct lfs_mattr *attrs, int attrcount) {
///     // check for any inline files that aren't RAM backed and
///     // forcefully evict them, needed for filesystem consistency
///     for (lfs_file_t *f = (lfs_file_t*)lfs->mlist; f; f = f->next) {
///         if (dir != &f->m && lfs_pair_cmp(f->m.pair, dir->pair) == 0 &&
///                 f->type == LFS_TYPE_REG && (f->flags & LFS_F_INLINE) &&
///                 f->ctz.size > lfs->cfg->cache_size) {
///             int err = lfs_file_outline(lfs, f);
///             if (err) {
///                 return err;
///             }
///
///             err = lfs_file_flush(lfs, f);
///             if (err) {
///                 return err;
///             }
///         }
///     }
///
///     lfs_block_t lpair[2] = {dir->pair[0], dir->pair[1]};
///     lfs_mdir_t ldir = *dir;
///     lfs_mdir_t pdir;
///     int state = lfs_dir_relocatingcommit(lfs, &ldir, dir->pair,
///             attrs, attrcount, &pdir);
///     if (state < 0) {
///         return state;
///     }
///
///     // update if we're not in mlist, note we may have already been
///     // updated if we are in mlist
///     if (lfs_pair_cmp(dir->pair, lpair) == 0) {
///         *dir = ldir;
///     }
///
///     // commit was successful, but may require other changes in the
///     // filesystem, these would normally be tail recursive, but we have
///     // flattened them here avoid unbounded stack usage
///
///     // need to drop?
///     if (state == LFS_OK_DROPPED) {
///         // steal state
///         int err = lfs_dir_getgstate(lfs, dir, &lfs->gdelta);
///         if (err) {
///             return err;
///         }
///
///         // steal tail, note that this can't create a recursive drop
///         lpair[0] = pdir.pair[0];
///         lpair[1] = pdir.pair[1];
///         lfs_pair_tole32(dir->tail);
///         state = lfs_dir_relocatingcommit(lfs, &pdir, lpair, LFS_MKATTRS(
///                     {LFS_MKTAG(LFS_TYPE_TAIL + dir->split, 0x3ff, 8),
///                         dir->tail}),
///                 NULL);
///         lfs_pair_fromle32(dir->tail);
///         if (state < 0) {
///             return state;
///         }
///
///         ldir = pdir;
///     }
///
///     // need to relocate?
///     bool orphans = false;
///     while (state == LFS_OK_RELOCATED) {
///         LFS_DEBUG("Relocating {0x%"PRIx32", 0x%"PRIx32"} "
///                     "-> {0x%"PRIx32", 0x%"PRIx32"}",
///                 lpair[0], lpair[1], ldir.pair[0], ldir.pair[1]);
///         state = 0;
///
///         // update internal root
///         if (lfs_pair_cmp(lpair, lfs->root) == 0) {
///             lfs->root[0] = ldir.pair[0];
///             lfs->root[1] = ldir.pair[1];
///         }
///
///         // update internally tracked dirs
///         for (struct lfs_mlist *d = lfs->mlist; d; d = d->next) {
///             if (lfs_pair_cmp(lpair, d->m.pair) == 0) {
///                 d->m.pair[0] = ldir.pair[0];
///                 d->m.pair[1] = ldir.pair[1];
///             }
///
///             if (d->type == LFS_TYPE_DIR &&
///                     lfs_pair_cmp(lpair, ((lfs_dir_t*)d)->head) == 0) {
///                 ((lfs_dir_t*)d)->head[0] = ldir.pair[0];
///                 ((lfs_dir_t*)d)->head[1] = ldir.pair[1];
///             }
///         }
///
///         // find parent
///         lfs_stag_t tag = lfs_fs_parent(lfs, lpair, &pdir);
///         if (tag < 0 && tag != LFS_ERR_NOENT) {
///             return tag;
///         }
///
///         bool hasparent = (tag != LFS_ERR_NOENT);
///         if (tag != LFS_ERR_NOENT) {
///             // note that if we have a parent, we must have a pred, so this will
///             // always create an orphan
///             int err = lfs_fs_preporphans(lfs, +1);
///             if (err) {
///                 return err;
///             }
///
///             // fix pending move in this pair? this looks like an optimization but
///             // is in fact _required_ since relocating may outdate the move.
///             uint16_t moveid = 0x3ff;
///             if (lfs_gstate_hasmovehere(&lfs->gstate, pdir.pair)) {
///                 moveid = lfs_tag_id(lfs->gstate.tag);
///                 LFS_DEBUG("Fixing move while relocating "
///                         "{0x%"PRIx32", 0x%"PRIx32"} 0x%"PRIx16"\n",
///                         pdir.pair[0], pdir.pair[1], moveid);
///                 lfs_fs_prepmove(lfs, 0x3ff, NULL);
///                 if (moveid < lfs_tag_id(tag)) {
///                     tag -= LFS_MKTAG(0, 1, 0);
///                 }
///             }
///
///             lfs_block_t ppair[2] = {pdir.pair[0], pdir.pair[1]};
///             lfs_pair_tole32(ldir.pair);
///             state = lfs_dir_relocatingcommit(lfs, &pdir, ppair, LFS_MKATTRS(
///                         {LFS_MKTAG_IF(moveid != 0x3ff,
///                             LFS_TYPE_DELETE, moveid, 0), NULL},
///                         {tag, ldir.pair}),
///                     NULL);
///             lfs_pair_fromle32(ldir.pair);
///             if (state < 0) {
///                 return state;
///             }
///
///             if (state == LFS_OK_RELOCATED) {
///                 lpair[0] = ppair[0];
///                 lpair[1] = ppair[1];
///                 ldir = pdir;
///                 orphans = true;
///                 continue;
///             }
///         }
///
///         // find pred
///         int err = lfs_fs_pred(lfs, lpair, &pdir);
///         if (err && err != LFS_ERR_NOENT) {
///             return err;
///         }
///         LFS_ASSERT(!(hasparent && err == LFS_ERR_NOENT));
///
///         // if we can't find dir, it must be new
///         if (err != LFS_ERR_NOENT) {
///             if (lfs_gstate_hasorphans(&lfs->gstate)) {
///                 // next step, clean up orphans
///                 err = lfs_fs_preporphans(lfs, -(int8_t)hasparent);
///                 if (err) {
///                     return err;
///                 }
///             }
///
///             // fix pending move in this pair? this looks like an optimization
///             // but is in fact _required_ since relocating may outdate the move.
///             uint16_t moveid = 0x3ff;
///             if (lfs_gstate_hasmovehere(&lfs->gstate, pdir.pair)) {
///                 moveid = lfs_tag_id(lfs->gstate.tag);
///                 LFS_DEBUG("Fixing move while relocating "
///                         "{0x%"PRIx32", 0x%"PRIx32"} 0x%"PRIx16"\n",
///                         pdir.pair[0], pdir.pair[1], moveid);
///                 lfs_fs_prepmove(lfs, 0x3ff, NULL);
///             }
///
///             // replace bad pair, either we clean up desync, or no desync occured
///             lpair[0] = pdir.pair[0];
///             lpair[1] = pdir.pair[1];
///             lfs_pair_tole32(ldir.pair);
///             state = lfs_dir_relocatingcommit(lfs, &pdir, lpair, LFS_MKATTRS(
///                         {LFS_MKTAG_IF(moveid != 0x3ff,
///                             LFS_TYPE_DELETE, moveid, 0), NULL},
///                         {LFS_MKTAG(LFS_TYPE_TAIL + pdir.split, 0x3ff, 8),
///                             ldir.pair}),
///                     NULL);
///             lfs_pair_fromle32(ldir.pair);
///             if (state < 0) {
///                 return state;
///             }
///
///             ldir = pdir;
///         }
///     }
///
///     return orphans ? LFS_OK_ORPHANED : 0;
/// }
/// ```
pub fn lfs_dir_orphaningcommit(
    _lfs: *const core::ffi::c_void,
    _dir: *mut LfsMdir,
    _attrs: *const core::ffi::c_void,
    _attrcount: i32,
) -> i32 {
    todo!("lfs_dir_orphaningcommit")
}

/// Per lfs.c lfs_dir_commit (lines 2601-2623)
///
/// C:
/// ```c
/// static int lfs_dir_commit(lfs_t *lfs, lfs_mdir_t *dir,
///         const struct lfs_mattr *attrs, int attrcount) {
///     int orphans = lfs_dir_orphaningcommit(lfs, dir, attrs, attrcount);
///     if (orphans < 0) {
///         return orphans;
///     }
///
///     if (orphans) {
///         // make sure we've removed all orphans, this is a noop if there
///         // are none, but if we had nested blocks failures we may have
///         // created some
///         int err = lfs_fs_deorphan(lfs, false);
///         if (err) {
///             return err;
///         }
///     }
///
///     return 0;
/// }
/// ```
pub fn lfs_dir_commit(
    _lfs: *const core::ffi::c_void,
    _dir: *mut LfsMdir,
    _attrs: *const core::ffi::c_void,
    _attrcount: i32,
) -> i32 {
    todo!("lfs_dir_commit")
}
