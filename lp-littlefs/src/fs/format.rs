//! Format. Per lfs.c lfs_format_.

use crate::bd::bd::lfs_bd_sync;
use crate::block_alloc::alloc::lfs_alloc_ckpoint;
use crate::dir::commit::lfs_dir_alloc;
use crate::dir::commit::lfs_dir_commit;
use crate::dir::fetch::lfs_dir_fetch;
use crate::dir::LfsMdir;
use crate::fs::init::{lfs_deinit, lfs_init};
use crate::lfs_superblock::lfs_superblock_tole32;
use crate::lfs_superblock::LfsSuperblock;
use crate::lfs_type::lfs_type::{LFS_TYPE_CREATE, LFS_TYPE_INLINESTRUCT, LFS_TYPE_SUPERBLOCK};
use crate::tag::lfs_mktag;
use crate::types::LFS_DISK_VERSION;
use crate::util::lfs_min;

/// Per lfs.c lfs_format_ (lines 4391-4462)
///
/// C:
/// ```c
/// static int lfs_format_(lfs_t *lfs, const struct lfs_config *cfg) {
///     int err = 0;
///     {
///         err = lfs_init(lfs, cfg);
///         if (err) {
///             return err;
///         }
///
///         LFS_ASSERT(cfg->block_count != 0);
///
///         // create free lookahead
///         memset(lfs->lookahead.buffer, 0, lfs->cfg->lookahead_size);
///         lfs->lookahead.start = 0;
///         lfs->lookahead.size = lfs_min(8*lfs->cfg->lookahead_size,
///                 lfs->block_count);
///         lfs->lookahead.next = 0;
///         lfs_alloc_ckpoint(lfs);
///
///         // create root dir
///         lfs_mdir_t root;
///         err = lfs_dir_alloc(lfs, &root);
///         if (err) {
///             goto cleanup;
///         }
///
///         // write one superblock
///         lfs_superblock_t superblock = {
///             .version     = lfs_fs_disk_version(lfs),
///             .block_size  = lfs->cfg->block_size,
///             .block_count = lfs->block_count,
///             .name_max    = lfs->name_max,
///             .file_max    = lfs->file_max,
///             .attr_max    = lfs->attr_max,
///         };
///
///         lfs_superblock_tole32(&superblock);
///         err = lfs_dir_commit(lfs, &root, LFS_MKATTRS(
///                 {LFS_MKTAG(LFS_TYPE_CREATE, 0, 0), NULL},
///                 {LFS_MKTAG(LFS_TYPE_SUPERBLOCK, 0, 8), "littlefs"},
///                 {LFS_MKTAG(LFS_TYPE_INLINESTRUCT, 0, sizeof(superblock)),
///                     &superblock}));
///         if (err) {
///             goto cleanup;
///         }
///
///         // force compaction to prevent accidentally mounting any
///         // older version of littlefs that may live on disk
///         root.erased = false;
///         err = lfs_dir_commit(lfs, &root, NULL, 0);
///         if (err) {
///             goto cleanup;
///         }
///
///         // sanity check that fetch works
///         err = lfs_dir_fetch(lfs, &root, (const lfs_block_t[2]){0, 1});
///         if (err) {
///             goto cleanup;
///         }
///     }
///
/// cleanup:
///     lfs_deinit(lfs);
///     return err;
///
/// }
/// #endif
///
/// struct lfs_tortoise_t {
///     lfs_block_t pair[2];
///     lfs_size_t i;
///     lfs_size_t period;
/// };
/// ```
pub fn lfs_format_(lfs: *mut super::lfs::Lfs, cfg: *const crate::lfs_config::LfsConfig) -> i32 {
    let mut err = lfs_init(lfs, cfg);
    if err != 0 {
        lfs_deinit(lfs);
        return err;
    }

    unsafe {
        let lfs = &mut *lfs;
        let cfg = &*cfg;
        crate::lfs_assert!(cfg.block_count != 0);

        // create free lookahead
        if !lfs.lookahead.buffer.is_null() {
            core::ptr::write_bytes(lfs.lookahead.buffer, 0, cfg.lookahead_size as usize);
        }
        lfs.lookahead.start = 0;
        lfs.lookahead.size = lfs_min(8 * cfg.lookahead_size, lfs.block_count);
        lfs.lookahead.next = 0;
        lfs_alloc_ckpoint(lfs);

        // create root dir
        let mut root = LfsMdir {
            pair: [0, 0],
            rev: 0,
            off: 0,
            etag: 0,
            count: 0,
            erased: false,
            split: false,
            tail: [0, 0],
        };
        err = lfs_dir_alloc(lfs, &mut root);
        if err != 0 {
            lfs_deinit(lfs as *mut _);
            return err;
        }

        // write one superblock
        let mut superblock = LfsSuperblock {
            version: LFS_DISK_VERSION,
            block_size: cfg.block_size,
            block_count: lfs.block_count,
            name_max: lfs.name_max,
            file_max: lfs.file_max,
            attr_max: lfs.attr_max,
        };
        lfs_superblock_tole32(&mut superblock);

        let magic = b"littlefs";
        let attrs = [
            crate::tag::lfs_mattr {
                tag: lfs_mktag(LFS_TYPE_CREATE, 0, 0),
                buffer: core::ptr::null(),
            },
            crate::tag::lfs_mattr {
                tag: lfs_mktag(LFS_TYPE_SUPERBLOCK, 0, 8),
                buffer: magic.as_ptr() as *const core::ffi::c_void,
            },
            crate::tag::lfs_mattr {
                tag: lfs_mktag(
                    LFS_TYPE_INLINESTRUCT,
                    0,
                    core::mem::size_of::<LfsSuperblock>() as u32,
                ),
                buffer: &superblock as *const _ as *const _,
            },
        ];
        err = lfs_dir_commit(
            lfs,
            &mut root,
            attrs.as_ptr() as *const core::ffi::c_void,
            3,
        );
        if err != 0 {
            lfs_deinit(lfs as *mut _);
            return err;
        }

        // Flush pcache so the second commit can read the first block from disk.
        // Otherwise the second compact reads from a block that was never written.
        err = lfs_bd_sync(lfs, &mut lfs.pcache, &mut lfs.rcache, false);
        if err != 0 {
            lfs_deinit(lfs as *mut _);
            return err;
        }

        // force compaction to prevent accidentally mounting any older version
        root.erased = false;
        err = lfs_dir_commit(lfs, &mut root, core::ptr::null(), 0);
        if err != 0 {
            lfs_deinit(lfs as *mut _);
            return err;
        }

        // sanity check that fetch works
        err = lfs_dir_fetch(lfs, &mut root, &root.pair);
        if err != 0 {
            lfs_deinit(lfs as *mut _);
            return err;
        }

        // flush pcache so raw block reads (e.g. test_superblocks_magic) see data
        err = lfs_bd_sync(lfs, &mut lfs.pcache, &mut lfs.rcache, false);
        if err != 0 {
            lfs_deinit(lfs as *mut _);
            return err;
        }
    }

    lfs_deinit(lfs);
    0
}
