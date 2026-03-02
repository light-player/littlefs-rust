//! Mount implementation.
//!
//! Reads and validates superblock from metadata pair (blocks 0, 1).

use crate::block::BlockDevice;
use crate::config::Config;
use crate::error::Error;
use crate::superblock::{MAGIC, REVISION_OFFSET};

/// Mount state to store in LittleFs.
#[derive(Clone)]
pub(crate) struct MountState {
    pub root: [u32; 2],
    pub block_size: u32,
    pub block_count: u32,
    pub name_max: u32,
    pub file_max: u32,
    pub attr_max: u32,
    pub disk_version: u32,
}

pub fn mount<B: BlockDevice>(bd: &B, config: &Config) -> Result<MountState, Error> {
    let block_size = config.block_size as usize;
    let mut block0 = alloc::vec![0u8; block_size];
    let mut block1 = alloc::vec![0u8; block_size];

    bd.read(0, 0, &mut block0)?;
    bd.read(1, 0, &mut block1)?;

    // Pick block with higher revision.
    let rev0 = u32::from_le_bytes(
        block0[REVISION_OFFSET as usize..REVISION_OFFSET as usize + 4]
            .try_into()
            .unwrap(),
    );
    let rev1 = u32::from_le_bytes(
        block1[REVISION_OFFSET as usize..REVISION_OFFSET as usize + 4]
            .try_into()
            .unwrap(),
    );

    let block = if (rev0 as i32).wrapping_sub(rev1 as i32) >= 0 {
        &block0
    } else {
        &block1
    };

    // Check magic at offset 12. Layout: [rev:4][create_tag:4][sb_tag:4][magic:8]...
    if block.len() < 20 {
        return Err(Error::Corrupt);
    }
    if &block[12..20] != MAGIC {
        return Err(Error::Corrupt);
    }

    // Validate block_size and block_count from superblock (at offset 20 + 4 + 8 = 32)
    // Layout after magic: [tag:4][superblock:24]. So superblock starts at 8+4+8 = 20? No.
    // [rev:4][create_tag:4][sb_tag:4][littlefs:8][struct_tag:4][superblock:24]
    use crate::superblock::Superblock;
    let sb_off = 24usize;
    if block.len() < sb_off + Superblock::SIZE {
        return Err(Error::Corrupt);
    }
    let disk_block_size = u32::from_le_bytes(block[sb_off + 4..sb_off + 8].try_into().unwrap());
    let disk_block_count = u32::from_le_bytes(block[sb_off + 8..sb_off + 12].try_into().unwrap());

    if disk_block_size != config.block_size {
        return Err(Error::Corrupt);
    }
    if config.block_count != 0 && disk_block_count != config.block_count {
        return Err(Error::Corrupt);
    }

    let name_max = u32::from_le_bytes(block[sb_off + 12..sb_off + 16].try_into().unwrap());
    let file_max = u32::from_le_bytes(block[sb_off + 16..sb_off + 20].try_into().unwrap());
    let attr_max = u32::from_le_bytes(block[sb_off + 20..sb_off + 24].try_into().unwrap());
    let disk_version = u32::from_le_bytes(block[sb_off..sb_off + 4].try_into().unwrap());

    Ok(MountState {
        root: [0, 1],
        block_size: disk_block_size,
        block_count: disk_block_count,
        name_max,
        file_max,
        attr_max,
        disk_version,
    })
}
