//! Rename. Per lfs.c lfs_rename_.

use crate::dir::commit::{lfs_dir_commit, lfs_dir_drop};
use crate::dir::fetch::lfs_dir_fetch;
use crate::dir::find::lfs_dir_find;
use crate::dir::traverse::lfs_dir_get;
use crate::dir::{LfsMdir, LfsMlist};
use crate::error::{
    LFS_ERR_INVAL, LFS_ERR_ISDIR, LFS_ERR_NAMETOOLONG, LFS_ERR_NOENT, LFS_ERR_NOTDIR,
    LFS_ERR_NOTEMPTY,
};
use crate::fs::parent::lfs_fs_pred;
use crate::fs::superblock::{lfs_fs_forceconsistency, lfs_fs_prepmove, lfs_fs_preporphans};
use crate::lfs_gstate::{lfs_gstate_hasmove, lfs_gstate_hasorphans};
use crate::lfs_type::lfs_type::{
    LFS_FROM_MOVE, LFS_TYPE_CREATE, LFS_TYPE_DELETE, LFS_TYPE_DIR, LFS_TYPE_STRUCT,
};
use crate::tag::{lfs_mattr, lfs_mktag, lfs_mktag_if, lfs_tag_id, lfs_tag_type3};
use crate::types::lfs_block_t;
use crate::util::{
    lfs_pair_cmp, lfs_pair_fromle32, lfs_path_isdir, lfs_path_islast, lfs_path_namelen,
};

/// Translation docs: Renames a file or directory from oldpath to newpath. Handles
/// same-directory renames, cross-directory moves, and overwriting existing entries.
/// When overwriting a directory, it must be empty. On failure returns a negative error code.
///
/// C: lfs.c:3961-4138
///
/// C:
/// ```c
/// static int lfs_rename_(lfs_t *lfs, const char *oldpath, const char *newpath) {
///     // deorphan if we haven't yet, needed at most once after poweron
///     int err = lfs_fs_forceconsistency(lfs);
///     if (err) {
///         return err;
///     }
///
///     // find old entry
///     lfs_mdir_t oldcwd;
///     lfs_stag_t oldtag = lfs_dir_find(lfs, &oldcwd, &oldpath, NULL);
///     if (oldtag < 0 || lfs_tag_id(oldtag) == 0x3ff) {
///         return (oldtag < 0) ? (int)oldtag : LFS_ERR_INVAL;
///     }
///
///     // find new entry
///     lfs_mdir_t newcwd;
///     uint16_t newid;
///     lfs_stag_t prevtag = lfs_dir_find(lfs, &newcwd, &newpath, &newid);
///     if ((prevtag < 0 || lfs_tag_id(prevtag) == 0x3ff) &&
///             !(prevtag == LFS_ERR_NOENT && lfs_path_islast(newpath))) {
///         return (prevtag < 0) ? (int)prevtag : LFS_ERR_INVAL;
///     }
///
///     // if we're in the same pair there's a few special cases...
///     bool samepair = (lfs_pair_cmp(oldcwd.pair, newcwd.pair) == 0);
///     uint16_t newoldid = lfs_tag_id(oldtag);
///
///     struct lfs_mlist prevdir;
///     prevdir.next = lfs->mlist;
///     if (prevtag == LFS_ERR_NOENT) {
///         // if we're a file, don't allow trailing slashes
///         if (lfs_path_isdir(newpath)
///                 && lfs_tag_type3(oldtag) != LFS_TYPE_DIR) {
///             return LFS_ERR_NOTDIR;
///         }
///
///         // check that name fits
///         lfs_size_t nlen = lfs_path_namelen(newpath);
///         if (nlen > lfs->name_max) {
///             return LFS_ERR_NAMETOOLONG;
///         }
///
///         // there is a small chance we are being renamed in the same
///         // directory/ to an id less than our old id, the global update
///         // to handle this is a bit messy
///         if (samepair && newid <= newoldid) {
///             newoldid += 1;
///         }
///     } else if (lfs_tag_type3(prevtag) != lfs_tag_type3(oldtag)) {
///         return (lfs_tag_type3(prevtag) == LFS_TYPE_DIR)
///                 ? LFS_ERR_ISDIR
///                 : LFS_ERR_NOTDIR;
///     } else if (samepair && newid == newoldid) {
///         // we're renaming to ourselves??
///         return 0;
///     } else if (lfs_tag_type3(prevtag) == LFS_TYPE_DIR) {
///         // must be empty before removal
///         lfs_block_t prevpair[2];
///         lfs_stag_t res = lfs_dir_get(lfs, &newcwd, LFS_MKTAG(0x700, 0x3ff, 0),
///                 LFS_MKTAG(LFS_TYPE_STRUCT, newid, 8), prevpair);
///         if (res < 0) {
///             return (int)res;
///         }
///         lfs_pair_fromle32(prevpair);
///
///         // must be empty before removal
///         err = lfs_dir_fetch(lfs, &prevdir.m, prevpair);
///         if (err) {
///             return err;
///         }
///
///         if (prevdir.m.count > 0 || prevdir.m.split) {
///             return LFS_ERR_NOTEMPTY;
///         }
///
///         // mark fs as orphaned
///         err = lfs_fs_preporphans(lfs, +1);
///         if (err) {
///             return err;
///         }
///
///         // I know it's crazy but yes, dir can be changed by our parent's
///         // commit (if predecessor is child)
///         prevdir.type = 0;
///         prevdir.id = 0;
///         lfs->mlist = &prevdir;
///     }
///
///     if (!samepair) {
///         lfs_fs_prepmove(lfs, newoldid, oldcwd.pair);
///     }
///
///     // move over all attributes
///     err = lfs_dir_commit(lfs, &newcwd, LFS_MKATTRS(
///             {LFS_MKTAG_IF(prevtag != LFS_ERR_NOENT,
///                 LFS_TYPE_DELETE, newid, 0), NULL},
///             {LFS_MKTAG(LFS_TYPE_CREATE, newid, 0), NULL},
///             {LFS_MKTAG(lfs_tag_type3(oldtag),
///                 newid, lfs_path_namelen(newpath)), newpath},
///             {LFS_MKTAG(LFS_FROM_MOVE, newid, lfs_tag_id(oldtag)), &oldcwd},
///             {LFS_MKTAG_IF(samepair,
///                 LFS_TYPE_DELETE, newoldid, 0), NULL}));
///     if (err) {
///         lfs->mlist = prevdir.next;
///         return err;
///     }
///
///     // let commit clean up after move (if we're different! otherwise move
///     // logic already fixed it for us)
///     if (!samepair && lfs_gstate_hasmove(&lfs->gstate)) {
///         // prep gstate and delete move id
///         lfs_fs_prepmove(lfs, 0x3ff, NULL);
///         err = lfs_dir_commit(lfs, &oldcwd, LFS_MKATTRS(
///                 {LFS_MKTAG(LFS_TYPE_DELETE, lfs_tag_id(oldtag), 0), NULL}));
///         if (err) {
///             lfs->mlist = prevdir.next;
///             return err;
///         }
///     }
///
///     lfs->mlist = prevdir.next;
///     if (lfs_gstate_hasorphans(&lfs->gstate)) {
///         LFS_ASSERT(prevtag != LFS_ERR_NOENT
///                 && lfs_tag_type3(prevtag) == LFS_TYPE_DIR);
///
///         // fix orphan
///         err = lfs_fs_preporphans(lfs, -1);
///         if (err) {
///             return err;
///         }
///
///         err = lfs_fs_pred(lfs, prevdir.m.pair, &newcwd);
///         if (err) {
///             return err;
///         }
///
///         err = lfs_dir_drop(lfs, &newcwd, &prevdir.m);
///         if (err) {
///             return err;
///         }
///     }
///
///     return 0;
/// }
/// #endif
///
/// static lfs_ssize_t lfs_getattr_(lfs_t *lfs, const char *path,
///         uint8_t type, void *buffer, lfs_size_t size) {
///     lfs_mdir_t cwd;
///     lfs_stag_t tag = lfs_dir_find(lfs, &cwd, &path, NULL);
///     if (tag < 0) {
///         return tag;
///     }
///
///     uint16_t id = lfs_tag_id(tag);
///     if (id == 0x3ff) {
///         // special case for root
///         id = 0;
///         int err = lfs_dir_fetch(lfs, &cwd, lfs->root);
///         if (err) {
///             return err;
///         }
///     }
///
///     tag = lfs_dir_get(lfs, &cwd, LFS_MKTAG(0x7ff, 0x3ff, 0),
///             LFS_MKTAG(LFS_TYPE_USERATTR + type,
///                 id, lfs_min(size, lfs->attr_max)),
///             buffer);
///     if (tag < 0) {
///         if (tag == LFS_ERR_NOENT) {
///             return LFS_ERR_NOATTR;
///         }
///
///         return tag;
///     }
///
///     return lfs_tag_size(tag);
/// }
/// ```
fn slice_until_nul(ptr: *const u8) -> &'static [u8] {
    if ptr.is_null() {
        return &[];
    }
    unsafe {
        let mut len = 0;
        #[cfg(feature = "loop_limits")]
        const MAX_SLICE_NUL_ITER: u32 = 4096;
        #[cfg(feature = "loop_limits")]
        let mut iter: u32 = 0;
        while *ptr.add(len) != 0 {
            #[cfg(feature = "loop_limits")]
            {
                if iter >= MAX_SLICE_NUL_ITER {
                    panic!("loop_limits: MAX_SLICE_NUL_ITER ({}) exceeded", MAX_SLICE_NUL_ITER);
                }
                iter += 1;
            }
            len += 1;
        }
        core::slice::from_raw_parts(ptr, len)
    }
}

pub fn lfs_rename_(lfs: *mut super::lfs::Lfs, oldpath: *const u8, newpath: *const u8) -> i32 {
    let err = lfs_fs_forceconsistency(lfs);
    if err != 0 {
        return err;
    }

    unsafe {
        let mut oldcwd = LfsMdir {
            pair: [0, 0],
            rev: 0,
            off: 0,
            etag: 0,
            count: 0,
            erased: false,
            split: false,
            tail: [(*lfs).root[0], (*lfs).root[1]],
        };
        let mut oldpath_ptr = oldpath;
        let oldtag = lfs_dir_find(lfs, &mut oldcwd, &mut oldpath_ptr, core::ptr::null_mut());
        if oldtag < 0 || lfs_tag_id(oldtag as u32) == 0x3ff {
            return if oldtag < 0 { oldtag } else { LFS_ERR_INVAL };
        }

        let mut newcwd = LfsMdir {
            pair: [0, 0],
            rev: 0,
            off: 0,
            etag: 0,
            count: 0,
            erased: false,
            split: false,
            tail: [(*lfs).root[0], (*lfs).root[1]],
        };
        let mut newpath_ptr = newpath;
        let mut newid: u16 = 0;
        let prevtag = lfs_dir_find(lfs, &mut newcwd, &mut newpath_ptr, &mut newid);
        let newpath_slice = slice_until_nul(newpath_ptr);
        if (prevtag < 0 || lfs_tag_id(prevtag as u32) == 0x3ff)
            && !(prevtag == LFS_ERR_NOENT && lfs_path_islast(newpath_slice))
        {
            return if prevtag < 0 { prevtag } else { LFS_ERR_INVAL };
        }

        let samepair = lfs_pair_cmp(&oldcwd.pair, &newcwd.pair) == 0;
        let mut newoldid = lfs_tag_id(oldtag as u32);

        let mut prevdir = LfsMlist {
            next: (*lfs).mlist,
            id: 0,
            type_: 0,
            m: core::mem::zeroed(),
        };

        if prevtag == LFS_ERR_NOENT {
            if lfs_path_isdir(newpath_slice)
                && u32::from(lfs_tag_type3(oldtag as u32)) != LFS_TYPE_DIR
            {
                return LFS_ERR_NOTDIR;
            }
            let nlen = lfs_path_namelen(newpath_slice);
            if nlen > (*lfs).name_max {
                return LFS_ERR_NAMETOOLONG;
            }
            if samepair && newid <= newoldid {
                newoldid += 1;
            }
        } else if u32::from(lfs_tag_type3(prevtag as u32))
            != u32::from(lfs_tag_type3(oldtag as u32))
        {
            return if u32::from(lfs_tag_type3(prevtag as u32)) == LFS_TYPE_DIR {
                LFS_ERR_ISDIR
            } else {
                LFS_ERR_NOTDIR
            };
        } else if samepair && newid == newoldid {
            return 0;
        } else if u32::from(lfs_tag_type3(prevtag as u32)) == LFS_TYPE_DIR {
            let mut prevpair: [lfs_block_t; 2] = [0, 0];
            let res = lfs_dir_get(
                lfs,
                &newcwd,
                lfs_mktag(0x700, 0x3ff, 0),
                lfs_mktag(LFS_TYPE_STRUCT, newid as u32, 8),
                prevpair.as_mut_ptr() as *mut core::ffi::c_void,
            );
            if res < 0 {
                return res;
            }
            lfs_pair_fromle32(&mut prevpair);

            let err = lfs_dir_fetch(lfs, &mut prevdir.m, &prevpair);
            if err != 0 {
                return err;
            }
            if prevdir.m.count > 0 || prevdir.m.split {
                return LFS_ERR_NOTEMPTY;
            }
            let err = lfs_fs_preporphans(lfs, 1);
            if err != 0 {
                return err;
            }
            prevdir.type_ = 0;
            prevdir.id = 0;
            (*lfs).mlist = &prevdir as *const _ as *mut _;
        }

        if !samepair {
            lfs_fs_prepmove(lfs, newoldid as u16, &oldcwd.pair);
        }

        let nlen = lfs_path_namelen(newpath_slice);
        let attrs = [
            lfs_mattr {
                tag: lfs_mktag_if(prevtag != LFS_ERR_NOENT, LFS_TYPE_DELETE, newid as u32, 0),
                buffer: core::ptr::null(),
            },
            lfs_mattr {
                tag: lfs_mktag(LFS_TYPE_CREATE, newid as u32, 0),
                buffer: core::ptr::null(),
            },
            lfs_mattr {
                tag: lfs_mktag(u32::from(lfs_tag_type3(oldtag as u32)), newid as u32, nlen),
                buffer: newpath_ptr as *const core::ffi::c_void,
            },
            lfs_mattr {
                tag: lfs_mktag(
                    LFS_FROM_MOVE,
                    newid as u32,
                    lfs_tag_id(oldtag as u32) as u32,
                ),
                buffer: &oldcwd as *const _ as *const core::ffi::c_void,
            },
            lfs_mattr {
                tag: lfs_mktag_if(samepair, LFS_TYPE_DELETE, newoldid as u32, 0),
                buffer: core::ptr::null(),
            },
        ];
        let err = lfs_dir_commit(lfs, &mut newcwd, attrs.as_ptr() as *const _, 5);
        (*lfs).mlist = prevdir.next;
        if err != 0 {
            return err;
        }

        if !samepair && lfs_gstate_hasmove(&(*lfs).gstate) {
            lfs_fs_prepmove(lfs, 0x3ff, core::ptr::null());
            let attrs2 = [lfs_mattr {
                tag: lfs_mktag(LFS_TYPE_DELETE, lfs_tag_id(oldtag as u32) as u32, 0),
                buffer: core::ptr::null(),
            }];
            let err = lfs_dir_commit(lfs, &mut oldcwd, attrs2.as_ptr() as *const _, 1);
            (*lfs).mlist = prevdir.next;
            if err != 0 {
                return err;
            }
        }

        if lfs_gstate_hasorphans(&(*lfs).gstate) {
            crate::lfs_assert!(prevtag != LFS_ERR_NOENT);
            crate::lfs_assert!(u32::from(lfs_tag_type3(prevtag as u32)) == LFS_TYPE_DIR);

            let err = lfs_fs_preporphans(lfs, -1);
            if err != 0 {
                return err;
            }
            let err = lfs_fs_pred(lfs, &prevdir.m.pair, &mut newcwd);
            if err != 0 {
                return err;
            }
            lfs_dir_drop(lfs, &mut newcwd, &prevdir.m)
        } else {
            0
        }
    }
}
