//! mkdir. Per lfs.c mkdir_.

use crate::block_alloc::alloc::lfs_alloc_ckpoint;
use crate::dir::commit::{lfs_dir_alloc, lfs_dir_commit};
use crate::dir::fetch::lfs_dir_fetch;
use crate::dir::find::lfs_dir_find;
use crate::dir::LfsMlist;
use crate::error::{LFS_ERR_EXIST, LFS_ERR_NAMETOOLONG, LFS_ERR_NOENT};
use crate::fs::superblock::{lfs_fs_forceconsistency, lfs_fs_preporphans};
use crate::lfs_type::lfs_type::{
    LFS_TYPE_CREATE, LFS_TYPE_DIR, LFS_TYPE_DIRSTRUCT, LFS_TYPE_SOFTTAIL,
};
use crate::tag::{lfs_mattr, lfs_mktag, lfs_mktag_if};
use crate::util::{lfs_pair_fromle32, lfs_pair_tole32, lfs_path_islast, lfs_path_namelen};

/// Per lfs.c lfs_mkdir_ (lines 2625-2719)
///
/// C:
/// ```c
/// static int lfs_mkdir_(lfs_t *lfs, const char *path) {
///     // deorphan if we haven't yet, needed at most once after poweron
///     int err = lfs_fs_forceconsistency(lfs);
///     if (err) {
///         return err;
///     }
///
///     struct lfs_mlist cwd;
///     cwd.next = lfs->mlist;
///     uint16_t id;
///     err = lfs_dir_find(lfs, &cwd.m, &path, &id);
///     if (!(err == LFS_ERR_NOENT && lfs_path_islast(path))) {
///         return (err < 0) ? err : LFS_ERR_EXIST;
///     }
///
///     // check that name fits
///     lfs_size_t nlen = lfs_path_namelen(path);
///     if (nlen > lfs->name_max) {
///         return LFS_ERR_NAMETOOLONG;
///     }
///
///     // build up new directory
///     lfs_alloc_ckpoint(lfs);
///     lfs_mdir_t dir;
///     err = lfs_dir_alloc(lfs, &dir);
///     if (err) {
///         return err;
///     }
///
///     // find end of list
///     lfs_mdir_t pred = cwd.m;
///     while (pred.split) {
///         err = lfs_dir_fetch(lfs, &pred, pred.tail);
///         if (err) {
///             return err;
///         }
///     }
///
///     // setup dir
///     lfs_pair_tole32(pred.tail);
///     err = lfs_dir_commit(lfs, &dir, LFS_MKATTRS(
///             {LFS_MKTAG(LFS_TYPE_SOFTTAIL, 0x3ff, 8), pred.tail}));
///     lfs_pair_fromle32(pred.tail);
///     if (err) {
///         return err;
///     }
///
///     // current block not end of list?
///     if (cwd.m.split) {
///         // update tails, this creates a desync
///         err = lfs_fs_preporphans(lfs, +1);
///         if (err) {
///             return err;
///         }
///
///         // it's possible our predecessor has to be relocated, and if
///         // our parent is our predecessor's predecessor, this could have
///         // caused our parent to go out of date, fortunately we can hook
///         // ourselves into littlefs to catch this
///         cwd.type = 0;
///         cwd.id = 0;
///         lfs->mlist = &cwd;
///
///         lfs_pair_tole32(dir.pair);
///         err = lfs_dir_commit(lfs, &pred, LFS_MKATTRS(
///                 {LFS_MKTAG(LFS_TYPE_SOFTTAIL, 0x3ff, 8), dir.pair}));
///         lfs_pair_fromle32(dir.pair);
///         if (err) {
///             lfs->mlist = cwd.next;
///             return err;
///         }
///
///         lfs->mlist = cwd.next;
///         err = lfs_fs_preporphans(lfs, -1);
///         if (err) {
///             return err;
///         }
///     }
///
///     // now insert into our parent block
///     lfs_pair_tole32(dir.pair);
///     err = lfs_dir_commit(lfs, &cwd.m, LFS_MKATTRS(
///             {LFS_MKTAG(LFS_TYPE_CREATE, id, 0), NULL},
///             {LFS_MKTAG(LFS_TYPE_DIR, id, nlen), path},
///             {LFS_MKTAG(LFS_TYPE_DIRSTRUCT, id, 8), dir.pair},
///             {LFS_MKTAG_IF(!cwd.m.split,
///                 LFS_TYPE_SOFTTAIL, 0x3ff, 8), dir.pair}));
///     lfs_pair_fromle32(dir.pair);
///     if (err) {
///         return err;
///     }
///
///     return 0;
/// }
/// #endif
/// ```
pub fn lfs_mkdir_(lfs: *mut super::lfs::Lfs, path: *const u8) -> i32 {
    let err = lfs_fs_forceconsistency(lfs);
    if err != 0 {
        return err;
    }

    unsafe {
        let mut cwd = LfsMlist {
            next: (*lfs).mlist,
            m: core::mem::zeroed(),
            type_: 0,
            id: 0,
        };
        cwd.m.tail = [(*lfs).root[0], (*lfs).root[1]];

        let mut path_ptr = path;
        let mut id: u16 = 0;
        let find_err = lfs_dir_find(lfs, &mut cwd.m, &mut path_ptr, &mut id);
        if !(find_err == LFS_ERR_NOENT && lfs_path_islast(slice_until_nul(path_ptr))) {
            return if find_err < 0 {
                find_err
            } else {
                LFS_ERR_EXIST
            };
        }

        let path_slice = slice_until_nul(path_ptr);
        let nlen = lfs_path_namelen(path_slice);
        if nlen > (*lfs).name_max {
            return LFS_ERR_NAMETOOLONG;
        }

        lfs_alloc_ckpoint(lfs);
        let mut dir = core::mem::zeroed();
        let err = lfs_dir_alloc(lfs, &mut dir);
        if err != 0 {
            return err;
        }

        let mut pred = cwd.m;
        #[cfg(feature = "loop_limits")]
        const MAX_MKDIR_PRED_ITER: u32 = 2048;
        #[cfg(feature = "loop_limits")]
        let mut iter: u32 = 0;
        while pred.split {
            #[cfg(feature = "loop_limits")]
            {
                if iter >= MAX_MKDIR_PRED_ITER {
                    panic!(
                        "loop_limits: MAX_MKDIR_PRED_ITER ({}) exceeded",
                        MAX_MKDIR_PRED_ITER
                    );
                }
                iter += 1;
            }
            let err = lfs_dir_fetch(lfs, &mut pred, &pred.tail);
            if err != 0 {
                return err;
            }
        }

        lfs_pair_tole32(&mut pred.tail);
        let attrs1 = [lfs_mattr {
            tag: lfs_mktag(LFS_TYPE_SOFTTAIL, 0x3ff, 8),
            buffer: pred.tail.as_ptr() as *const core::ffi::c_void,
        }];
        let err = lfs_dir_commit(lfs, &mut dir, attrs1.as_ptr() as *const _, 1);
        lfs_pair_fromle32(&mut pred.tail);
        if err != 0 {
            return err;
        }

        if cwd.m.split {
            let err = lfs_fs_preporphans(lfs, 1);
            if err != 0 {
                return err;
            }

            cwd.type_ = 0;
            cwd.id = 0;
            (*lfs).mlist = &cwd as *const _ as *mut _;

            lfs_pair_tole32(&mut dir.pair);
            let attrs2 = [lfs_mattr {
                tag: lfs_mktag(LFS_TYPE_SOFTTAIL, 0x3ff, 8),
                buffer: dir.pair.as_ptr() as *const core::ffi::c_void,
            }];
            let err = lfs_dir_commit(lfs, &mut pred, attrs2.as_ptr() as *const _, 1);
            lfs_pair_fromle32(&mut dir.pair);
            (*lfs).mlist = cwd.next;
            if err != 0 {
                return err;
            }

            let err = lfs_fs_preporphans(lfs, -1);
            if err != 0 {
                return err;
            }
        }

        lfs_pair_tole32(&mut dir.pair);
        let attrs3 = [
            lfs_mattr {
                tag: lfs_mktag(LFS_TYPE_CREATE, id as u32, 0),
                buffer: core::ptr::null(),
            },
            lfs_mattr {
                tag: lfs_mktag(LFS_TYPE_DIR, id as u32, nlen),
                buffer: path_ptr as *const core::ffi::c_void,
            },
            lfs_mattr {
                tag: lfs_mktag(LFS_TYPE_DIRSTRUCT, id as u32, 8),
                buffer: dir.pair.as_ptr() as *const core::ffi::c_void,
            },
            lfs_mattr {
                tag: lfs_mktag_if(!cwd.m.split, LFS_TYPE_SOFTTAIL, 0x3ff, 8),
                buffer: dir.pair.as_ptr() as *const core::ffi::c_void,
            },
        ];
        let err = lfs_dir_commit(lfs, &mut cwd.m, attrs3.as_ptr() as *const _, 4);
        lfs_pair_fromle32(&mut dir.pair);
        err
    }
}

/// Helper: slice from pointer until null byte.
fn slice_until_nul(ptr: *const u8) -> &'static [u8] {
    if ptr.is_null() {
        return &[];
    }
    unsafe {
        let mut len = 0;
        while *ptr.add(len) != 0 {
            len += 1;
        }
        core::slice::from_raw_parts(ptr, len)
    }
}
