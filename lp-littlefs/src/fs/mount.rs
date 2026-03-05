//! Mount/unmount. Per lfs.c lfs_mount_, lfs_unmount_.

/// Per lfs.c lfs_tortoise_t and lfs_tortoise_detectcycles (lines 4464-4480)
#[repr(C)]
pub struct LfsTortoise {
    pub pair: [crate::types::lfs_block_t; 2],
    pub i: crate::types::lfs_size_t,
    pub period: crate::types::lfs_size_t,
}

/// Per lfs.c lfs_tortoise_detectcycles (lines 4464-4480)
pub fn lfs_tortoise_detectcycles(
    dir: *const crate::dir::LfsMdir,
    tortoise: *mut LfsTortoise,
) -> i32 {
    use crate::types::LFS_BLOCK_NULL;
    use crate::util::lfs_pair_issync;

    if tortoise.is_null() {
        return 0;
    }
    unsafe {
        let dir_ref = &*dir;
        let tortoise_ref = &mut *tortoise;
        if lfs_pair_issync(&dir_ref.tail, &tortoise_ref.pair) {
            return crate::error::LFS_ERR_CORRUPT;
        }
        if tortoise_ref.i == tortoise_ref.period {
            tortoise_ref.pair = dir_ref.tail;
            tortoise_ref.i = 0;
            tortoise_ref.period *= 2;
        }
        tortoise_ref.i += 1;
    }
    0
}

/// Per lfs.c lfs_mount_ (lines 4482-4645)
///
/// C:
/// ```c
/// static int lfs_mount_(lfs_t *lfs, const struct lfs_config *cfg) {
///     int err = lfs_init(lfs, cfg);
///     if (err) {
///         return err;
///     }
///
///     // scan directory blocks for superblock and any global updates
///     lfs_mdir_t dir = {.tail = {0, 1}};
///     struct lfs_tortoise_t tortoise = {
///         .pair = {LFS_BLOCK_NULL, LFS_BLOCK_NULL},
///         .i = 1,
///         .period = 1,
///     };
///     while (!lfs_pair_isnull(dir.tail)) {
///         err = lfs_tortoise_detectcycles(&dir, &tortoise);
///         if (err < 0) {
///             goto cleanup;
///         }
///
///         // fetch next block in tail list
///         lfs_stag_t tag = lfs_dir_fetchmatch(lfs, &dir, dir.tail,
///                 LFS_MKTAG(0x7ff, 0x3ff, 0),
///                 LFS_MKTAG(LFS_TYPE_SUPERBLOCK, 0, 8),
///                 NULL,
///                 lfs_dir_find_match, &(struct lfs_dir_find_match){
///                     lfs, "littlefs", 8});
///         if (tag < 0) {
///             err = tag;
///             goto cleanup;
///         }
///
///         // has superblock?
///         if (tag && !lfs_tag_isdelete(tag)) {
///             // update root
///             lfs->root[0] = dir.pair[0];
///             lfs->root[1] = dir.pair[1];
///
///             // grab superblock
///             lfs_superblock_t superblock;
///             tag = lfs_dir_get(lfs, &dir, LFS_MKTAG(0x7ff, 0x3ff, 0),
///                     LFS_MKTAG(LFS_TYPE_INLINESTRUCT, 0, sizeof(superblock)),
///                     &superblock);
///             if (tag < 0) {
///                 err = tag;
///                 goto cleanup;
///             }
///             lfs_superblock_fromle32(&superblock);
///
///             // check version
///             uint16_t major_version = (0xffff & (superblock.version >> 16));
///             uint16_t minor_version = (0xffff & (superblock.version >>  0));
///             if (major_version != lfs_fs_disk_version_major(lfs)
///                     || minor_version > lfs_fs_disk_version_minor(lfs)) {
///                 LFS_ERROR("Invalid version "
///                         "v%"PRIu16".%"PRIu16" != v%"PRIu16".%"PRIu16,
///                         major_version,
///                         minor_version,
///                         lfs_fs_disk_version_major(lfs),
///                         lfs_fs_disk_version_minor(lfs));
///                 err = LFS_ERR_INVAL;
///                 goto cleanup;
///             }
///
///             // found older minor version? set an in-device only bit in the
///             // gstate so we know we need to rewrite the superblock before
///             // the first write
///             bool needssuperblock = false;
///             if (minor_version < lfs_fs_disk_version_minor(lfs)) {
///                 LFS_DEBUG("Found older minor version "
///                         "v%"PRIu16".%"PRIu16" < v%"PRIu16".%"PRIu16,
///                         major_version,
///                         minor_version,
///                         lfs_fs_disk_version_major(lfs),
///                         lfs_fs_disk_version_minor(lfs));
///                 needssuperblock = true;
///             }
///             // note this bit is reserved on disk, so fetching more gstate
///             // will not interfere here
///             lfs_fs_prepsuperblock(lfs, needssuperblock);
///
///             // check superblock configuration
///             if (superblock.name_max) {
///                 if (superblock.name_max > lfs->name_max) {
///                     LFS_ERROR("Unsupported name_max (%"PRIu32" > %"PRIu32")",
///                             superblock.name_max, lfs->name_max);
///                     err = LFS_ERR_INVAL;
///                     goto cleanup;
///                 }
///
///                 lfs->name_max = superblock.name_max;
///             }
///
///             if (superblock.file_max) {
///                 if (superblock.file_max > lfs->file_max) {
///                     LFS_ERROR("Unsupported file_max (%"PRIu32" > %"PRIu32")",
///                             superblock.file_max, lfs->file_max);
///                     err = LFS_ERR_INVAL;
///                     goto cleanup;
///                 }
///
///                 lfs->file_max = superblock.file_max;
///             }
///
///             if (superblock.attr_max) {
///                 if (superblock.attr_max > lfs->attr_max) {
///                     LFS_ERROR("Unsupported attr_max (%"PRIu32" > %"PRIu32")",
///                             superblock.attr_max, lfs->attr_max);
///                     err = LFS_ERR_INVAL;
///                     goto cleanup;
///                 }
///
///                 lfs->attr_max = superblock.attr_max;
///
///                 // we also need to update inline_max in case attr_max changed
///                 lfs->inline_max = lfs_min(lfs->inline_max, lfs->attr_max);
///             }
///
///             // this is where we get the block_count from disk if block_count=0
///             if (lfs->cfg->block_count
///                     && superblock.block_count != lfs->cfg->block_count) {
///                 LFS_ERROR("Invalid block count (%"PRIu32" != %"PRIu32")",
///                         superblock.block_count, lfs->cfg->block_count);
///                 err = LFS_ERR_INVAL;
///                 goto cleanup;
///             }
///
///             lfs->block_count = superblock.block_count;
///
///             if (superblock.block_size != lfs->cfg->block_size) {
///                 LFS_ERROR("Invalid block size (%"PRIu32" != %"PRIu32")",
///                         superblock.block_size, lfs->cfg->block_size);
///                 err = LFS_ERR_INVAL;
///                 goto cleanup;
///             }
///         }
///
///         // has gstate?
///         err = lfs_dir_getgstate(lfs, &dir, &lfs->gstate);
///         if (err) {
///             goto cleanup;
///         }
///     }
///
///     // update littlefs with gstate
///     if (!lfs_gstate_iszero(&lfs->gstate)) {
///         LFS_DEBUG("Found pending gstate 0x%08"PRIx32"%08"PRIx32"%08"PRIx32,
///                 lfs->gstate.tag,
///                 lfs->gstate.pair[0],
///                 lfs->gstate.pair[1]);
///     }
///     lfs->gstate.tag += !lfs_tag_isvalid(lfs->gstate.tag);
///     lfs->gdisk = lfs->gstate;
///
///     // setup free lookahead, to distribute allocations uniformly across
///     // boots, we start the allocator at a random location
///     lfs->lookahead.start = lfs->seed % lfs->block_count;
///     lfs_alloc_drop(lfs);
///
///     return 0;
///
/// cleanup:
///     lfs_unmount_(lfs);
///     return err;
/// }
/// ```
pub fn lfs_mount_(lfs: *mut super::lfs::Lfs, cfg: *const crate::lfs_config::LfsConfig) -> i32 {
    use crate::block_alloc::alloc::lfs_alloc_drop;
    use crate::dir::fetch::{lfs_dir_fetchmatch, lfs_dir_getgstate};
    use crate::dir::find::{lfs_dir_find_match, LfsDirFindMatch};
    use crate::dir::traverse::lfs_dir_get;
    use crate::error::LFS_ERR_INVAL;
    use crate::fs::init::{lfs_deinit, lfs_init};
    use crate::fs::superblock::lfs_fs_prepsuperblock;
    use crate::lfs_gstate::lfs_gstate_iszero;
    use crate::lfs_superblock::{lfs_superblock_fromle32, LfsSuperblock};
    use crate::lfs_type::lfs_type::{LFS_TYPE_INLINESTRUCT, LFS_TYPE_SUPERBLOCK};
    use crate::tag::{lfs_mktag, lfs_tag_isdelete, lfs_tag_isvalid};
    use crate::types::{LFS_BLOCK_NULL, LFS_DISK_VERSION_MAJOR, LFS_DISK_VERSION_MINOR};
    use crate::util::{lfs_min, lfs_pair_isnull};

    let mut err = lfs_init(lfs, cfg);
    if err != 0 {
        return err;
    }

    unsafe {
        let lfs = &mut *lfs;
        let cfg = &*cfg;

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

        let magic = b"littlefs";
        let find_match = LfsDirFindMatch {
            lfs: lfs as *mut _,
            name: magic.as_ptr(),
            size: 8,
        };

        let mut err_inner = 0i32;
        #[cfg(feature = "loop_limits")]
        let mut mount_iter: u32 = 0;
        while !lfs_pair_isnull(&dir.tail) {
            #[cfg(feature = "loop_limits")]
            {
                if mount_iter >= 64 {
                    break;
                }
                mount_iter += 1;
            }
            err_inner = lfs_tortoise_detectcycles(&dir as *const _, &mut tortoise);
            if err_inner < 0 {
                break;
            }

            let tag = lfs_dir_fetchmatch(
                lfs as *mut _ as *const core::ffi::c_void,
                &mut dir as *mut _,
                &dir.tail as *const _,
                lfs_mktag(0x7ff, 0x3ff, 0),
                lfs_mktag(LFS_TYPE_SUPERBLOCK, 0, 8),
                core::ptr::null_mut(),
                Some(lfs_dir_find_match),
                &find_match as *const _ as *mut core::ffi::c_void,
            );

            if tag < 0 {
                err_inner = tag;
                break;
            }

            if tag != 0 && !lfs_tag_isdelete(tag as crate::types::lfs_tag_t) {
                lfs.root[0] = dir.pair[0];
                lfs.root[1] = dir.pair[1];

                let mut superblock = core::mem::zeroed::<LfsSuperblock>();
                let sbtag = lfs_dir_get(
                    lfs as *mut _,
                    &dir as *const _,
                    lfs_mktag(0x7ff, 0x3ff, 0),
                    lfs_mktag(
                        LFS_TYPE_INLINESTRUCT,
                        0,
                        core::mem::size_of::<LfsSuperblock>() as u32,
                    ),
                    &mut superblock as *mut _ as *mut core::ffi::c_void,
                );
                if sbtag < 0 {
                    err_inner = sbtag;
                    break;
                }
                lfs_superblock_fromle32(&mut superblock);

                let major_version = (0xffff & (superblock.version >> 16)) as u16;
                let minor_version = (0xffff & superblock.version) as u16;
                if major_version != LFS_DISK_VERSION_MAJOR as u16
                    || minor_version > LFS_DISK_VERSION_MINOR as u16
                {
                    err_inner = LFS_ERR_INVAL;
                    break;
                }

                let needssuperblock = minor_version < LFS_DISK_VERSION_MINOR as u16;
                lfs_fs_prepsuperblock(lfs as *mut _, needssuperblock);

                if superblock.name_max != 0 {
                    if superblock.name_max > lfs.name_max {
                        err_inner = LFS_ERR_INVAL;
                        break;
                    }
                    lfs.name_max = superblock.name_max;
                }
                if superblock.file_max != 0 {
                    if superblock.file_max > lfs.file_max {
                        err_inner = LFS_ERR_INVAL;
                        break;
                    }
                    lfs.file_max = superblock.file_max;
                }
                if superblock.attr_max != 0 {
                    if superblock.attr_max > lfs.attr_max {
                        err_inner = LFS_ERR_INVAL;
                        break;
                    }
                    lfs.attr_max = superblock.attr_max;
                    lfs.inline_max = lfs_min(lfs.inline_max, lfs.attr_max);
                }

                if cfg.block_count != 0 && superblock.block_count != cfg.block_count {
                    err_inner = LFS_ERR_INVAL;
                    break;
                }
                lfs.block_count = superblock.block_count;

                if superblock.block_size != cfg.block_size {
                    err_inner = LFS_ERR_INVAL;
                    break;
                }
            }

            err_inner = lfs_dir_getgstate(lfs as *mut _, &dir as *const _, &mut lfs.gstate);
            if err_inner != 0 {
                break;
            }
        }

        if err_inner != 0 {
            lfs_deinit(lfs as *mut _);
            err_inner
        } else {
            if !lfs_gstate_iszero(&lfs.gstate) {
                lfs.gstate.tag = lfs
                    .gstate
                    .tag
                    .wrapping_add(!lfs_tag_isvalid(lfs.gstate.tag) as u32);
            }
            lfs.gdisk = lfs.gstate;

            lfs.lookahead.start = lfs.seed % lfs.block_count;
            lfs_alloc_drop(lfs as *mut _);

            0
        }
    }
}

/// Per lfs.c lfs_unmount_ (lines 4647-4651)
///
/// C:
/// ```c
/// static int lfs_unmount_(lfs_t *lfs) {
///     return lfs_deinit(lfs);
/// }
///
///
/// ```
pub fn lfs_unmount_(lfs: *mut super::lfs::Lfs) -> i32 {
    crate::fs::init::lfs_deinit(lfs)
}
