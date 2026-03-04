//! Directory open/read. Per lfs.c lfs_dir_open_, lfs_dir_close_, lfs_dir_read_, etc.

use crate::dir::fetch::{lfs_dir_fetch, lfs_dir_getinfo};
use crate::dir::find::lfs_dir_find;
use crate::dir::lfs_mlist::lfs_mlist_append;
use crate::dir::lfs_mlist::lfs_mlist_remove;
use crate::dir::traverse::lfs_dir_get;
use crate::dir::LfsDir;
use crate::lfs_info::LfsInfo;
use crate::lfs_type::lfs_type::LFS_TYPE_DIR;
use crate::tag::{lfs_mktag, lfs_tag_id, lfs_tag_type3};
use crate::types::lfs_off_t;
use crate::util::lfs_pair_fromle32;

/// Per lfs.c lfs_dir_open_ (lines 2721-2763)
///
/// C:
/// ```c
/// static int lfs_dir_open_(lfs_t *lfs, lfs_dir_t *dir, const char *path) {
///     lfs_stag_t tag = lfs_dir_find(lfs, &dir->m, &path, NULL);
///     if (tag < 0) {
///         return tag;
///     }
///
///     if (lfs_tag_type3(tag) != LFS_TYPE_DIR) {
///         return LFS_ERR_NOTDIR;
///     }
///
///     lfs_block_t pair[2];
///     if (lfs_tag_id(tag) == 0x3ff) {
///         // handle root dir separately
///         pair[0] = lfs->root[0];
///         pair[1] = lfs->root[1];
///     } else {
///         // get dir pair from parent
///         lfs_stag_t res = lfs_dir_get(lfs, &dir->m, LFS_MKTAG(0x700, 0x3ff, 0),
///                 LFS_MKTAG(LFS_TYPE_STRUCT, lfs_tag_id(tag), 8), pair);
///         if (res < 0) {
///             return res;
///         }
///         lfs_pair_fromle32(pair);
///     }
///
///     // fetch first pair
///     int err = lfs_dir_fetch(lfs, &dir->m, pair);
///     if (err) {
///         return err;
///     }
///
///     // setup entry
///     dir->head[0] = dir->m.pair[0];
///     dir->head[1] = dir->m.pair[1];
///     dir->id = 0;
///     dir->pos = 0;
///
///     // add to list of mdirs
///     dir->type = LFS_TYPE_DIR;
///     lfs_mlist_append(lfs, (struct lfs_mlist *)dir);
///
///     return 0;
/// }
/// ```
pub fn lfs_dir_open_(lfs: *mut crate::fs::Lfs, dir: *mut LfsDir, path: *const u8) -> i32 {
    if lfs.is_null() || dir.is_null() || path.is_null() {
        return crate::error::LFS_ERR_INVAL;
    }
    unsafe {
        let dir_ref = &mut *dir;
        let mut path_ptr = path;

        let tag = lfs_dir_find(lfs, &mut dir_ref.m, &mut path_ptr, core::ptr::null_mut());
        if tag < 0 {
            return tag;
        }

        if u32::from(lfs_tag_type3(tag as u32)) != LFS_TYPE_DIR {
            return crate::error::LFS_ERR_NOTDIR;
        }

        let mut pair = [0u32; 2];
        if lfs_tag_id(tag as u32) == 0x3ff {
            pair[0] = (*lfs).root[0];
            pair[1] = (*lfs).root[1];
        } else {
            let res = lfs_dir_get(
                lfs,
                &dir_ref.m as *const _,
                lfs_mktag(0x700, 0x3ff, 0),
                lfs_mktag(
                    crate::lfs_type::lfs_type::LFS_TYPE_STRUCT,
                    lfs_tag_id(tag as u32) as u32,
                    8,
                ),
                pair.as_mut_ptr() as *mut core::ffi::c_void,
            );
            if res < 0 {
                return res;
            }
            lfs_pair_fromle32(&mut pair);
        }

        let err = lfs_dir_fetch(lfs, &mut dir_ref.m, &pair);
        if err != 0 {
            return err;
        }

        dir_ref.head[0] = dir_ref.m.pair[0];
        dir_ref.head[1] = dir_ref.m.pair[1];
        dir_ref.id = 0;
        dir_ref.pos = 0;
        dir_ref.type_ = LFS_TYPE_DIR as u8;
        lfs_mlist_append(lfs, dir as *mut crate::dir::lfs_mlist::LfsMlist);

        0
    }
}

/// Per lfs.c lfs_dir_close_ (lines 2765-2770)
///
/// C:
/// ```c
/// static int lfs_dir_close_(lfs_t *lfs, lfs_dir_t *dir) {
///     // remove from list of mdirs
///     lfs_mlist_remove(lfs, (struct lfs_mlist *)dir);
///
///     return 0;
/// }
/// ```
pub fn lfs_dir_close_(lfs: *mut crate::fs::Lfs, dir: *mut LfsDir) -> i32 {
    if lfs.is_null() || dir.is_null() {
        return crate::error::LFS_ERR_INVAL;
    }
    unsafe {
        lfs_mlist_remove(lfs, dir as *mut crate::dir::lfs_mlist::LfsMlist);
    }
    0
}

/// Per lfs.c lfs_dir_read_ (lines 2772-2815)
///
/// C:
/// ```c
/// static int lfs_dir_read_(lfs_t *lfs, lfs_dir_t *dir, struct lfs_info *info) {
///     memset(info, 0, sizeof(*info));
///
///     // special offset for '.' and '..'
///     if (dir->pos == 0) {
///         info->type = LFS_TYPE_DIR;
///         strcpy(info->name, ".");
///         dir->pos += 1;
///         return true;
///     } else if (dir->pos == 1) {
///         info->type = LFS_TYPE_DIR;
///         strcpy(info->name, "..");
///         dir->pos += 1;
///         return true;
///     }
///
///     while (true) {
///         if (dir->id == dir->m.count) {
///             if (!dir->m.split) {
///                 return false;
///             }
///
///             int err = lfs_dir_fetch(lfs, &dir->m, dir->m.tail);
///             if (err) {
///                 return err;
///             }
///
///             dir->id = 0;
///         }
///
///         int err = lfs_dir_getinfo(lfs, &dir->m, dir->id, info);
///         if (err && err != LFS_ERR_NOENT) {
///             return err;
///         }
///
///         dir->id += 1;
///         if (err != LFS_ERR_NOENT) {
///             break;
///         }
///     }
///
///     dir->pos += 1;
///     return true;
/// }
/// ```
pub fn lfs_dir_read_(lfs: *mut crate::fs::Lfs, dir: *mut LfsDir, info: *mut LfsInfo) -> i32 {
    if lfs.is_null() || dir.is_null() || info.is_null() {
        return crate::error::LFS_ERR_INVAL;
    }
    unsafe {
        let dir_ref = &mut *dir;
        let info_ref = &mut *info;

        info_ref.type_ = 0;
        info_ref.size = 0;
        core::ptr::write_bytes(info_ref.name.as_mut_ptr(), 0, info_ref.name.len());

        if dir_ref.pos == 0 {
            info_ref.type_ = LFS_TYPE_DIR as u8;
            info_ref.name[0] = b'.';
            info_ref.name[1] = 0;
            dir_ref.pos += 1;
            return 1;
        }
        if dir_ref.pos == 1 {
            info_ref.type_ = LFS_TYPE_DIR as u8;
            info_ref.name[0] = b'.';
            info_ref.name[1] = b'.';
            info_ref.name[2] = 0;
            dir_ref.pos += 1;
            return 1;
        }

        loop {
            if dir_ref.id == dir_ref.m.count {
                if !dir_ref.m.split {
                    return 0;
                }
                let err = lfs_dir_fetch(lfs, &mut dir_ref.m, &dir_ref.m.tail);
                if err != 0 {
                    return err;
                }
                dir_ref.id = 0;
            }

            let err = lfs_dir_getinfo(lfs, &dir_ref.m, dir_ref.id, info);
            if err != 0 && err != crate::error::LFS_ERR_NOENT {
                return err;
            }
            dir_ref.id += 1;
            if err != crate::error::LFS_ERR_NOENT {
                break;
            }
        }

        dir_ref.pos += 1;
        1
    }
}

/// Per lfs.c lfs_dir_seek_ (lines 2817-2851)
///
/// C:
/// ```c
/// static int lfs_dir_seek_(lfs_t *lfs, lfs_dir_t *dir, lfs_off_t off) {
///     // simply walk from head dir
///     int err = lfs_dir_rewind_(lfs, dir);
///     if (err) {
///         return err;
///     }
///
///     // first two for ./..
///     dir->pos = lfs_min(2, off);
///     off -= dir->pos;
///
///     // skip superblock entry
///     dir->id = (off > 0 && lfs_pair_cmp(dir->head, lfs->root) == 0);
///
///     while (off > 0) {
///         if (dir->id == dir->m.count) {
///             if (!dir->m.split) {
///                 return LFS_ERR_INVAL;
///             }
///
///             err = lfs_dir_fetch(lfs, &dir->m, dir->m.tail);
///             if (err) {
///                 return err;
///             }
///
///             dir->id = 0;
///         }
///
///         int diff = lfs_min(dir->m.count - dir->id, off);
///         dir->id += diff;
///         dir->pos += diff;
///         off -= diff;
///     }
///
///     return 0;
/// }
/// ```
pub fn lfs_dir_seek_(_lfs: *const core::ffi::c_void, _dir: *mut LfsDir, _off: lfs_off_t) -> i32 {
    todo!("lfs_dir_seek_")
}

/// Per lfs.c lfs_dir_tell_ (lines 2854-2857)
///
/// C:
/// ```c
/// static lfs_soff_t lfs_dir_tell_(lfs_t *lfs, lfs_dir_t *dir) {
///     (void)lfs;
///     return dir->pos;
/// }
/// ```
pub fn lfs_dir_tell_(
    _lfs: *const core::ffi::c_void,
    _dir: *const LfsDir,
) -> crate::types::lfs_soff_t {
    todo!("lfs_dir_tell_")
}

/// Per lfs.c lfs_dir_rewind_ (lines 2859-2869)
///
/// C:
/// ```c
/// static int lfs_dir_rewind_(lfs_t *lfs, lfs_dir_t *dir) {
///     // reload the head dir
///     int err = lfs_dir_fetch(lfs, &dir->m, dir->head);
///     if (err) {
///         return err;
///     }
///
///     dir->id = 0;
///     dir->pos = 0;
///     return 0;
/// }
/// ```
pub fn lfs_dir_rewind_(_lfs: *const core::ffi::c_void, _dir: *mut LfsDir) -> i32 {
    todo!("lfs_dir_rewind_")
}
