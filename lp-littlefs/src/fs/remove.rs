//! remove. Per lfs.c remove_.
//
/// Per lfs.c lfs_remove_ (lines 3880-3960)
///
/// C:
/// ```c
/// static int lfs_remove_(lfs_t *lfs, const char *path) {
///     // deorphan if we haven't yet, needed at most once after poweron
///     int err = lfs_fs_forceconsistency(lfs);
///     if (err) {
///         return err;
///     }
///
///     lfs_mdir_t cwd;
///     lfs_stag_t tag = lfs_dir_find(lfs, &cwd, &path, NULL);
///     if (tag < 0 || lfs_tag_id(tag) == 0x3ff) {
///         return (tag < 0) ? (int)tag : LFS_ERR_INVAL;
///     }
///
///     struct lfs_mlist dir;
///     dir.next = lfs->mlist;
///     if (lfs_tag_type3(tag) == LFS_TYPE_DIR) {
///         // must be empty before removal
///         lfs_block_t pair[2];
///         lfs_stag_t res = lfs_dir_get(lfs, &cwd, LFS_MKTAG(0x700, 0x3ff, 0),
///                 LFS_MKTAG(LFS_TYPE_STRUCT, lfs_tag_id(tag), 8), pair);
///         if (res < 0) {
///             return (int)res;
///         }
///         lfs_pair_fromle32(pair);
///
///         err = lfs_dir_fetch(lfs, &dir.m, pair);
///         if (err) {
///             return err;
///         }
///
///         if (dir.m.count > 0 || dir.m.split) {
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
///         dir.type = 0;
///         dir.id = 0;
///         lfs->mlist = &dir;
///     }
///
///     // delete the entry
///     err = lfs_dir_commit(lfs, &cwd, LFS_MKATTRS(
///             {LFS_MKTAG(LFS_TYPE_DELETE, lfs_tag_id(tag), 0), NULL}));
///     if (err) {
///         lfs->mlist = dir.next;
///         return err;
///     }
///
///     lfs->mlist = dir.next;
///     if (lfs_gstate_hasorphans(&lfs->gstate)) {
///         LFS_ASSERT(lfs_tag_type3(tag) == LFS_TYPE_DIR);
///
///         // fix orphan
///         err = lfs_fs_preporphans(lfs, -1);
///         if (err) {
///             return err;
///         }
///
///         err = lfs_fs_pred(lfs, dir.m.pair, &cwd);
///         if (err) {
///             return err;
///         }
///
///         err = lfs_dir_drop(lfs, &cwd, &dir.m);
///         if (err) {
///             return err;
///         }
///     }
///
///     return 0;
/// }
/// #endif
///
/// #ifndef LFS_READONLY
/// ```
pub fn lfs_remove_(_lfs: *mut super::lfs::Lfs, _path: *const i8) -> i32 {
    todo!("lfs_remove_")
}
