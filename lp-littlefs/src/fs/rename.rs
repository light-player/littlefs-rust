//! rename. Per lfs.c rename_.
//
/// Per lfs.c lfs_rename_ (lines 3961-4138)
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

pub fn lfs_rename_(_lfs: *mut super::lfs::Lfs, _oldpath: *const i8, _newpath: *const i8) -> i32 {
    todo!("lfs_rename_")
}
