//! File operations. Per lfs.c lfs_file_opencfg_, lfs_file_close_, lfs_file_sync_, etc.

use crate::bd::bd::{lfs_bd_read, lfs_cache_drop, lfs_cache_zero};
use crate::dir::traverse::lfs_dir_getread;
use crate::dir::LfsMdir;
use crate::file::ctz::lfs_ctz_find;
use crate::file::LfsFile;
use crate::lfs_info::LfsFileConfig;
use crate::lfs_type::lfs_open_flags::{
    LFS_F_DIRTY, LFS_F_INLINE, LFS_F_READING, LFS_F_WRITING, LFS_O_RDONLY,
};
use crate::lfs_type::lfs_type::LFS_TYPE_INLINESTRUCT;
use crate::tag::lfs_mktag;
use crate::types::LFS_BLOCK_INLINE;
use crate::types::{lfs_block_t, lfs_off_t, lfs_size_t};
use crate::util::lfs_min;

/// Per lfs.c lfs_file_opencfg_ (lines 3065-3236)
///
/// C:
/// ```c
/// static int lfs_file_opencfg_(lfs_t *lfs, lfs_file_t *file,
///         const char *path, int flags,
///         const struct lfs_file_config *cfg) {
/// #ifndef LFS_READONLY
///     // deorphan if we haven't yet, needed at most once after poweron
///     if ((flags & LFS_O_WRONLY) == LFS_O_WRONLY) {
///         int err = lfs_fs_forceconsistency(lfs);
///         if (err) {
///             return err;
///         }
///     }
/// #else
///     LFS_ASSERT((flags & LFS_O_RDONLY) == LFS_O_RDONLY);
/// #endif
///
///     // setup simple file details
///     int err;
///     file->cfg = cfg;
///     file->flags = flags;
///     file->pos = 0;
///     file->off = 0;
///     file->cache.buffer = NULL;
///
///     // allocate entry for file if it doesn't exist
///     lfs_stag_t tag = lfs_dir_find(lfs, &file->m, &path, &file->id);
///     if (tag < 0 && !(tag == LFS_ERR_NOENT && lfs_path_islast(path))) {
///         err = tag;
///         goto cleanup;
///     }
///
///     // get id, add to list of mdirs to catch update changes
///     file->type = LFS_TYPE_REG;
///     lfs_mlist_append(lfs, (struct lfs_mlist *)file);
///
/// #ifdef LFS_READONLY
///     if (tag == LFS_ERR_NOENT) {
///         err = LFS_ERR_NOENT;
///         goto cleanup;
/// #else
///     if (tag == LFS_ERR_NOENT) {
///         if (!(flags & LFS_O_CREAT)) {
///             err = LFS_ERR_NOENT;
///             goto cleanup;
///         }
///
///         // don't allow trailing slashes
///         if (lfs_path_isdir(path)) {
///             err = LFS_ERR_NOTDIR;
///             goto cleanup;
///         }
///
///         // check that name fits
///         lfs_size_t nlen = lfs_path_namelen(path);
///         if (nlen > lfs->name_max) {
///             err = LFS_ERR_NAMETOOLONG;
///             goto cleanup;
///         }
///
///         // get next slot and create entry to remember name
///         err = lfs_dir_commit(lfs, &file->m, LFS_MKATTRS(
///                 {LFS_MKTAG(LFS_TYPE_CREATE, file->id, 0), NULL},
///                 {LFS_MKTAG(LFS_TYPE_REG, file->id, nlen), path},
///                 {LFS_MKTAG(LFS_TYPE_INLINESTRUCT, file->id, 0), NULL}));
///
///         // it may happen that the file name doesn't fit in the metadata blocks, e.g., a 256 byte file name will
///         // not fit in a 128 byte block.
///         err = (err == LFS_ERR_NOSPC) ? LFS_ERR_NAMETOOLONG : err;
///         if (err) {
///             goto cleanup;
///         }
///
///         tag = LFS_MKTAG(LFS_TYPE_INLINESTRUCT, 0, 0);
///     } else if (flags & LFS_O_EXCL) {
///         err = LFS_ERR_EXIST;
///         goto cleanup;
/// #endif
///     } else if (lfs_tag_type3(tag) != LFS_TYPE_REG) {
///         err = LFS_ERR_ISDIR;
///         goto cleanup;
/// #ifndef LFS_READONLY
///     } else if (flags & LFS_O_TRUNC) {
///         // truncate if requested
///         tag = LFS_MKTAG(LFS_TYPE_INLINESTRUCT, file->id, 0);
///         file->flags |= LFS_F_DIRTY;
/// #endif
///     } else {
///         // try to load what's on disk, if it's inlined we'll fix it later
///         tag = lfs_dir_get(lfs, &file->m, LFS_MKTAG(0x700, 0x3ff, 0),
///                 LFS_MKTAG(LFS_TYPE_STRUCT, file->id, 8), &file->ctz);
///         if (tag < 0) {
///             err = tag;
///             goto cleanup;
///         }
///         lfs_ctz_fromle32(&file->ctz);
///     }
///
///     // fetch attrs
///     for (unsigned i = 0; i < file->cfg->attr_count; i++) {
///         // if opened for read / read-write operations
///         if ((file->flags & LFS_O_RDONLY) == LFS_O_RDONLY) {
///             lfs_stag_t res = lfs_dir_get(lfs, &file->m,
///                     LFS_MKTAG(0x7ff, 0x3ff, 0),
///                     LFS_MKTAG(LFS_TYPE_USERATTR + file->cfg->attrs[i].type,
///                         file->id, file->cfg->attrs[i].size),
///                         file->cfg->attrs[i].buffer);
///             if (res < 0 && res != LFS_ERR_NOENT) {
///                 err = res;
///                 goto cleanup;
///             }
///         }
///
/// #ifndef LFS_READONLY
///         // if opened for write / read-write operations
///         if ((file->flags & LFS_O_WRONLY) == LFS_O_WRONLY) {
///             if (file->cfg->attrs[i].size > lfs->attr_max) {
///                 err = LFS_ERR_NOSPC;
///                 goto cleanup;
///             }
///
///             file->flags |= LFS_F_DIRTY;
///         }
/// #endif
///     }
///
///     // allocate buffer if needed
///     if (file->cfg->buffer) {
///         file->cache.buffer = file->cfg->buffer;
///     } else {
///         file->cache.buffer = lfs_malloc(lfs->cfg->cache_size);
///         if (!file->cache.buffer) {
///             err = LFS_ERR_NOMEM;
///             goto cleanup;
///         }
///     }
///
///     // zero to avoid information leak
///     lfs_cache_zero(lfs, &file->cache);
///
///     if (lfs_tag_type3(tag) == LFS_TYPE_INLINESTRUCT) {
///         // load inline files
///         file->ctz.head = LFS_BLOCK_INLINE;
///         file->ctz.size = lfs_tag_size(tag);
///         file->flags |= LFS_F_INLINE;
///         file->cache.block = file->ctz.head;
///         file->cache.off = 0;
///         file->cache.size = lfs->cfg->cache_size;
///
///         // don't always read (may be new/trunc file)
///         if (file->ctz.size > 0) {
///             lfs_stag_t res = lfs_dir_get(lfs, &file->m,
///                     LFS_MKTAG(0x700, 0x3ff, 0),
///                     LFS_MKTAG(LFS_TYPE_STRUCT, file->id,
///                         lfs_min(file->cache.size, 0x3fe)),
///                     file->cache.buffer);
///             if (res < 0) {
///                 err = res;
///                 goto cleanup;
///             }
///         }
///     }
///
///     return 0;
///
/// cleanup:
///     // clean up lingering resources
/// #ifndef LFS_READONLY
///     file->flags |= LFS_F_ERRED;
/// #endif
///     lfs_file_close_(lfs, file);
///     return err;
/// }
/// ```
pub fn lfs_file_opencfg_(
    lfs: *mut crate::fs::Lfs,
    file: *mut LfsFile,
    path: *const i8,
    flags: i32,
    cfg: *const LfsFileConfig,
) -> i32 {
    use crate::block_alloc::alloc::lfs_alloc_ckpoint;
    use crate::dir::find::lfs_dir_find;
    use crate::dir::lfs_mlist::lfs_mlist_append;
    use crate::dir::traverse::lfs_dir_get;
    use crate::error::{
        LFS_ERR_EXIST, LFS_ERR_ISDIR, LFS_ERR_NAMETOOLONG, LFS_ERR_NOENT, LFS_ERR_NOMEM,
    };
    use crate::file::lfs_ctz::lfs_ctz_fromle32;
    use crate::fs::superblock::lfs_fs_forceconsistency;
    use crate::lfs_type::lfs_open_flags::{LFS_O_CREAT, LFS_O_EXCL};
    use crate::lfs_type::lfs_type::{LFS_TYPE_CREATE, LFS_TYPE_REG};
    use crate::tag::{lfs_mktag, lfs_tag_size, lfs_tag_type3};
    use crate::types::LFS_BLOCK_INLINE;
    use crate::util::{
        lfs_min, lfs_path_isdir, lfs_path_islast, lfs_path_namelen, lfs_path_slice_from_cstr,
    };

    let path_u8 = path as *const u8;
    unsafe {
        if (flags & 2) != 0 {
            let err = lfs_fs_forceconsistency(lfs);
            if err != 0 {
                return err;
            }
        }

        let file_ref = &mut *file;
        file_ref.cfg = cfg;
        file_ref.flags = flags as u32;
        file_ref.pos = 0;
        file_ref.off = 0;
        file_ref.cache.buffer = core::ptr::null_mut();

        let mut path_ptr = path_u8;
        let mut tag = lfs_dir_find(lfs, &mut file_ref.m, &mut path_ptr, &mut file_ref.id);
        if tag < 0 && !(tag == LFS_ERR_NOENT && lfs_path_islast(lfs_path_slice_from_cstr(path_ptr)))
        {
            let err = tag;
            lfs_file_close_(lfs, file);
            return err;
        }

        file_ref.type_ = LFS_TYPE_REG as u8;
        lfs_mlist_append(lfs, file as *mut crate::dir::LfsMlist);

        if tag == LFS_ERR_NOENT {
            if (flags & LFS_O_CREAT) == 0 {
                lfs_file_close_(lfs, file);
                return LFS_ERR_NOENT;
            }
            if lfs_path_isdir(lfs_path_slice_from_cstr(path_ptr)) {
                lfs_file_close_(lfs, file);
                return crate::error::LFS_ERR_NOTDIR;
            }
            let nlen = lfs_path_namelen(lfs_path_slice_from_cstr(path_ptr));
            if nlen > (*lfs).name_max {
                lfs_file_close_(lfs, file);
                return LFS_ERR_NAMETOOLONG;
            }
            lfs_alloc_ckpoint(lfs);
            let attrs = [
                crate::tag::lfs_mattr {
                    tag: lfs_mktag(LFS_TYPE_CREATE, file_ref.id as u32, 0),
                    buffer: core::ptr::null(),
                },
                crate::tag::lfs_mattr {
                    tag: lfs_mktag(LFS_TYPE_REG, file_ref.id as u32, nlen),
                    buffer: path_ptr as *const core::ffi::c_void,
                },
                crate::tag::lfs_mattr {
                    tag: lfs_mktag(LFS_TYPE_INLINESTRUCT, file_ref.id as u32, 0),
                    buffer: core::ptr::null(),
                },
            ];
            let err = crate::dir::commit::lfs_dir_commit(
                lfs,
                &mut file_ref.m,
                attrs.as_ptr() as *const _,
                3,
            );
            let err = if err == crate::error::LFS_ERR_NOSPC {
                LFS_ERR_NAMETOOLONG
            } else {
                err
            };
            if err != 0 {
                lfs_file_close_(lfs, file);
                return err;
            }
        } else if (flags & LFS_O_EXCL) != 0 {
            lfs_file_close_(lfs, file);
            return LFS_ERR_EXIST;
        } else if u32::from(lfs_tag_type3(tag as u32)) != LFS_TYPE_REG {
            lfs_file_close_(lfs, file);
            return LFS_ERR_ISDIR;
        } else {
            // C: tag = lfs_dir_get(...) — overwrite tag with STRUCT tag for later use
            let struct_tag = lfs_dir_get(
                lfs,
                &file_ref.m as *const _,
                lfs_mktag(0x700, 0x3ff, 0),
                lfs_mktag(
                    crate::lfs_type::lfs_type::LFS_TYPE_STRUCT,
                    file_ref.id as u32,
                    8,
                ),
                &mut file_ref.ctz as *mut _ as *mut core::ffi::c_void,
            );
            if struct_tag < 0 {
                lfs_file_close_(lfs, file);
                return struct_tag;
            }
            tag = struct_tag;
            lfs_ctz_fromle32(&mut file_ref.ctz);
        }

        if !cfg.is_null() && !(*cfg).buffer.is_null() {
            file_ref.cache.buffer = (*cfg).buffer as *mut u8;
        } else {
            #[cfg(feature = "alloc")]
            {
                file_ref.cache.buffer = crate::lfs_alloc_module::lfs_malloc(
                    (*lfs).cfg.as_ref().expect("cfg").cache_size,
                );
            }
            #[cfg(not(feature = "alloc"))]
            {
                lfs_file_close_(lfs, file);
                return LFS_ERR_NOMEM;
            }
            if file_ref.cache.buffer.is_null() {
                lfs_file_close_(lfs, file);
                return LFS_ERR_NOMEM;
            }
        }

        lfs_cache_zero(lfs as *const crate::fs::Lfs, &mut file_ref.cache);

        let tag_val = if tag == LFS_ERR_NOENT {
            lfs_mktag(LFS_TYPE_INLINESTRUCT, 0, 0) as i32
        } else {
            tag
        };
        if u32::from(lfs_tag_type3(tag_val as u32)) == LFS_TYPE_INLINESTRUCT {
            file_ref.ctz.head = LFS_BLOCK_INLINE;
            file_ref.ctz.size = if tag_val == LFS_ERR_NOENT {
                0
            } else {
                lfs_tag_size(tag_val as u32)
            };
            file_ref.flags |= LFS_F_INLINE as u32;
            file_ref.cache.block = file_ref.ctz.head;
            file_ref.cache.off = 0;
            file_ref.cache.size = (*lfs).cfg.as_ref().expect("cfg").cache_size;
            if file_ref.ctz.size > 0 {
                let res = lfs_dir_get(
                    lfs,
                    &file_ref.m as *const _,
                    lfs_mktag(0x700, 0x3ff, 0),
                    lfs_mktag(
                        crate::lfs_type::lfs_type::LFS_TYPE_STRUCT,
                        file_ref.id as u32,
                        lfs_min(file_ref.cache.size, 0x3fe),
                    ),
                    file_ref.cache.buffer as *mut core::ffi::c_void,
                );
                if res < 0 {
                    lfs_file_close_(lfs, file);
                    return res;
                }
            }
        }
    }
    0
}

/// Per lfs.c lfs_file_open_ (lines 3238-3244)
///
/// C: Wrapper that calls opencfg with default config.
pub fn lfs_file_open_(
    lfs: *mut crate::fs::Lfs,
    file: *mut LfsFile,
    path: *const i8,
    flags: i32,
) -> i32 {
    let defaults = LfsFileConfig {
        buffer: core::ptr::null_mut(),
        attrs: core::ptr::null_mut(),
        attr_count: 0,
    };
    lfs_file_opencfg_(lfs, file, path, flags, &defaults)
}

/// Per lfs.c lfs_file_close_ (lines 3246-3264)
///
/// Translation docs: Sync if dirty, remove from mlist, free cache buffer if we allocated it.
///
/// C: lfs.c:3246-3264
pub fn lfs_file_close_(lfs: *mut crate::fs::Lfs, file: *mut LfsFile) -> i32 {
    use crate::dir::lfs_mlist::lfs_mlist_remove;

    let err = lfs_file_sync_(lfs, file);
    if err != 0 {
        return err;
    }

    unsafe {
        lfs_mlist_remove(lfs, file as *mut crate::dir::LfsMlist);

        let cfg = (*file).cfg;
        if !cfg.is_null() && (*cfg).buffer.is_null() {
            #[cfg(feature = "alloc")]
            {
                crate::lfs_alloc_module::lfs_free(
                    (*file).cache.buffer,
                    (*lfs).cfg.as_ref().expect("cfg").cache_size,
                );
            }
        }
    }

    err
}

/// Per lfs.c lfs_file_relocate (lines 3266-3335)
///
/// C:
/// ```c
/// static int lfs_file_relocate(lfs_t *lfs, lfs_file_t *file) {
///     while (true) {
///         // just relocate what exists into new block
///         lfs_block_t nblock;
///         int err = lfs_alloc(lfs, &nblock);
///         if (err) {
///             return err;
///         }
///
///         err = lfs_bd_erase(lfs, nblock);
///         if (err) {
///             if (err == LFS_ERR_CORRUPT) {
///                 goto relocate;
///             }
///             return err;
///         }
///
///         // either read from dirty cache or disk
///         for (lfs_off_t i = 0; i < file->off; i++) {
///             uint8_t data;
///             if (file->flags & LFS_F_INLINE) {
///                 err = lfs_dir_getread(lfs, &file->m,
///                         // note we evict inline files before they can be dirty
///                         NULL, &file->cache, file->off-i,
///                         LFS_MKTAG(0xfff, 0x1ff, 0),
///                         LFS_MKTAG(LFS_TYPE_INLINESTRUCT, file->id, 0),
///                         i, &data, 1);
///                 if (err) {
///                     return err;
///                 }
///             } else {
///                 err = lfs_bd_read(lfs,
///                         &file->cache, &lfs->rcache, file->off-i,
///                         file->block, i, &data, 1);
///                 if (err) {
///                     return err;
///                 }
///             }
///
///             err = lfs_bd_prog(lfs,
///                     &lfs->pcache, &lfs->rcache, true,
///                     nblock, i, &data, 1);
///             if (err) {
///                 if (err == LFS_ERR_CORRUPT) {
///                     goto relocate;
///                 }
///                 return err;
///             }
///         }
///
///         // copy over new state of file
///         memcpy(file->cache.buffer, lfs->pcache.buffer, lfs->cfg->cache_size);
///         file->cache.block = lfs->pcache.block;
///         file->cache.off = lfs->pcache.off;
///         file->cache.size = lfs->pcache.size;
///         lfs_cache_zero(lfs, &lfs->pcache);
///
///         file->block = nblock;
///         file->flags |= LFS_F_WRITING;
///         return 0;
///
/// relocate:
///         LFS_DEBUG("Bad block at 0x%"PRIx32, nblock);
///
///         // just clear cache and try a new block
///         lfs_cache_drop(lfs, &lfs->pcache);
///     }
/// }
/// ```
pub fn lfs_file_relocate(_lfs: *const core::ffi::c_void, _file: *mut LfsFile) -> i32 {
    todo!("lfs_file_relocate")
}

/// Per lfs.c lfs_file_outline (lines 3337-3348)
///
/// C:
/// ```c
/// static int lfs_file_outline(lfs_t *lfs, lfs_file_t *file) {
///     file->off = file->pos;
///     lfs_alloc_ckpoint(lfs);
///     int err = lfs_file_relocate(lfs, file);
///     if (err) {
///         return err;
///     }
///
///     file->flags &= ~LFS_F_INLINE;
///     return 0;
/// }
/// ```
pub fn lfs_file_outline(_lfs: *const core::ffi::c_void, _file: *mut LfsFile) -> i32 {
    todo!("lfs_file_outline")
}

/// Per lfs.c lfs_file_flush (lines 3350-3429)
///
/// C:
/// ```c
/// static int lfs_file_flush(lfs_t *lfs, lfs_file_t *file) {
///     if (file->flags & LFS_F_READING) {
///         if (!(file->flags & LFS_F_INLINE)) {
///             lfs_cache_drop(lfs, &file->cache);
///         }
///         file->flags &= ~LFS_F_READING;
///     }
///
/// #ifndef LFS_READONLY
///     if (file->flags & LFS_F_WRITING) {
///         lfs_off_t pos = file->pos;
///
///         if (!(file->flags & LFS_F_INLINE)) {
///             // copy over anything after current branch
///             lfs_file_t orig = {
///                 .ctz.head = file->ctz.head,
///                 .ctz.size = file->ctz.size,
///                 .flags = LFS_O_RDONLY,
///                 .pos = file->pos,
///                 .cache = lfs->rcache,
///             };
///             lfs_cache_drop(lfs, &lfs->rcache);
///
///             while (file->pos < file->ctz.size) {
///                 // copy over a byte at a time, leave it up to caching
///                 // to make this efficient
///                 uint8_t data;
///                 lfs_ssize_t res = lfs_file_flushedread(lfs, &orig, &data, 1);
///                 if (res < 0) {
///                     return res;
///                 }
///
///                 res = lfs_file_flushedwrite(lfs, file, &data, 1);
///                 if (res < 0) {
///                     return res;
///                 }
///
///                 // keep our reference to the rcache in sync
///                 if (lfs->rcache.block != LFS_BLOCK_NULL) {
///                     lfs_cache_drop(lfs, &orig.cache);
///                     lfs_cache_drop(lfs, &lfs->rcache);
///                 }
///             }
///
///             // write out what we have
///             while (true) {
///                 int err = lfs_bd_flush(lfs, &file->cache, &lfs->rcache, true);
///                 if (err) {
///                     if (err == LFS_ERR_CORRUPT) {
///                         goto relocate;
///                     }
///                     return err;
///                 }
///
///                 break;
///
/// relocate:
///                 LFS_DEBUG("Bad block at 0x%"PRIx32, file->block);
///                 err = lfs_file_relocate(lfs, file);
///                 if (err) {
///                     return err;
///                 }
///             }
///         } else {
///             file->pos = lfs_max(file->pos, file->ctz.size);
///         }
///
///         // actual file updates
///         file->ctz.head = file->block;
///         file->ctz.size = file->pos;
///         file->flags &= ~LFS_F_WRITING;
///         file->flags |= LFS_F_DIRTY;
///
///         file->pos = pos;
///     }
/// #endif
///
///     return 0;
/// }
/// ```
pub fn lfs_file_flush(lfs: *const core::ffi::c_void, file: *mut LfsFile) -> i32 {
    use crate::util::lfs_max;

    unsafe {
        let lfs = lfs as *mut crate::fs::Lfs;
        let file_ref = &mut *file;
        if (file_ref.flags as i32 & LFS_F_READING) != 0 {
            if (file_ref.flags as i32 & LFS_F_INLINE) == 0 {
                lfs_cache_drop(lfs as *const crate::fs::Lfs, &mut file_ref.cache);
            }
            file_ref.flags &= !(LFS_F_READING as u32);
        }

        // F_WRITING path (lfs.c:3358-3425) - inline-only for Phase 04
        if (file_ref.flags as i32 & LFS_F_WRITING) != 0 {
            let pos = file_ref.pos;
            if (file_ref.flags as i32 & LFS_F_INLINE) != 0 {
                file_ref.pos = lfs_max(pos, file_ref.ctz.size);
            } else {
                return crate::error::LFS_ERR_IO;
            }
            file_ref.ctz.head = file_ref.block;
            file_ref.ctz.size = file_ref.pos;
            file_ref.flags &= !(LFS_F_WRITING as u32);
            file_ref.flags |= LFS_F_DIRTY as u32;
            file_ref.pos = pos;
        }
    }
    0
}

/// Per lfs.c lfs_file_sync_ (lines 3431-3490)
///
/// C:
/// ```c
/// static int lfs_file_sync_(lfs_t *lfs, lfs_file_t *file) {
///     if (file->flags & LFS_F_ERRED) {
///         // it's not safe to do anything if our file errored
///         return 0;
///     }
///
///     int err = lfs_file_flush(lfs, file);
///     if (err) {
///         file->flags |= LFS_F_ERRED;
///         return err;
///     }
///
///     if ((file->flags & LFS_F_DIRTY) &&
///             !lfs_pair_isnull(file->m.pair)) {
///         // before we commit metadata, we need sync the disk to make sure
///         // data writes don't complete after metadata writes
///         if (!(file->flags & LFS_F_INLINE)) {
///             err = lfs_bd_sync(lfs, &lfs->pcache, &lfs->rcache, false);
///             if (err) {
///                 return err;
///             }
///         }
///
///         // update dir entry
///         uint16_t type;
///         const void *buffer;
///         lfs_size_t size;
///         struct lfs_ctz ctz;
///         if (file->flags & LFS_F_INLINE) {
///             // inline the whole file
///             type = LFS_TYPE_INLINESTRUCT;
///             buffer = file->cache.buffer;
///             size = file->ctz.size;
///         } else {
///             // update the ctz reference
///             type = LFS_TYPE_CTZSTRUCT;
///             // copy ctz so alloc will work during a relocate
///             ctz = file->ctz;
///             lfs_ctz_tole32(&ctz);
///             buffer = &ctz;
///             size = sizeof(ctz);
///         }
///
///         // commit file data and attributes
///         err = lfs_dir_commit(lfs, &file->m, LFS_MKATTRS(
///                 {LFS_MKTAG(type, file->id, size), buffer},
///                 {LFS_MKTAG(LFS_FROM_USERATTRS, file->id,
///                     file->cfg->attr_count), file->cfg->attrs}));
///         if (err) {
///             file->flags |= LFS_F_ERRED;
///             return err;
///         }
///
///         file->flags &= ~LFS_F_DIRTY;
///     }
///
///     return 0;
/// }
/// ```
pub fn lfs_file_sync_(lfs: *mut crate::fs::Lfs, file: *mut LfsFile) -> i32 {
    use crate::dir::commit::lfs_dir_commit;
    use crate::fs::superblock::lfs_fs_deorphan;
    use crate::tag::lfs_mktag;
    use crate::types::LFS_BLOCK_INLINE;
    use crate::util::lfs_pair_isnull;

    unsafe {
        let file_ref = &mut *file;
        if (file_ref.flags as i32 & 0x080000) != 0 {
            return 0;
        }

        let err = lfs_file_flush(lfs as *const core::ffi::c_void, file);
        if err != 0 {
            file_ref.flags |= 0x080000;
            return err;
        }

        if (file_ref.flags as i32 & 0x010000) != 0 && !lfs_pair_isnull(&file_ref.m.pair) {
            let lfs_ref = &*lfs;
            let cfg = lfs_ref.cfg.as_ref().expect("cfg");

            if (file_ref.flags as i32 & LFS_F_INLINE) == 0 {
                let err =
                    crate::bd::bd::lfs_bd_sync(lfs, &mut (*lfs).pcache, &mut (*lfs).rcache, false);
                if err != 0 {
                    return err;
                }
            }

            let (type_, buffer, size) = if (file_ref.flags as i32 & LFS_F_INLINE) != 0 {
                (
                    LFS_TYPE_INLINESTRUCT,
                    file_ref.cache.buffer as *const core::ffi::c_void,
                    file_ref.ctz.size,
                )
            } else {
                let mut ctz = file_ref.ctz;
                crate::file::lfs_ctz::lfs_ctz_tole32(&mut ctz);
                (
                    crate::lfs_type::lfs_type::LFS_TYPE_CTZSTRUCT,
                    &ctz as *const _ as *const core::ffi::c_void,
                    core::mem::size_of::<crate::file::LfsCtz>() as u32,
                )
            };

            let attrs = [
                crate::tag::lfs_mattr {
                    tag: lfs_mktag(type_, file_ref.id as u32, size),
                    buffer,
                },
                crate::tag::lfs_mattr {
                    tag: lfs_mktag(
                        crate::lfs_type::lfs_type::LFS_FROM_USERATTRS,
                        file_ref.id as u32,
                        file_ref.cfg.as_ref().map_or(0, |c| c.attr_count),
                    ) as u32,
                    buffer: file_ref.cfg.as_ref().map_or(core::ptr::null(), |c| c.attrs)
                        as *const core::ffi::c_void,
                },
            ];
            let err = lfs_dir_commit(lfs, &mut file_ref.m, attrs.as_ptr() as *const _, 2);
            if err != 0 {
                file_ref.flags |= 0x080000;
                return err;
            }
            file_ref.flags &= !0x010000;
        }
    }
    0
}

/// Per lfs.c lfs_file_flushedread (lines 3492-3551)
///
/// Translation docs: Read file data. Handles inline and CTZ files.
/// Uses file cache for block caching; dir_getread for inline, bd_read for CTZ.
///
/// C: lfs.c:3493-3551
pub fn lfs_file_flushedread(
    lfs: *mut crate::fs::Lfs,
    file: *mut LfsFile,
    buffer: *mut core::ffi::c_void,
    size: lfs_size_t,
) -> crate::types::lfs_ssize_t {
    if buffer.is_null() {
        return 0;
    }
    let data = buffer as *mut u8;

    unsafe {
        let file_ref = &mut *file;
        let lfs_ref = &*lfs;
        let cfg = lfs_ref.cfg.as_ref().expect("cfg");
        let block_size = cfg.block_size;

        if file_ref.pos >= file_ref.ctz.size {
            return 0;
        }

        let size = lfs_min(size, file_ref.ctz.size - file_ref.pos);
        let mut nsize = size;

        let mut data = data;
        while nsize > 0 {
            if (file_ref.flags as i32 & LFS_F_READING) == 0 || file_ref.off == block_size {
                if (file_ref.flags as i32 & LFS_F_INLINE) == 0 {
                    let err = lfs_ctz_find(
                        lfs,
                        core::ptr::null(),
                        &mut file_ref.cache,
                        file_ref.ctz.head,
                        file_ref.ctz.size,
                        file_ref.pos,
                        &mut file_ref.block,
                        &mut file_ref.off,
                    );
                    if err != 0 {
                        return err as crate::types::lfs_ssize_t;
                    }
                } else {
                    file_ref.block = LFS_BLOCK_INLINE;
                    file_ref.off = file_ref.pos;
                }
                file_ref.flags |= LFS_F_READING as u32;
            }

            let diff = lfs_min(nsize, block_size - file_ref.off);
            if (file_ref.flags as i32 & LFS_F_INLINE) != 0 {
                let gtag = lfs_mktag(LFS_TYPE_INLINESTRUCT, file_ref.id as u32, 0);
                let err = lfs_dir_getread(
                    lfs,
                    &file_ref.m,
                    core::ptr::null(),
                    &mut file_ref.cache,
                    block_size,
                    lfs_mktag(0xfff, 0x1ff, 0),
                    gtag,
                    file_ref.off,
                    data as *mut core::ffi::c_void,
                    diff,
                );
                if err != 0 {
                    return err as crate::types::lfs_ssize_t;
                }
            } else {
                let err = lfs_bd_read(
                    lfs,
                    core::ptr::null(),
                    &mut file_ref.cache,
                    block_size,
                    file_ref.block,
                    file_ref.off,
                    data,
                    diff,
                );
                if err != 0 {
                    return err as crate::types::lfs_ssize_t;
                }
            }

            file_ref.pos += diff;
            file_ref.off += diff;
            data = data.add(diff as usize);
            nsize -= diff;
        }

        size as crate::types::lfs_ssize_t
    }
}

/// Per lfs.c lfs_file_read_ (lines 3553-3570)
///
/// Translation docs: Read file. Asserts RDONLY; flushes pending writes if any; delegates to flushedread.
///
/// C: lfs.c:3553-3570
pub fn lfs_file_read_(
    lfs: *mut crate::fs::Lfs,
    file: *mut LfsFile,
    buffer: *mut core::ffi::c_void,
    size: lfs_size_t,
) -> crate::types::lfs_ssize_t {
    crate::lfs_assert!((unsafe { (*file).flags as i32 } & LFS_O_RDONLY) == LFS_O_RDONLY);

    unsafe {
        if ((*file).flags as i32 & LFS_F_WRITING) != 0 {
            let err = lfs_file_flush(lfs as *const core::ffi::c_void, file);
            if err != 0 {
                return err as crate::types::lfs_ssize_t;
            }
        }
    }

    lfs_file_flushedread(lfs, file, buffer, size)
}

/// Per lfs.c lfs_file_flushedwrite (lines 3572-3654)
///
/// C:
/// ```c
/// static lfs_ssize_t lfs_file_flushedwrite(lfs_t *lfs, lfs_file_t *file,
///         const void *buffer, lfs_size_t size) {
///     const uint8_t *data = buffer;
///     lfs_size_t nsize = size;
///
///     if ((file->flags & LFS_F_INLINE) &&
///             lfs_max(file->pos+nsize, file->ctz.size) > lfs->inline_max) {
///         // inline file doesn't fit anymore
///         int err = lfs_file_outline(lfs, file);
///         if (err) {
///             file->flags |= LFS_F_ERRED;
///             return err;
///         }
///     }
///
///     while (nsize > 0) {
///         // check if we need a new block
///         if (!(file->flags & LFS_F_WRITING) ||
///                 file->off == lfs->cfg->block_size) {
///             if (!(file->flags & LFS_F_INLINE)) {
///                 if (!(file->flags & LFS_F_WRITING) && file->pos > 0) {
///                     // find out which block we're extending from
///                     int err = lfs_ctz_find(lfs, NULL, &file->cache,
///                             file->ctz.head, file->ctz.size,
///                             file->pos-1, &file->block, &(lfs_off_t){0});
///                     if (err) {
///                         file->flags |= LFS_F_ERRED;
///                         return err;
///                     }
///
///                     // mark cache as dirty since we may have read data into it
///                     lfs_cache_zero(lfs, &file->cache);
///                 }
///
///                 // extend file with new blocks
///                 lfs_alloc_ckpoint(lfs);
///                 int err = lfs_ctz_extend(lfs, &file->cache, &lfs->rcache,
///                         file->block, file->pos,
///                         &file->block, &file->off);
///                 if (err) {
///                     file->flags |= LFS_F_ERRED;
///                     return err;
///                 }
///             } else {
///                 file->block = LFS_BLOCK_INLINE;
///                 file->off = file->pos;
///             }
///
///             file->flags |= LFS_F_WRITING;
///         }
///
///         // program as much as we can in current block
///         lfs_size_t diff = lfs_min(nsize, lfs->cfg->block_size - file->off);
///         while (true) {
///             int err = lfs_bd_prog(lfs, &file->cache, &lfs->rcache, true,
///                     file->block, file->off, data, diff);
///             if (err) {
///                 if (err == LFS_ERR_CORRUPT) {
///                     goto relocate;
///                 }
///                 file->flags |= LFS_F_ERRED;
///                 return err;
///             }
///
///             break;
/// relocate:
///             err = lfs_file_relocate(lfs, file);
///             if (err) {
///                 file->flags |= LFS_F_ERRED;
///                 return err;
///             }
///         }
///
///         file->pos += diff;
///         file->off += diff;
///         data += diff;
///         nsize -= diff;
///
///         lfs_alloc_ckpoint(lfs);
///     }
///
///     return size;
/// }
/// ```
pub fn lfs_file_flushedwrite(
    lfs: *mut crate::fs::Lfs,
    file: *mut LfsFile,
    buffer: *const core::ffi::c_void,
    size: lfs_size_t,
) -> crate::types::lfs_ssize_t {
    use crate::bd::bd::lfs_bd_prog;

    if buffer.is_null() {
        return 0;
    }
    let data = buffer as *const u8;

    unsafe {
        let file_ref = &mut *file;
        let lfs_ref = &*lfs;
        let cfg = lfs_ref.cfg.as_ref().expect("cfg");
        let block_size = cfg.block_size;
        let mut nsize = size;

        if (file_ref.flags as i32 & LFS_F_INLINE) != 0
            && crate::util::lfs_max(file_ref.pos + nsize, file_ref.ctz.size) > lfs_ref.inline_max
        {
            return crate::error::LFS_ERR_IO as crate::types::lfs_ssize_t;
        }

        let mut data = data;
        while nsize > 0 {
            if (file_ref.flags as i32 & LFS_F_WRITING) == 0 || file_ref.off == block_size {
                if (file_ref.flags as i32 & LFS_F_INLINE) != 0 {
                    file_ref.block = LFS_BLOCK_INLINE;
                    file_ref.off = file_ref.pos;
                } else {
                    return crate::error::LFS_ERR_IO as crate::types::lfs_ssize_t;
                }
                file_ref.flags |= LFS_F_WRITING as u32;
            }

            let diff = lfs_min(nsize, block_size - file_ref.off);
            let err = lfs_bd_prog(
                lfs,
                &mut file_ref.cache,
                &mut (*lfs).rcache,
                true,
                file_ref.block,
                file_ref.off,
                data,
                diff,
            );
            if err != 0 {
                file_ref.flags |= 0x080000;
                return err as crate::types::lfs_ssize_t;
            }

            file_ref.pos += diff;
            file_ref.off += diff;
            data = data.add(diff as usize);
            nsize -= diff;

            crate::block_alloc::alloc::lfs_alloc_ckpoint(lfs);
        }
        size as crate::types::lfs_ssize_t
    }
}

/// Per lfs.c lfs_file_write_ (lines 3656-3698)
///
/// C:
/// ```c
/// static lfs_ssize_t lfs_file_write_(lfs_t *lfs, lfs_file_t *file,
///         const void *buffer, lfs_size_t size) {
///     LFS_ASSERT((file->flags & LFS_O_WRONLY) == LFS_O_WRONLY);
///
///     if (file->flags & LFS_F_READING) {
///         // drop any reads
///         int err = lfs_file_flush(lfs, file);
///         if (err) {
///             return err;
///         }
///     }
///
///     if ((file->flags & LFS_O_APPEND) && file->pos < file->ctz.size) {
///         file->pos = file->ctz.size;
///     }
///
///     if (file->pos + size > lfs->file_max) {
///         // Larger than file limit?
///         return LFS_ERR_FBIG;
///     }
///
///     if (!(file->flags & LFS_F_WRITING) && file->pos > file->ctz.size) {
///         // fill with zeros
///         lfs_off_t pos = file->pos;
///         file->pos = file->ctz.size;
///
///         while (file->pos < pos) {
///             lfs_ssize_t res = lfs_file_flushedwrite(lfs, file, &(uint8_t){0}, 1);
///             if (res < 0) {
///                 return res;
///             }
///         }
///     }
///
///     lfs_ssize_t nsize = lfs_file_flushedwrite(lfs, file, buffer, size);
///     if (nsize < 0) {
///         return nsize;
///     }
///
///     file->flags &= ~LFS_F_ERRED;
///     return nsize;
/// }
/// ```
pub fn lfs_file_write_(
    lfs: *mut crate::fs::Lfs,
    file: *mut LfsFile,
    buffer: *const core::ffi::c_void,
    size: lfs_size_t,
) -> crate::types::lfs_ssize_t {
    crate::lfs_assert!((unsafe { (*file).flags as i32 } & 2) == 2);

    unsafe {
        if ((*file).flags as i32 & LFS_F_READING) != 0 {
            let err = lfs_file_flush(lfs as *const core::ffi::c_void, file);
            if err != 0 {
                return err as crate::types::lfs_ssize_t;
            }
        }
        if ((*file).flags as i32 & 0x0800) != 0 && (*file).pos < (*file).ctz.size {
            (*file).pos = (*file).ctz.size;
        }
        if (*file).pos + size > (*lfs).file_max {
            return crate::error::LFS_ERR_FBIG as crate::types::lfs_ssize_t;
        }
        let nsize = lfs_file_flushedwrite(lfs, file, buffer, size);
        if nsize >= 0 {
            (*file).flags &= !0x080000;
        }
        nsize
    }
}

/// Per lfs.c lfs_file_seek_ (lines 3700-3751)
///
/// Translation docs: Seek to new position. SEEK_SET, SEEK_CUR, SEEK_END.
/// May avoid flush if new pos is in current cache (reading path).
///
/// C: lfs.c:3700-3751
pub fn lfs_file_seek_(
    lfs: *mut crate::fs::Lfs,
    file: *mut LfsFile,
    off: crate::types::lfs_soff_t,
    whence: i32,
) -> crate::types::lfs_soff_t {
    use crate::error::LFS_ERR_INVAL;
    use crate::file::ctz::lfs_ctz_index;
    use crate::lfs_type::lfs_whence_flags::{LFS_SEEK_CUR, LFS_SEEK_END, LFS_SEEK_SET};

    unsafe {
        let lfs_ref = &*lfs;
        let file_ref = &mut *file;
        let file_max = lfs_ref.file_max;
        let block_size = lfs_ref.cfg.as_ref().expect("cfg").block_size;

        let mut npos = file_ref.pos;
        if whence == LFS_SEEK_SET {
            npos = off as lfs_off_t;
        } else if whence == LFS_SEEK_CUR {
            npos = (file_ref.pos as i64 + off as i64) as lfs_off_t;
        } else if whence == LFS_SEEK_END {
            npos = (lfs_file_size_(lfs as *const core::ffi::c_void, file) as i64 + off as i64)
                as lfs_off_t;
        }

        if npos > file_max {
            return LFS_ERR_INVAL as crate::types::lfs_soff_t;
        }

        if file_ref.pos == npos {
            return npos as crate::types::lfs_soff_t;
        }

        if (file_ref.flags as i32 & LFS_F_READING) != 0 && file_ref.off != block_size {
            let mut opos = file_ref.pos;
            let mut npos_off = npos;
            let oindex = lfs_ctz_index(lfs as *const crate::fs::Lfs, &mut opos);
            let nindex = lfs_ctz_index(lfs as *const crate::fs::Lfs, &mut npos_off);
            if oindex == nindex
                && npos_off >= file_ref.cache.off
                && npos_off < file_ref.cache.off + file_ref.cache.size
            {
                file_ref.pos = npos;
                file_ref.off = npos_off;
                return npos as crate::types::lfs_soff_t;
            }
        }

        let err = lfs_file_flush(lfs as *const core::ffi::c_void, file);
        if err != 0 {
            return err as crate::types::lfs_soff_t;
        }

        (*file).pos = npos;
        npos as crate::types::lfs_soff_t
    }
}

/// Per lfs.c lfs_file_truncate_ (lines 3753-3838)
///
/// C:
/// ```c
/// static int lfs_file_truncate_(lfs_t *lfs, lfs_file_t *file, lfs_off_t size) {
///     LFS_ASSERT((file->flags & LFS_O_WRONLY) == LFS_O_WRONLY);
///
///     if (size > LFS_FILE_MAX) {
///         return LFS_ERR_INVAL;
///     }
///
///     lfs_off_t pos = file->pos;
///     lfs_off_t oldsize = lfs_file_size_(lfs, file);
///     if (size < oldsize) {
///         // revert to inline file?
///         if (size <= lfs->inline_max) {
///             // flush+seek to head
///             lfs_soff_t res = lfs_file_seek_(lfs, file, 0, LFS_SEEK_SET);
///             if (res < 0) {
///                 return (int)res;
///             }
///
///             // read our data into rcache temporarily
///             lfs_cache_drop(lfs, &lfs->rcache);
///             res = lfs_file_flushedread(lfs, file,
///                     lfs->rcache.buffer, size);
///             if (res < 0) {
///                 return (int)res;
///             }
///
///             file->ctz.head = LFS_BLOCK_INLINE;
///             file->ctz.size = size;
///             file->flags |= LFS_F_DIRTY | LFS_F_READING | LFS_F_INLINE;
///             file->cache.block = file->ctz.head;
///             file->cache.off = 0;
///             file->cache.size = lfs->cfg->cache_size;
///             memcpy(file->cache.buffer, lfs->rcache.buffer, size);
///
///         } else {
///             // need to flush since directly changing metadata
///             int err = lfs_file_flush(lfs, file);
///             if (err) {
///                 return err;
///             }
///
///             // lookup new head in ctz skip list
///             err = lfs_ctz_find(lfs, NULL, &file->cache,
///                     file->ctz.head, file->ctz.size,
///                     size-1, &file->block, &(lfs_off_t){0});
///             if (err) {
///                 return err;
///             }
///
///             // need to set pos/block/off consistently so seeking back to
///             // the old position does not get confused
///             file->pos = size;
///             file->ctz.head = file->block;
///             file->ctz.size = size;
///             file->flags |= LFS_F_DIRTY | LFS_F_READING;
///         }
///     } else if (size > oldsize) {
///         // flush+seek if not already at end
///         lfs_soff_t res = lfs_file_seek_(lfs, file, 0, LFS_SEEK_END);
///         if (res < 0) {
///             return (int)res;
///         }
///
///         // fill with zeros
///         while (file->pos < size) {
///             res = lfs_file_write_(lfs, file, &(uint8_t){0}, 1);
///             if (res < 0) {
///                 return (int)res;
///             }
///         }
///     }
///
///     // restore pos
///     lfs_soff_t res = lfs_file_seek_(lfs, file, pos, LFS_SEEK_SET);
///     if (res < 0) {
///       return (int)res;
///     }
///
///     return 0;
/// }
/// #endif
/// ```
pub fn lfs_file_truncate_(
    _lfs: *const core::ffi::c_void,
    _file: *mut LfsFile,
    _size: lfs_off_t,
) -> i32 {
    todo!("lfs_file_truncate_")
}

/// Per lfs.c lfs_file_tell_ (lines 3835-3838)
///
/// Translation docs: Returns the current file position.
///
/// C:
/// ```c
/// static lfs_soff_t lfs_file_tell_(lfs_t *lfs, lfs_file_t *file) {
///     (void)lfs;
///     return file->pos;
/// }
/// ```
pub fn lfs_file_tell_(
    _lfs: *const core::ffi::c_void,
    file: *const LfsFile,
) -> crate::types::lfs_soff_t {
    unsafe { (*file).pos as crate::types::lfs_soff_t }
}

/// Per lfs.c lfs_file_rewind_ (lines 3840-3850)
///
/// Translation docs: Seek to start of file.
///
/// C: lfs.c:3840-3850
pub fn lfs_file_rewind_(lfs: *mut crate::fs::Lfs, file: *mut LfsFile) -> i32 {
    let res = lfs_file_seek_(
        lfs,
        file,
        0,
        crate::lfs_type::lfs_whence_flags::LFS_SEEK_SET,
    );
    if res < 0 {
        return res as i32;
    }
    0
}

/// Per lfs.c lfs_file_size_ (lines 3849-3851)
///
/// Translation docs: Returns the file size in bytes.
///
/// C:
/// ```c
/// static lfs_soff_t lfs_file_size_(lfs_t *lfs, lfs_file_t *file) {
///     (void)lfs;
///     return file->ctz.size;
/// }
/// ```
pub fn lfs_file_size_(
    _lfs: *const core::ffi::c_void,
    file: *const LfsFile,
) -> crate::types::lfs_soff_t {
    unsafe { (*file).ctz.size as crate::types::lfs_soff_t }
}
