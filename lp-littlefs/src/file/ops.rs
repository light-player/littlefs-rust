//! File operations. Per lfs.c lfs_file_opencfg_, lfs_file_close_, lfs_file_sync_, etc.

use crate::dir::LfsMdir;
use crate::file::LfsFile;
use crate::lfs_info::LfsFileConfig;
use crate::types::{lfs_off_t, lfs_size_t};

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
    _lfs: *const core::ffi::c_void,
    _file: *mut LfsFile,
    _path: *const i8,
    _flags: i32,
    _cfg: *const LfsFileConfig,
) -> i32 {
    todo!("lfs_file_opencfg_")
}

/// Per lfs.c lfs_file_open_ (lines 3238-3244)
///
/// C:
/// ```c
/// static int lfs_file_open_(lfs_t *lfs, lfs_file_t *file,
///         const char *path, int flags) {
///     static const struct lfs_file_config defaults = {0};
///     return lfs_file_opencfg_(lfs, file, path, flags, &defaults);
/// }
/// ```
pub fn lfs_file_open_(
    _lfs: *const core::ffi::c_void,
    _file: *mut LfsFile,
    _path: *const i8,
    _flags: i32,
) -> i32 {
    todo!("lfs_file_open_")
}

/// Per lfs.c lfs_file_close_ (lines 3246-3264)
///
/// C:
/// ```c
/// static int lfs_file_close_(lfs_t *lfs, lfs_file_t *file) {
/// #ifndef LFS_READONLY
///     int err = lfs_file_sync_(lfs, file);
/// #else
///     int err = 0;
/// #endif
///
///     // remove from list of mdirs
///     lfs_mlist_remove(lfs, (struct lfs_mlist*)file);
///
///     // clean up memory
///     if (!file->cfg->buffer) {
///         lfs_free(file->cache.buffer);
///     }
///
///     return err;
/// }
/// ```
pub fn lfs_file_close_(_lfs: *const core::ffi::c_void, _file: *mut LfsFile) -> i32 {
    todo!("lfs_file_close_")
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
pub fn lfs_file_flush(_lfs: *const core::ffi::c_void, _file: *mut LfsFile) -> i32 {
    todo!("lfs_file_flush")
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
pub fn lfs_file_sync_(_lfs: *const core::ffi::c_void, _file: *mut LfsFile) -> i32 {
    todo!("lfs_file_sync_")
}

/// Per lfs.c lfs_file_flushedread (lines 3492-3551)
///
/// C:
/// ```c
/// static lfs_ssize_t lfs_file_flushedread(lfs_t *lfs, lfs_file_t *file,
///         void *buffer, lfs_size_t size) {
///     uint8_t *data = buffer;
///     lfs_size_t nsize = size;
///
///     if (file->pos >= file->ctz.size) {
///         // eof if past end
///         return 0;
///     }
///
///     size = lfs_min(size, file->ctz.size - file->pos);
///     nsize = size;
///
///     while (nsize > 0) {
///         // check if we need a new block
///         if (!(file->flags & LFS_F_READING) ||
///                 file->off == lfs->cfg->block_size) {
///             if (!(file->flags & LFS_F_INLINE)) {
///                 int err = lfs_ctz_find(lfs, NULL, &file->cache,
///                         file->ctz.head, file->ctz.size,
///                         file->pos, &file->block, &file->off);
///                 if (err) {
///                     return err;
///                 }
///             } else {
///                 file->block = LFS_BLOCK_INLINE;
///                 file->off = file->pos;
///             }
///
///             file->flags |= LFS_F_READING;
///         }
///
///         // read as much as we can in current block
///         lfs_size_t diff = lfs_min(nsize, lfs->cfg->block_size - file->off);
///         if (file->flags & LFS_F_INLINE) {
///             int err = lfs_dir_getread(lfs, &file->m,
///                     NULL, &file->cache, lfs->cfg->block_size,
///                     LFS_MKTAG(0xfff, 0x1ff, 0),
///                     LFS_MKTAG(LFS_TYPE_INLINESTRUCT, file->id, 0),
///                     file->off, data, diff);
///             if (err) {
///                 return err;
///             }
///         } else {
///             int err = lfs_bd_read(lfs,
///                     NULL, &file->cache, lfs->cfg->block_size,
///                     file->block, file->off, data, diff);
///             if (err) {
///                 return err;
///             }
///         }
///
///         file->pos += diff;
///         file->off += diff;
///         data += diff;
///         nsize -= diff;
///     }
///
///     return size;
/// }
/// ```
pub fn lfs_file_flushedread(
    _lfs: *const core::ffi::c_void,
    _file: *mut LfsFile,
    _buffer: *mut core::ffi::c_void,
    _size: lfs_size_t,
) -> crate::types::lfs_ssize_t {
    todo!("lfs_file_flushedread")
}

/// Per lfs.c lfs_file_read_ (lines 3553-3570)
///
/// C:
/// ```c
/// static lfs_ssize_t lfs_file_read_(lfs_t *lfs, lfs_file_t *file,
///         void *buffer, lfs_size_t size) {
///     LFS_ASSERT((file->flags & LFS_O_RDONLY) == LFS_O_RDONLY);
///
/// #ifndef LFS_READONLY
///     if (file->flags & LFS_F_WRITING) {
///         // flush out any writes
///         int err = lfs_file_flush(lfs, file);
///         if (err) {
///             return err;
///         }
///     }
/// #endif
///
///     return lfs_file_flushedread(lfs, file, buffer, size);
/// }
/// ```
pub fn lfs_file_read_(
    _lfs: *const core::ffi::c_void,
    _file: *mut LfsFile,
    _buffer: *mut core::ffi::c_void,
    _size: lfs_size_t,
) -> crate::types::lfs_ssize_t {
    todo!("lfs_file_read_")
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
    _lfs: *const core::ffi::c_void,
    _file: *mut LfsFile,
    _buffer: *const core::ffi::c_void,
    _size: lfs_size_t,
) -> crate::types::lfs_ssize_t {
    todo!("lfs_file_flushedwrite")
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
    _lfs: *const core::ffi::c_void,
    _file: *mut LfsFile,
    _buffer: *const core::ffi::c_void,
    _size: lfs_size_t,
) -> crate::types::lfs_ssize_t {
    todo!("lfs_file_write_")
}

/// Per lfs.c lfs_file_seek_ (lines 3700-3751)
///
/// C:
/// ```c
/// static lfs_soff_t lfs_file_seek_(lfs_t *lfs, lfs_file_t *file,
///         lfs_soff_t off, int whence) {
///     // find new pos
///     //
///     // fortunately for us, littlefs is limited to 31-bit file sizes, so we
///     // don't have to worry too much about integer overflow
///     lfs_off_t npos = file->pos;
///     if (whence == LFS_SEEK_SET) {
///         npos = off;
///     } else if (whence == LFS_SEEK_CUR) {
///         npos = file->pos + (lfs_off_t)off;
///     } else if (whence == LFS_SEEK_END) {
///         npos = (lfs_off_t)lfs_file_size_(lfs, file) + (lfs_off_t)off;
///     }
///
///     if (npos > lfs->file_max) {
///         // file position out of range
///         return LFS_ERR_INVAL;
///     }
///
///     if (file->pos == npos) {
///         // noop - position has not changed
///         return npos;
///     }
///
///     // if we're only reading and our new offset is still in the file's cache
///     // we can avoid flushing and needing to reread the data
///     if ((file->flags & LFS_F_READING)
///             && file->off != lfs->cfg->block_size) {
///         int oindex = lfs_ctz_index(lfs, &(lfs_off_t){file->pos});
///         lfs_off_t noff = npos;
///         int nindex = lfs_ctz_index(lfs, &noff);
///         if (oindex == nindex
///                 && noff >= file->cache.off
///                 && noff < file->cache.off + file->cache.size) {
///             file->pos = npos;
///             file->off = noff;
///             return npos;
///         }
///     }
///
///     // write out everything beforehand, may be noop if rdonly
///     int err = lfs_file_flush(lfs, file);
///     if (err) {
///         return err;
///     }
///
///     // update pos
///     file->pos = npos;
///     return npos;
/// }
/// ```
pub fn lfs_file_seek_(
    _lfs: *const core::ffi::c_void,
    _file: *mut LfsFile,
    _off: crate::types::lfs_soff_t,
    _whence: i32,
) -> crate::types::lfs_soff_t {
    todo!("lfs_file_seek_")
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
/// C:
/// ```c
/// static lfs_soff_t lfs_file_tell_(lfs_t *lfs, lfs_file_t *file) {
///     (void)lfs;
///     return file->pos;
/// }
/// ```
pub fn lfs_file_tell_(
    _lfs: *const core::ffi::c_void,
    _file: *const LfsFile,
) -> crate::types::lfs_soff_t {
    todo!("lfs_file_tell_")
}

/// Per lfs.c lfs_file_rewind_ (lines 3840-3850)
///
/// C:
/// ```c
/// static int lfs_file_rewind_(lfs_t *lfs, lfs_file_t *file) {
///     lfs_soff_t res = lfs_file_seek_(lfs, file, 0, LFS_SEEK_SET);
///     if (res < 0) {
///         return (int)res;
///     }
///     return 0;
/// }
/// ```
pub fn lfs_file_rewind_(_lfs: *const core::ffi::c_void, _file: *mut LfsFile) -> i32 {
    todo!("lfs_file_rewind_")
}

/// Per lfs.c lfs_file_size_ (lines 3849-3851)
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
    _file: *const LfsFile,
) -> crate::types::lfs_soff_t {
    todo!("lfs_file_size_")
}
