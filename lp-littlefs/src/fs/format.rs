//! Format implementation.
//!
//! Writes initial superblock to metadata pair (blocks 0, 1) per SPEC.md.

use crate::block::BlockDevice;
use crate::config::Config;
use crate::crc;
use crate::error::Error;
use crate::superblock::{tag, Superblock, DISK_VERSION};

/// LFS_NAME_MAX, LFS_FILE_MAX, LFS_ATTR_MAX from lfs.h
const NAME_MAX: u32 = 255;
const FILE_MAX: u32 = 2_147_483_647;
const ATTR_MAX: u32 = 1022;

fn mktag(type_: u32, id: u32, size: u32) -> u32 {
    (type_ << 20) | (id << 10) | size
}

fn to_be32(x: u32) -> [u8; 4] {
    x.to_be_bytes()
}

fn to_le32(x: u32) -> [u8; 4] {
    x.to_le_bytes()
}

pub fn format<B: BlockDevice>(bd: &B, config: &Config) -> Result<(), Error> {
    if config.block_count == 0 {
        return Err(Error::Inval);
    }

    let block_size = config.block_size as usize;
    let prog_size = config.prog_size as usize;

    let mut block = alloc::vec![0xff; block_size];

    // Revision 1 (LE)
    block[0..4].copy_from_slice(&to_le32(1));

    let mut ptag: u32 = 0xffff_ffff;
    let mut off = 4usize;
    let mut crc = crc::crc32(0xffff_ffff, &block[0..4]);

    // CREATE id 0 size 0
    let tag_create = mktag(tag::TYPE_CREATE, 0, 0);
    let stored_tag = (tag_create & 0x7fff_ffff) ^ ptag;
    block[off..off + 4].copy_from_slice(&to_be32(stored_tag));
    crc = crc::crc32(crc, &block[off..off + 4]);
    ptag = tag_create & 0x7fff_ffff;
    off += 4;

    // SUPERBLOCK id 0 size 8, data "littlefs"
    let tag_sb = mktag(tag::TYPE_SUPERBLOCK, 0, 8);
    let stored_tag = (tag_sb & 0x7fff_ffff) ^ ptag;
    block[off..off + 4].copy_from_slice(&to_be32(stored_tag));
    crc = crc::crc32(crc, &block[off..off + 4]);
    ptag = tag_sb & 0x7fff_ffff;
    off += 4;
    block[off..off + 8].copy_from_slice(b"littlefs");
    crc = crc::crc32(crc, b"littlefs");
    off += 8;

    // INLINESTRUCT id 0 size 24, superblock
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

    let tag_struct = mktag(tag::TYPE_INLINESTRUCT, 0, Superblock::SIZE as u32);
    let stored_tag = (tag_struct & 0x7fff_ffff) ^ ptag;
    block[off..off + 4].copy_from_slice(&to_be32(stored_tag));
    crc = crc::crc32(crc, &block[off..off + 4]);
    ptag = tag_struct & 0x7fff_ffff;
    off += 4;
    block[off..off + Superblock::SIZE].copy_from_slice(&sb_bytes);
    crc = crc::crc32(crc, &sb_bytes);
    off += Superblock::SIZE;

    // Pad to prog_size for CRC tag+value. Need CRC tag (4) + CRC (4) = 8.
    off = (off + prog_size - 1) & !(prog_size - 1);
    if off + 8 > block_size {
        return Err(Error::Nospc);
    }

    // CRC tag - LFS_TYPE_CCRC = 0x500. Size is noff - (commit->off+4).
    // Simplified: use TYPE_CRC (0x500), id 0x3ff, size 4.
    let tag_crc = mktag(tag::TYPE_CRC, 0x3ff, 4);
    let stored_crc_tag = (tag_crc & 0x7fff_ffff) ^ ptag;
    block[off..off + 4].copy_from_slice(&to_be32(stored_crc_tag));
    crc = crc::crc32(crc, &block[off..off + 4]);
    off += 4;
    block[off..off + 4].copy_from_slice(&to_le32(crc));
    off += 4;

    // Pad remainder with 0xff
    block[off..].fill(0xff);

    // Erase and prog blocks 0 and 1
    bd.erase(0)?;
    bd.erase(1)?;
    for i in (0..block_size).step_by(prog_size) {
        bd.prog(0, i as u32, &block[i..i + prog_size])?;
        bd.prog(1, i as u32, &block[i..i + prog_size])?;
    }
    bd.sync()?;
    Ok(())
}
