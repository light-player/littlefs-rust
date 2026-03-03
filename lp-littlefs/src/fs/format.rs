//! Format implementation.
//!
//! Writes initial superblock to metadata pair (blocks 0, 1) per SPEC.md.
//! Uses dir_commit_append + dir_compact for C-compatible CRC layout.

use core::cell::RefCell;

use crate::block::BlockDevice;
use crate::config::Config;
use crate::error::Error;
use crate::superblock::{Superblock, DISK_VERSION};

use super::bdcache;
use super::commit;
use super::metadata;

/// LFS_NAME_MAX, LFS_FILE_MAX, LFS_ATTR_MAX from lfs.h
const NAME_MAX: u32 = 255;
const FILE_MAX: u32 = 2_147_483_647;
const ATTR_MAX: u32 = 1022;

pub fn format<B: BlockDevice>(bd: &B, config: &Config) -> Result<(), Error> {
    if config.block_count == 0 {
        return Err(Error::Inval);
    }

    let rcache = RefCell::new(bdcache::new_read_cache(config)?);
    let pcache = RefCell::new(bdcache::new_prog_cache(config)?);
    let ctx = bdcache::BdContext::new(bd, config, &rcache, &pcache);

    let block_size = config.block_size as usize;
    let root = [0u32, 1];

    ctx.erase(0)?;
    ctx.erase(1)?;

    let superblock = Superblock {
        version: DISK_VERSION,
        block_size: config.block_size,
        block_count: config.block_count,
        name_max: NAME_MAX,
        file_max: FILE_MAX,
        attr_max: ATTR_MAX,
    };
    let sb_bytes: [u8; Superblock::SIZE] = [
        superblock.version.to_le_bytes(),
        superblock.block_size.to_le_bytes(),
        superblock.block_count.to_le_bytes(),
        superblock.name_max.to_le_bytes(),
        superblock.file_max.to_le_bytes(),
        superblock.attr_max.to_le_bytes(),
    ]
    .into_iter()
    .flatten()
    .collect::<alloc::vec::Vec<_>>()
    .try_into()
    .unwrap();

    let mut dir = metadata::MdDir::alloc_empty(root, block_size);
    let attrs = [
        commit::CommitAttr::create(0),
        commit::CommitAttr::superblock_magic(),
        commit::CommitAttr::inline_struct(0, &sb_bytes),
    ];

    commit::dir_commit_append(&ctx, &mut dir, &attrs, &mut None, DISK_VERSION)?;
    bdcache::bd_sync(bd, config, &rcache, &pcache)?;

    // Mirror block 0 to block 1 so both have identical content (rev 1).
    // C format uses dir_compact which swaps; we copy instead to keep both rev 1
    // for consistent fetch behavior when revs are equal.
    let mut block = alloc::vec![0xff; block_size];
    ctx.read(0, 0, &mut block)?;
    ctx.erase(1)?;
    for i in (0..block_size).step_by(config.prog_size as usize) {
        ctx.prog(1, i as u32, &block[i..i + config.prog_size as usize])?;
    }
    bdcache::bd_sync(bd, config, &rcache, &pcache)?;

    Ok(())
}
