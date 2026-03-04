//! attr. Per lfs.c attr_.

/// Per lfs.c lfs_getattr_ (lines 4107-4135)
///
/// C:
/// ```c
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
/// ```
pub fn lfs_getattr_(
    _lfs: *mut super::lfs::Lfs,
    _path: *const i8,
    _type: u8,
    _buffer: *mut core::ffi::c_void,
    _size: crate::types::lfs_size_t,
) -> crate::types::lfs_ssize_t {
    todo!("lfs_getattr_")
}

/// Per lfs.c lfs_commitattr (lines 4141-4163)
///
/// C:
/// ```c
/// static int lfs_commitattr(lfs_t *lfs, const char *path,
///         uint8_t type, const void *buffer, lfs_size_t size) {
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
///     return lfs_dir_commit(lfs, &cwd, LFS_MKATTRS(
///             {LFS_MKTAG(LFS_TYPE_USERATTR + type, id, size), buffer}));
/// }
/// #endif
/// ```
pub fn lfs_commitattr(
    _lfs: *mut super::lfs::Lfs,
    _path: *const i8,
    _type: u8,
    _buffer: *const core::ffi::c_void,
    _size: crate::types::lfs_size_t,
) -> i32 {
    todo!("lfs_commitattr")
}

/// Per lfs.c lfs_setattr_ (lines 4165-4174)
///
/// C:
/// ```c
/// static int lfs_setattr_(lfs_t *lfs, const char *path,
///         uint8_t type, const void *buffer, lfs_size_t size) {
///     if (size > lfs->attr_max) {
///         return LFS_ERR_NOSPC;
///     }
///
///     return lfs_commitattr(lfs, path, type, buffer, size);
/// }
/// #endif
/// ```
pub fn lfs_setattr_(
    _lfs: *mut super::lfs::Lfs,
    _path: *const i8,
    _type: u8,
    _buffer: *const core::ffi::c_void,
    _size: crate::types::lfs_size_t,
) -> i32 {
    todo!("lfs_setattr_")
}

/// Per lfs.c lfs_removeattr_ (lines 4176-4196)
///
/// C:
/// ```c
/// static int lfs_removeattr_(lfs_t *lfs, const char *path, uint8_t type) {
///     return lfs_commitattr(lfs, path, type, NULL, 0x3ff);
/// }
/// #endif
/// ```
pub fn lfs_removeattr_(_lfs: *mut super::lfs::Lfs, _path: *const i8, _type: u8) -> i32 {
    todo!("lfs_removeattr_")
}
