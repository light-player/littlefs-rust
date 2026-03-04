//! mkdir. Per lfs.c mkdir_.
//
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
pub fn lfs_mkdir_(_lfs: *mut super::lfs::Lfs, _path: *const i8) -> i32 {
    todo!("lfs_mkdir_")
}
