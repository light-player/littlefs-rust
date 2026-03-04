//! Directory fetch. Per lfs.c lfs_dir_fetch, lfs_dir_getgstate, lfs_dir_getinfo.

use crate::dir::LfsMdir;
use crate::lfs_gstate::LfsGstate;
use crate::lfs_info::LfsInfo;
use crate::types::{lfs_block_t, lfs_stag_t, lfs_tag_t};

/// Per lfs.c lfs_dir_fetchmatch (lines 1107-1386)
///
/// C:
/// ```c
/// static lfs_stag_t lfs_dir_fetchmatch(lfs_t *lfs,
///         lfs_mdir_t *dir, const lfs_block_t pair[2],
///         lfs_tag_t fmask, lfs_tag_t ftag, uint16_t *id,
///         int (*cb)(void *data, lfs_tag_t tag, const void *buffer), void *data) {
///     // we can find tag very efficiently during a fetch, since we're already
///     // scanning the entire directory
///     lfs_stag_t besttag = -1;
///
///     // if either block address is invalid we return LFS_ERR_CORRUPT here,
///     // otherwise later writes to the pair could fail
///     if (lfs->block_count
///             && (pair[0] >= lfs->block_count || pair[1] >= lfs->block_count)) {
///         return LFS_ERR_CORRUPT;
///     }
///
///     // find the block with the most recent revision
///     uint32_t revs[2] = {0, 0};
///     int r = 0;
///     for (int i = 0; i < 2; i++) {
///         int err = lfs_bd_read(lfs,
///                 NULL, &lfs->rcache, sizeof(revs[i]),
///                 pair[i], 0, &revs[i], sizeof(revs[i]));
///         revs[i] = lfs_fromle32(revs[i]);
///         if (err && err != LFS_ERR_CORRUPT) {
///             return err;
///         }
///
///         if (err != LFS_ERR_CORRUPT &&
///                 lfs_scmp(revs[i], revs[(i+1)%2]) > 0) {
///             r = i;
///         }
///     }
///
///     dir->pair[0] = pair[(r+0)%2];
///     dir->pair[1] = pair[(r+1)%2];
///     dir->rev = revs[(r+0)%2];
///     dir->off = 0; // nonzero = found some commits
///
///     // now scan tags to fetch the actual dir and find possible match
///     for (int i = 0; i < 2; i++) {
///         lfs_off_t off = 0;
///         lfs_tag_t ptag = 0xffffffff;
///
///         uint16_t tempcount = 0;
///         lfs_block_t temptail[2] = {LFS_BLOCK_NULL, LFS_BLOCK_NULL};
///         bool tempsplit = false;
///         lfs_stag_t tempbesttag = besttag;
///
///         // assume not erased until proven otherwise
///         bool maybeerased = false;
///         bool hasfcrc = false;
///         struct lfs_fcrc fcrc;
///
///         dir->rev = lfs_tole32(dir->rev);
///         uint32_t crc = lfs_crc(0xffffffff, &dir->rev, sizeof(dir->rev));
///         dir->rev = lfs_fromle32(dir->rev);
///
///         while (true) {
///             // extract next tag
///             lfs_tag_t tag;
///             off += lfs_tag_dsize(ptag);
///             int err = lfs_bd_read(lfs,
///                     NULL, &lfs->rcache, lfs->cfg->block_size,
///                     dir->pair[0], off, &tag, sizeof(tag));
///             if (err) {
///                 if (err == LFS_ERR_CORRUPT) {
///                     // can't continue?
///                     break;
///                 }
///                 return err;
///             }
///
///             crc = lfs_crc(crc, &tag, sizeof(tag));
///             tag = lfs_frombe32(tag) ^ ptag;
///
///             // next commit not yet programmed?
///             if (!lfs_tag_isvalid(tag)) {
///                 // we only might be erased if the last tag was a crc
///                 maybeerased = (lfs_tag_type2(ptag) == LFS_TYPE_CCRC);
///                 break;
///             // out of range?
///             } else if (off + lfs_tag_dsize(tag) > lfs->cfg->block_size) {
///                 break;
///             }
///
///             ptag = tag;
///
///             if (lfs_tag_type2(tag) == LFS_TYPE_CCRC) {
///                 // check the crc attr
///                 uint32_t dcrc;
///                 err = lfs_bd_read(lfs,
///                         NULL, &lfs->rcache, lfs->cfg->block_size,
///                         dir->pair[0], off+sizeof(tag), &dcrc, sizeof(dcrc));
///                 if (err) {
///                     if (err == LFS_ERR_CORRUPT) {
///                         break;
///                     }
///                     return err;
///                 }
///                 dcrc = lfs_fromle32(dcrc);
///
///                 if (crc != dcrc) {
///                     break;
///                 }
///
///                 // reset the next bit if we need to
///                 ptag ^= (lfs_tag_t)(lfs_tag_chunk(tag) & 1U) << 31;
///
///                 // toss our crc into the filesystem seed for
///                 // pseudorandom numbers, note we use another crc here
///                 // as a collection function because it is sufficiently
///                 // random and convenient
///                 lfs->seed = lfs_crc(lfs->seed, &crc, sizeof(crc));
///
///                 // update with what's found so far
///                 besttag = tempbesttag;
///                 dir->off = off + lfs_tag_dsize(tag);
///                 dir->etag = ptag;
///                 dir->count = tempcount;
///                 dir->tail[0] = temptail[0];
///                 dir->tail[1] = temptail[1];
///                 dir->split = tempsplit;
///
///                 // reset crc, hasfcrc
///                 crc = 0xffffffff;
///                 continue;
///             }
///
///             // crc the entry first, hopefully leaving it in the cache
///             err = lfs_bd_crc(lfs,
///                     NULL, &lfs->rcache, lfs->cfg->block_size,
///                     dir->pair[0], off+sizeof(tag),
///                     lfs_tag_dsize(tag)-sizeof(tag), &crc);
///             if (err) {
///                 if (err == LFS_ERR_CORRUPT) {
///                     break;
///                 }
///                 return err;
///             }
///
///             // directory modification tags?
///             if (lfs_tag_type1(tag) == LFS_TYPE_NAME) {
///                 // increase count of files if necessary
///                 if (lfs_tag_id(tag) >= tempcount) {
///                     tempcount = lfs_tag_id(tag) + 1;
///                 }
///             } else if (lfs_tag_type1(tag) == LFS_TYPE_SPLICE) {
///                 tempcount += lfs_tag_splice(tag);
///
///                 if (tag == (LFS_MKTAG(LFS_TYPE_DELETE, 0, 0) |
///                         (LFS_MKTAG(0, 0x3ff, 0) & tempbesttag))) {
///                     tempbesttag |= 0x80000000;
///                 } else if (tempbesttag != -1 &&
///                         lfs_tag_id(tag) <= lfs_tag_id(tempbesttag)) {
///                     tempbesttag += LFS_MKTAG(0, lfs_tag_splice(tag), 0);
///                 }
///             } else if (lfs_tag_type1(tag) == LFS_TYPE_TAIL) {
///                 tempsplit = (lfs_tag_chunk(tag) & 1);
///
///                 err = lfs_bd_read(lfs,
///                         NULL, &lfs->rcache, lfs->cfg->block_size,
///                         dir->pair[0], off+sizeof(tag), &temptail, 8);
///                 if (err) {
///                     if (err == LFS_ERR_CORRUPT) {
///                         break;
///                     }
///                     return err;
///                 }
///                 lfs_pair_fromle32(temptail);
///             } else if (lfs_tag_type3(tag) == LFS_TYPE_FCRC) {
///                 err = lfs_bd_read(lfs,
///                         NULL, &lfs->rcache, lfs->cfg->block_size,
///                         dir->pair[0], off+sizeof(tag),
///                         &fcrc, sizeof(fcrc));
///                 if (err) {
///                     if (err == LFS_ERR_CORRUPT) {
///                         break;
///                     }
///                     return err;
///                 }
///
///                 lfs_fcrc_fromle32(&fcrc);
///                 hasfcrc = true;
///             }
///
///             // found a match for our fetcher?
///             if ((fmask & tag) == (fmask & ftag)) {
///                 int res = cb(data, tag, &(struct lfs_diskoff){
///                         dir->pair[0], off+sizeof(tag)});
///                 if (res < 0) {
///                     if (res == LFS_ERR_CORRUPT) {
///                         break;
///                     }
///                     return res;
///                 }
///
///                 if (res == LFS_CMP_EQ) {
///                     // found a match
///                     tempbesttag = tag;
///                 } else if ((LFS_MKTAG(0x7ff, 0x3ff, 0) & tag) ==
///                         (LFS_MKTAG(0x7ff, 0x3ff, 0) & tempbesttag)) {
///                     // found an identical tag, but contents didn't match
///                     // this must mean that our besttag has been overwritten
///                     tempbesttag = -1;
///                 } else if (res == LFS_CMP_GT &&
///                         lfs_tag_id(tag) <= lfs_tag_id(tempbesttag)) {
///                     // found a greater match, keep track to keep things sorted
///                     tempbesttag = tag | 0x80000000;
///                 }
///             }
///         }
///
///         // found no valid commits?
///         if (dir->off == 0) {
///             // try the other block?
///             lfs_pair_swap(dir->pair);
///             dir->rev = revs[(r+1)%2];
///             continue;
///         }
///
///         // did we end on a valid commit? we may have an erased block
///         dir->erased = false;
///         if (maybeerased && dir->off % lfs->cfg->prog_size == 0) {
///         #ifdef LFS_MULTIVERSION
///             // note versions < lfs2.1 did not have fcrc tags, if
///             // we're < lfs2.1 treat missing fcrc as erased data
///             //
///             // we don't strictly need to do this, but otherwise writing
///             // to lfs2.0 disks becomes very inefficient
///             if (lfs_fs_disk_version(lfs) < 0x00020001) {
///                 dir->erased = true;
///
///             } else
///         #endif
///             if (hasfcrc) {
///                 // check for an fcrc matching the next prog's erased state, if
///                 // this failed most likely a previous prog was interrupted, we
///                 // need a new erase
///                 uint32_t fcrc_ = 0xffffffff;
///                 int err = lfs_bd_crc(lfs,
///                         NULL, &lfs->rcache, lfs->cfg->block_size,
///                         dir->pair[0], dir->off, fcrc.size, &fcrc_);
///                 if (err && err != LFS_ERR_CORRUPT) {
///                     return err;
///                 }
///
///                 // found beginning of erased part?
///                 dir->erased = (fcrc_ == fcrc.crc);
///             }
///         }
///
///         // synthetic move
///         if (lfs_gstate_hasmovehere(&lfs->gdisk, dir->pair)) {
///             if (lfs_tag_id(lfs->gdisk.tag) == lfs_tag_id(besttag)) {
///                 besttag |= 0x80000000;
///             } else if (besttag != -1 &&
///                     lfs_tag_id(lfs->gdisk.tag) < lfs_tag_id(besttag)) {
///                 besttag -= LFS_MKTAG(0, 1, 0);
///             }
///         }
///
///         // found tag? or found best id?
///         if (id) {
///             *id = lfs_min(lfs_tag_id(besttag), dir->count);
///         }
///
///         if (lfs_tag_isvalid(besttag)) {
///             return besttag;
///         } else if (lfs_tag_id(besttag) < dir->count) {
///             return LFS_ERR_NOENT;
///         } else {
///             return 0;
///         }
///     }
///
///     LFS_ERROR("Corrupted dir pair at {0x%"PRIx32", 0x%"PRIx32"}",
///             dir->pair[0], dir->pair[1]);
///     return LFS_ERR_CORRUPT;
/// }
///
/// ```
pub fn lfs_dir_fetchmatch(
    _lfs: *const core::ffi::c_void,
    _dir: *mut LfsMdir,
    _pair: *const [lfs_block_t; 2],
    _fmask: lfs_tag_t,
    _ftag: lfs_tag_t,
    _id: *mut u16,
    _cb: Option<
        unsafe extern "C" fn(*mut core::ffi::c_void, lfs_tag_t, *const core::ffi::c_void) -> i32,
    >,
    _data: *mut core::ffi::c_void,
) -> lfs_stag_t {
    todo!("lfs_dir_fetchmatch")
}

/// Per lfs.c lfs_dir_fetch (lines 1387-1393)
///
/// C:
/// ```c
/// static int lfs_dir_fetch(lfs_t *lfs,
///         lfs_mdir_t *dir, const lfs_block_t pair[2]) {
///     // note, mask=-1, tag=-1 can never match a tag since this
///     // pattern has the invalid bit set
///     return (int)lfs_dir_fetchmatch(lfs, dir, pair,
///             (lfs_tag_t)-1, (lfs_tag_t)-1, NULL, NULL, NULL);
/// }
/// ```
pub fn lfs_dir_fetch(
    _lfs: *const core::ffi::c_void,
    _dir: *mut LfsMdir,
    _pair: &[lfs_block_t; 2],
) -> i32 {
    todo!("lfs_dir_fetch")
}

/// Per lfs.c lfs_dir_getgstate (lines 1395-1411)
///
/// C:
/// ```c
/// static int lfs_dir_getgstate(lfs_t *lfs, const lfs_mdir_t *dir,
///         lfs_gstate_t *gstate) {
///     lfs_gstate_t temp;
///     lfs_stag_t res = lfs_dir_get(lfs, dir, LFS_MKTAG(0x7ff, 0, 0),
///             LFS_MKTAG(LFS_TYPE_MOVESTATE, 0, sizeof(temp)), &temp);
///     if (res < 0 && res != LFS_ERR_NOENT) {
///         return res;
///     }
///
///     if (res != LFS_ERR_NOENT) {
///         // xor together to find resulting gstate
///         lfs_gstate_fromle32(&temp);
///         lfs_gstate_xor(gstate, &temp);
///     }
///
///     return 0;
/// }
/// ```
pub fn lfs_dir_getgstate(
    _lfs: *const core::ffi::c_void,
    _dir: *const LfsMdir,
    _gstate: *mut LfsGstate,
) -> i32 {
    todo!("lfs_dir_getgstate")
}

/// Per lfs.c lfs_dir_getinfo (lines 1413-1445)
///
/// C:
/// ```c
/// static int lfs_dir_getinfo(lfs_t *lfs, lfs_mdir_t *dir,
///         uint16_t id, struct lfs_info *info) {
///     if (id == 0x3ff) {
///         // special case for root
///         strcpy(info->name, "/");
///         info->type = LFS_TYPE_DIR;
///         return 0;
///     }
///
///     lfs_stag_t tag = lfs_dir_get(lfs, dir, LFS_MKTAG(0x780, 0x3ff, 0),
///             LFS_MKTAG(LFS_TYPE_NAME, id, lfs->name_max+1), info->name);
///     if (tag < 0) {
///         return (int)tag;
///     }
///
///     info->type = lfs_tag_type3(tag);
///
///     struct lfs_ctz ctz;
///     tag = lfs_dir_get(lfs, dir, LFS_MKTAG(0x700, 0x3ff, 0),
///             LFS_MKTAG(LFS_TYPE_STRUCT, id, sizeof(ctz)), &ctz);
///     if (tag < 0) {
///         return (int)tag;
///     }
///     lfs_ctz_fromle32(&ctz);
///
///     if (lfs_tag_type3(tag) == LFS_TYPE_CTZSTRUCT) {
///         info->size = ctz.size;
///     } else if (lfs_tag_type3(tag) == LFS_TYPE_INLINESTRUCT) {
///         info->size = lfs_tag_size(tag);
///     }
///
///     return 0;
/// }
/// ```
pub fn lfs_dir_getinfo(
    _lfs: *const core::ffi::c_void,
    _dir: *const LfsMdir,
    _id: u16,
    _info: *mut LfsInfo,
) -> i32 {
    todo!("lfs_dir_getinfo")
}
