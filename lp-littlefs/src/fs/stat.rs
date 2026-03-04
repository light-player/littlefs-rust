//! Stat. Per lfs.c lfs_stat_, lfs_fs_stat_, lfs_fs_size_.

/// Per lfs.c lfs_stat_ (lines 3863-3878)
///
/// C:
/// ```c
/// static int lfs_stat_(lfs_t *lfs, const char *path, struct lfs_info *info) {
///     lfs_mdir_t cwd;
///     lfs_stag_t tag = lfs_dir_find(lfs, &cwd, &path, NULL);
///     if (tag < 0) {
///         return (int)tag;
///     }
///
///     // only allow trailing slashes on dirs
///     if (strchr(path, '/') != NULL
///             && lfs_tag_type3(tag) != LFS_TYPE_DIR) {
///         return LFS_ERR_NOTDIR;
///     }
///
///     return lfs_dir_getinfo(lfs, &cwd, lfs_tag_id(tag), info);
/// }
/// ```
pub fn lfs_stat_(
    _lfs: *mut super::lfs::Lfs,
    _path: *const i8,
    _info: *mut crate::lfs_info::LfsInfo,
) -> i32 {
    todo!("lfs_stat_")
}

/// Per lfs.c lfs_fs_stat_ (lines 4653-4691)
///
/// C:
/// ```c
/// static int lfs_fs_stat_(lfs_t *lfs, struct lfs_fsinfo *fsinfo) {
///     // if the superblock is up-to-date, we must be on the most recent
///     // minor version of littlefs
///     if (!lfs_gstate_needssuperblock(&lfs->gstate)) {
///         fsinfo->disk_version = lfs_fs_disk_version(lfs);
///
///     // otherwise we need to read the minor version on disk
///     } else {
///         // fetch the superblock
///         lfs_mdir_t dir;
///         int err = lfs_dir_fetch(lfs, &dir, lfs->root);
///         if (err) {
///             return err;
///         }
///
///         lfs_superblock_t superblock;
///         lfs_stag_t tag = lfs_dir_get(lfs, &dir, LFS_MKTAG(0x7ff, 0x3ff, 0),
///                 LFS_MKTAG(LFS_TYPE_INLINESTRUCT, 0, sizeof(superblock)),
///                 &superblock);
///         if (tag < 0) {
///             return tag;
///         }
///         lfs_superblock_fromle32(&superblock);
///
///         // read the on-disk version
///         fsinfo->disk_version = superblock.version;
///     }
///
///     // filesystem geometry
///     fsinfo->block_size = lfs->cfg->block_size;
///     fsinfo->block_count = lfs->block_count;
///
///     // other on-disk configuration, we cache all of these for internal use
///     fsinfo->name_max = lfs->name_max;
///     fsinfo->file_max = lfs->file_max;
///     fsinfo->attr_max = lfs->attr_max;
///
///     return 0;
/// }
/// ```
pub fn lfs_fs_stat_(lfs: *mut super::lfs::Lfs, fsinfo: *mut crate::lfs_info::LfsFsinfo) -> i32 {
    use crate::dir::fetch::lfs_dir_fetch;
    use crate::dir::traverse::lfs_dir_get;
    use crate::lfs_gstate::lfs_gstate_needssuperblock;
    use crate::lfs_superblock::{lfs_superblock_fromle32, LfsSuperblock};
    use crate::lfs_type::lfs_type::LFS_TYPE_INLINESTRUCT;
    use crate::tag::lfs_mktag;
    use crate::types::LFS_DISK_VERSION;

    unsafe {
        let lfs_ref = &*lfs;
        let fsinfo = &mut *fsinfo;

        if !lfs_gstate_needssuperblock(&lfs_ref.gstate) {
            fsinfo.disk_version = LFS_DISK_VERSION;
        } else {
            let mut dir = crate::dir::LfsMdir {
                pair: [0, 0],
                rev: 0,
                off: 0,
                etag: 0,
                count: 0,
                erased: false,
                split: false,
                tail: [0, 0],
            };
            let err = lfs_dir_fetch(lfs, &mut dir, &lfs_ref.root);
            if err != 0 {
                return err;
            }
            let mut superblock = core::mem::zeroed::<LfsSuperblock>();
            let tag = lfs_dir_get(
                lfs,
                &dir as *const _,
                lfs_mktag(0x7ff, 0x3ff, 0),
                lfs_mktag(
                    LFS_TYPE_INLINESTRUCT,
                    0,
                    core::mem::size_of::<LfsSuperblock>() as u32,
                ),
                &mut superblock as *mut _ as *mut core::ffi::c_void,
            );
            if tag < 0 {
                return tag;
            }
            lfs_superblock_fromle32(&mut superblock);
            fsinfo.disk_version = superblock.version;
        }

        fsinfo.block_size = (*lfs_ref.cfg).block_size;
        fsinfo.block_count = lfs_ref.block_count;
        fsinfo.name_max = lfs_ref.name_max;
        fsinfo.file_max = lfs_ref.file_max;
        fsinfo.attr_max = lfs_ref.attr_max;
    }
    0
}

/// Per lfs.c lfs_fs_size_count (lines 5172-5177)
///
/// C:
/// ```c
/// static int lfs_fs_size_count(void *p, lfs_block_t block) {
///     (void)block;
///     lfs_size_t *size = p;
///     *size += 1;
///     return 0;
/// }
/// ```
pub fn lfs_fs_size_count(_p: *mut core::ffi::c_void, _block: crate::types::lfs_block_t) -> i32 {
    todo!("lfs_fs_size_count")
}

/// Per lfs.c lfs_fs_size_ (lines 5179-5188)
///
/// C:
/// ```c
/// static lfs_ssize_t lfs_fs_size_(lfs_t *lfs) {
///     lfs_size_t size = 0;
///     int err = lfs_fs_traverse_(lfs, lfs_fs_size_count, &size, false);
///     if (err) {
///         return err;
///     }
///
///     return size;
/// }
/// ```
pub fn lfs_fs_size_(_lfs: *mut super::lfs::Lfs) -> crate::types::lfs_ssize_t {
    todo!("lfs_fs_size_")
}
