//! Directory find. Per lfs.c lfs_dir_find, lfs_dir_find_match.

use crate::bd::bd::lfs_bd_cmp;
use crate::tag::lfs_diskoff;
use crate::tag::lfs_tag_size;
use crate::types::lfs_tag_t;
use crate::util::lfs_min;

const LFS_CMP_EQ: i32 = 0;
const LFS_CMP_LT: i32 = -1;
const LFS_CMP_GT: i32 = 1;

/// Per lfs.c struct lfs_dir_find_match (lines 1447-1475)
#[repr(C)]
pub struct LfsDirFindMatch {
    pub lfs: *mut crate::fs::Lfs,
    pub name: *const u8,
    pub size: crate::types::lfs_size_t,
}

/// Per lfs.c lfs_dir_find_match (and struct lfs_dir_find_match) (lines 1447-1475)
///
/// C:
/// ```c
/// struct lfs_dir_find_match {
///     lfs_t *lfs;
///     const void *name;
///     lfs_size_t size;
/// };
///
/// static int lfs_dir_find_match(void *data,
///         lfs_tag_t tag, const void *buffer) {
///     struct lfs_dir_find_match *name = data;
///     lfs_t *lfs = name->lfs;
///     const struct lfs_diskoff *disk = buffer;
///
///     // compare with disk
///     lfs_size_t diff = lfs_min(name->size, lfs_tag_size(tag));
///     int res = lfs_bd_cmp(lfs,
///             NULL, &lfs->rcache, diff,
///             disk->block, disk->off, name->name, diff);
///     if (res != LFS_CMP_EQ) {
///         return res;
///     }
///
///     // only equal if our size is still the same
///     if (name->size != lfs_tag_size(tag)) {
///         return (name->size < lfs_tag_size(tag)) ? LFS_CMP_LT : LFS_CMP_GT;
///     }
///
///     // found a match!
///     return LFS_CMP_EQ;
/// }
///
/// ```
pub unsafe extern "C" fn lfs_dir_find_match(
    data: *mut core::ffi::c_void,
    tag: lfs_tag_t,
    buffer: *const core::ffi::c_void,
) -> i32 {
    if data.is_null() || buffer.is_null() {
        return LFS_CMP_LT;
    }
    unsafe {
        let name = &*(data as *const LfsDirFindMatch);
        let disk = &*(buffer as *const lfs_diskoff);
        let lfs = &mut *name.lfs;

        let diff = lfs_min(name.size, lfs_tag_size(tag));
        let res = lfs_bd_cmp(
            name.lfs,
            core::ptr::null(),
            &mut lfs.rcache,
            diff,
            disk.block,
            disk.off,
            name.name,
            diff,
        );
        if res != LFS_CMP_EQ {
            return res;
        }
        if name.size != lfs_tag_size(tag) {
            return if name.size < lfs_tag_size(tag) {
                LFS_CMP_LT
            } else {
                LFS_CMP_GT
            };
        }
        LFS_CMP_EQ
    }
}

/// Per lfs.c lfs_dir_find (lines 1483-1590)
///
/// C:
/// ```c
/// static lfs_stag_t lfs_dir_find(lfs_t *lfs, lfs_mdir_t *dir,
///         const char **path, uint16_t *id) {
///     // we reduce path to a single name if we can find it
///     const char *name = *path;
///
///     // default to root dir
///     lfs_stag_t tag = LFS_MKTAG(LFS_TYPE_DIR, 0x3ff, 0);
///     dir->tail[0] = lfs->root[0];
///     dir->tail[1] = lfs->root[1];
///
///     // empty paths are not allowed
///     if (*name == '\0') {
///         return LFS_ERR_INVAL;
///     }
///
///     while (true) {
/// nextname:
///         // skip slashes if we're a directory
///         if (lfs_tag_type3(tag) == LFS_TYPE_DIR) {
///             name += strspn(name, "/");
///         }
///         lfs_size_t namelen = strcspn(name, "/");
///
///         // skip '.'
///         if (namelen == 1 && memcmp(name, ".", 1) == 0) {
///             name += namelen;
///             goto nextname;
///         }
///
///         // error on unmatched '..', trying to go above root?
///         if (namelen == 2 && memcmp(name, "..", 2) == 0) {
///             return LFS_ERR_INVAL;
///         }
///
///         // skip if matched by '..' in name
///         const char *suffix = name + namelen;
///         lfs_size_t sufflen;
///         int depth = 1;
///         while (true) {
///             suffix += strspn(suffix, "/");
///             sufflen = strcspn(suffix, "/");
///             if (sufflen == 0) {
///                 break;
///             }
///
///             if (sufflen == 1 && memcmp(suffix, ".", 1) == 0) {
///                 // noop
///             } else if (sufflen == 2 && memcmp(suffix, "..", 2) == 0) {
///                 depth -= 1;
///                 if (depth == 0) {
///                     name = suffix + sufflen;
///                     goto nextname;
///                 }
///             } else {
///                 depth += 1;
///             }
///
///             suffix += sufflen;
///         }
///
///         // found path
///         if (*name == '\0') {
///             return tag;
///         }
///
///         // update what we've found so far
///         *path = name;
///
///         // only continue if we're a directory
///         if (lfs_tag_type3(tag) != LFS_TYPE_DIR) {
///             return LFS_ERR_NOTDIR;
///         }
///
///         // grab the entry data
///         if (lfs_tag_id(tag) != 0x3ff) {
///             lfs_stag_t res = lfs_dir_get(lfs, dir, LFS_MKTAG(0x700, 0x3ff, 0),
///                     LFS_MKTAG(LFS_TYPE_STRUCT, lfs_tag_id(tag), 8), dir->tail);
///             if (res < 0) {
///                 return res;
///             }
///             lfs_pair_fromle32(dir->tail);
///         }
///
///         // find entry matching name
///         while (true) {
///             tag = lfs_dir_fetchmatch(lfs, dir, dir->tail,
///                     LFS_MKTAG(0x780, 0, 0),
///                     LFS_MKTAG(LFS_TYPE_NAME, 0, namelen),
///                     id,
///                     lfs_dir_find_match, &(struct lfs_dir_find_match){
///                         lfs, name, namelen});
///             if (tag < 0) {
///                 return tag;
///             }
///
///             if (tag) {
///                 break;
///             }
///
///             if (!dir->split) {
///                 return LFS_ERR_NOENT;
///             }
///         }
///
///         // to next name
///         name += namelen;
///     }
/// }
/// ```
pub fn lfs_dir_find(
    _lfs: *const core::ffi::c_void,
    _dir: *mut crate::dir::LfsMdir,
    _path: *mut *const i8,
    _id: *mut u16,
) -> crate::types::lfs_stag_t {
    todo!("lfs_dir_find")
}
