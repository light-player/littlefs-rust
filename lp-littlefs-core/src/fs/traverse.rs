//! FS traverse. Per lfs.c lfs_fs_traverse_.
//
/// Per lfs.c lfs_fs_traverse_ (lines 4693-4794)
///
/// C:
/// ```c
/// int lfs_fs_traverse_(lfs_t *lfs,
///         int (*cb)(void *data, lfs_block_t block), void *data,
///         bool includeorphans) {
///     // iterate over metadata pairs
///     lfs_mdir_t dir = {.tail = {0, 1}};
///
/// #ifdef LFS_MIGRATE
///     // also consider v1 blocks during migration
///     if (lfs->lfs1) {
///         int err = lfs1_traverse(lfs, cb, data);
///         if (err) {
///             return err;
///         }
///
///         dir.tail[0] = lfs->root[0];
///         dir.tail[1] = lfs->root[1];
///     }
/// #endif
///
///     struct lfs_tortoise_t tortoise = {
///         .pair = {LFS_BLOCK_NULL, LFS_BLOCK_NULL},
///         .i = 1,
///         .period = 1,
///     };
///     int err = LFS_ERR_OK;
///     while (!lfs_pair_isnull(dir.tail)) {
///         err = lfs_tortoise_detectcycles(&dir, &tortoise);
///         if (err < 0) {
///             return LFS_ERR_CORRUPT;
///         }
///
///         for (int i = 0; i < 2; i++) {
///             int err = cb(data, dir.tail[i]);
///             if (err) {
///                 return err;
///             }
///         }
///
///         // iterate through ids in directory
///         int err = lfs_dir_fetch(lfs, &dir, dir.tail);
///         if (err) {
///             return err;
///         }
///
///         for (uint16_t id = 0; id < dir.count; id++) {
///             struct lfs_ctz ctz;
///             lfs_stag_t tag = lfs_dir_get(lfs, &dir, LFS_MKTAG(0x700, 0x3ff, 0),
///                     LFS_MKTAG(LFS_TYPE_STRUCT, id, sizeof(ctz)), &ctz);
///             if (tag < 0) {
///                 if (tag == LFS_ERR_NOENT) {
///                     continue;
///                 }
///                 return tag;
///             }
///             lfs_ctz_fromle32(&ctz);
///
///             if (lfs_tag_type3(tag) == LFS_TYPE_CTZSTRUCT) {
///                 err = lfs_ctz_traverse(lfs, NULL, &lfs->rcache,
///                         ctz.head, ctz.size, cb, data);
///                 if (err) {
///                     return err;
///                 }
///             } else if (includeorphans &&
///                     lfs_tag_type3(tag) == LFS_TYPE_DIRSTRUCT) {
///                 for (int i = 0; i < 2; i++) {
///                     err = cb(data, (&ctz.head)[i]);
///                     if (err) {
///                         return err;
///                     }
///                 }
///             }
///         }
///     }
///
/// #ifndef LFS_READONLY
///     // iterate over any open files
///     for (lfs_file_t *f = (lfs_file_t*)lfs->mlist; f; f = f->next) {
///         if (f->type != LFS_TYPE_REG) {
///             continue;
///         }
///
///         if ((f->flags & LFS_F_DIRTY) && !(f->flags & LFS_F_INLINE)) {
///             int err = lfs_ctz_traverse(lfs, &f->cache, &lfs->rcache,
///                     f->ctz.head, f->ctz.size, cb, data);
///             if (err) {
///                 return err;
///             }
///         }
///
///         if ((f->flags & LFS_F_WRITING) && !(f->flags & LFS_F_INLINE)) {
///             int err = lfs_ctz_traverse(lfs, &f->cache, &lfs->rcache,
///                     f->block, f->pos, cb, data);
///             if (err) {
///                 return err;
///             }
///         }
///     }
/// #endif
///
///     return 0;
/// }
///
/// ```
/// Translation docs: Traverses all blocks in use by the filesystem, calling cb for each.
/// Used by block allocator (lfs_alloc_scan) to build the free-block bitmap.
/// includeorphans: when true, include directory struct blocks in the traversal.
///
/// C: lfs.c:4693-4794
pub fn lfs_fs_traverse_(
    lfs: *mut super::lfs::Lfs,
    cb: Option<unsafe extern "C" fn(*mut core::ffi::c_void, crate::types::lfs_block_t) -> i32>,
    data: *mut core::ffi::c_void,
    includeorphans: bool,
) -> i32 {
    use crate::dir::fetch::lfs_dir_fetch;
    use crate::dir::traverse::lfs_dir_get;
    use crate::error::LFS_ERR_CORRUPT;
    use crate::file::ctz::lfs_ctz_traverse;
    use crate::fs::mount::{lfs_tortoise_detectcycles, LfsTortoise};
    use crate::lfs_type::lfs_type::{LFS_TYPE_CTZSTRUCT, LFS_TYPE_DIRSTRUCT};
    use crate::tag::{lfs_mktag, lfs_tag_type3};
    use crate::types::{lfs_block_t, LFS_BLOCK_NULL};
    use crate::util::{lfs_pair_fromle32, lfs_pair_isnull};

    if cb.is_none() {
        return 0;
    }
    let cb = cb.unwrap();

    unsafe {
        // iterate over metadata pairs
        let mut dir = crate::dir::LfsMdir {
            pair: [0, 0],
            rev: 0,
            off: 0,
            etag: 0,
            count: 0,
            erased: false,
            split: false,
            tail: [0, 1],
        };
        let mut tortoise = LfsTortoise {
            pair: [LFS_BLOCK_NULL, LFS_BLOCK_NULL],
            i: 1,
            period: 1,
        };

        #[cfg(feature = "loop_limits")]
        const MAX_TRAVERSE_TAIL: u32 = 512;
        #[cfg(feature = "loop_limits")]
        let mut iter: u32 = 0;
        crate::lfs_trace!("fs_traverse: tail loop start");
        while !lfs_pair_isnull(&dir.tail) {
            #[cfg(feature = "loop_limits")]
            {
                if iter >= MAX_TRAVERSE_TAIL {
                    panic!(
                        "loop_limits: MAX_TRAVERSE_TAIL ({}) exceeded",
                        MAX_TRAVERSE_TAIL
                    );
                }
                if iter > 0 && iter.is_multiple_of(20) {
                    crate::lfs_trace!("fs_traverse: iter={} tail={:?}", iter, dir.tail);
                }
                iter += 1;
            }
            let err = lfs_tortoise_detectcycles(&dir, &mut tortoise);
            if err < 0 {
                return crate::lfs_err!(LFS_ERR_CORRUPT);
            }

            for i in 0..2 {
                let err = cb(data, dir.tail[i]);
                if err != 0 {
                    return crate::lfs_pass_err!(err);
                }
            }

            // iterate through ids in directory
            crate::lfs_trace!("fs_traverse: fetch tail={:?} count={}", dir.tail, dir.count);
            let err = lfs_dir_fetch(lfs, &mut dir, &dir.tail);
            if err != 0 {
                return crate::lfs_pass_err!(err);
            }

            for id in 0..dir.count {
                let mut raw: [lfs_block_t; 2] = [0, 0];
                let tag = lfs_dir_get(
                    lfs,
                    &dir,
                    lfs_mktag(0x700, 0x3ff, 0),
                    lfs_mktag(crate::lfs_type::lfs_type::LFS_TYPE_STRUCT, id as u32, 8),
                    raw.as_mut_ptr() as *mut core::ffi::c_void,
                );
                if tag < 0 {
                    if tag == crate::error::LFS_ERR_NOENT {
                        continue;
                    }
                    return tag;
                }
                lfs_pair_fromle32(&mut raw);

                if u32::from(lfs_tag_type3(tag as u32)) == LFS_TYPE_CTZSTRUCT {
                    let err = lfs_ctz_traverse(
                        lfs,
                        core::ptr::null(),
                        &mut (*lfs).rcache,
                        raw[0],
                        raw[1],
                        Some(cb),
                        data,
                    );
                    if err != 0 {
                        return crate::lfs_pass_err!(err);
                    }
                } else if includeorphans
                    && u32::from(lfs_tag_type3(tag as u32)) == LFS_TYPE_DIRSTRUCT
                {
                    #[allow(clippy::needless_range_loop)] // Rule 2: preserve C loop structure
                    for i in 0..2 {
                        let err = cb(data, raw[i]);
                        if err != 0 {
                            return crate::lfs_pass_err!(err);
                        }
                    }
                }
            }
        }

        // iterate over any open files
        use crate::dir::LfsMlist;
        use crate::file::ctz::lfs_ctz_traverse;
        use crate::file::LfsFile;
        use crate::lfs_type::lfs_open_flags::{LFS_F_DIRTY, LFS_F_INLINE, LFS_F_WRITING};
        use crate::lfs_type::lfs_type::LFS_TYPE_REG;

        let mut m = (*lfs).mlist;
        #[cfg(feature = "loop_limits")]
        const MAX_MLIST: u32 = 64;
        #[cfg(feature = "loop_limits")]
        let mut mlist_iter: u32 = 0;
        while !m.is_null() {
            #[cfg(feature = "loop_limits")]
            {
                if mlist_iter >= MAX_MLIST {
                    panic!("loop_limits: MAX_MLIST ({}) exceeded", MAX_MLIST);
                }
                mlist_iter += 1;
            }
            let f = m as *mut LfsFile;
            let f_ref = &*f;
            if f_ref.type_ as u32 == LFS_TYPE_REG {
                if (f_ref.flags as i32 & LFS_F_DIRTY) != 0
                    && (f_ref.flags as i32 & LFS_F_INLINE) == 0
                {
                    let err = lfs_ctz_traverse(
                        lfs,
                        &(*f).cache,
                        &mut (*lfs).rcache,
                        f_ref.ctz.head,
                        f_ref.ctz.size,
                        Some(cb),
                        data,
                    );
                    if err != 0 {
                        return crate::lfs_pass_err!(err);
                    }
                }
                if (f_ref.flags as i32 & LFS_F_WRITING) != 0
                    && (f_ref.flags as i32 & LFS_F_INLINE) == 0
                {
                    let err = lfs_ctz_traverse(
                        lfs,
                        &(*f).cache,
                        &mut (*lfs).rcache,
                        f_ref.block,
                        f_ref.pos,
                        Some(cb),
                        data,
                    );
                    if err != 0 {
                        return crate::lfs_pass_err!(err);
                    }
                }
            }
            m = (*m).next;
        }

        0
    }
}
