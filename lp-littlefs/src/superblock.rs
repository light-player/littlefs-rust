//! Superblock and metadata layout.
//!
//! Per SPEC.md and lfs.h. Tags are big-endian in commits.

/// Magic string at offset 8 in a valid superblock metadata block.
/// SPEC: "littlefs" (8 bytes).
pub const MAGIC: &[u8; 8] = b"littlefs";

/// Disk version 2.1 (0x00020001).
/// Matches LFS_DISK_VERSION in lfs.h.
pub const DISK_VERSION: u32 = 0x0002_0001;

/// Metadata tag types (lfs.h enum lfs_type).
/// Tags in commits are stored big-endian.
/// Use type1 = (tag >> 20) & 0x7, type3 = (tag >> 20) & 0x7ff for branching.
#[allow(dead_code)]
pub mod tag {
    // Abstract type1 (3 bits)
    pub const TYPE_NAME: u32 = 0x000;
    pub const TYPE_STRUCT: u32 = 0x200;
    pub const TYPE_SPLICE: u32 = 0x400;
    pub const TYPE_TAIL: u32 = 0x600;
    pub const TYPE_CRC: u32 = 0x500;

    // type3 specializations
    pub const TYPE_CREATE: u32 = 0x401;
    pub const TYPE_DELETE: u32 = 0x4ff;
    pub const TYPE_SUPERBLOCK: u32 = 0x0ff;
    pub const TYPE_REG: u32 = 0x001;
    pub const TYPE_DIR: u32 = 0x002;
    pub const TYPE_INLINESTRUCT: u32 = 0x201;
    pub const TYPE_DIRSTRUCT: u32 = 0x200;
    pub const TYPE_CTZSTRUCT: u32 = 0x202;
    pub const TYPE_SOFTTAIL: u32 = 0x600;
    pub const TYPE_HARDTAIL: u32 = 0x601;
    pub const TYPE_CCRC: u32 = 0x500;
}

/// On-disk superblock struct (little-endian).
/// Matches lfs_superblock_t in lfs.h.
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct Superblock {
    pub version: u32,
    pub block_size: u32,
    pub block_count: u32,
    pub name_max: u32,
    pub file_max: u32,
    pub attr_max: u32,
}

impl Superblock {
    /// Size of the on-disk superblock struct in bytes.
    pub const SIZE: usize = 24;
}

/// Revision count at start of metadata block (4 bytes, little-endian).
pub const REVISION_OFFSET: u32 = 0;

/// Magic is at offset 12 in block. Layout: [rev:4][create_tag:4][sb_tag:4][magic:8]...
pub const MAGIC_OFFSET: u32 = 12;
