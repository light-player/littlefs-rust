//! Type definitions. Per lfs.h typedefs and limits.

#![allow(non_camel_case_types)]

/// uint32_t
pub type lfs_size_t = u32;
/// uint32_t
pub type lfs_off_t = u32;
/// int32_t (signed size)
pub type lfs_ssize_t = i32;
/// int32_t (signed offset)
pub type lfs_soff_t = i32;
/// uint32_t (block index)
pub type lfs_block_t = u32;
/// uint32_t (metadata tag)
pub type lfs_tag_t = u32;
/// int32_t (signed tag)
pub type lfs_stag_t = i32;

/// Max name length in bytes. Stored in superblock.
pub const LFS_NAME_MAX: usize = 255;
/// Max file size in bytes.
pub const LFS_FILE_MAX: i64 = 2_147_483_647;
/// Max custom attribute size in bytes.
pub const LFS_ATTR_MAX: usize = 1022;

/// Version info. Per lfs.h LFS_VERSION.
pub const LFS_VERSION: u32 = 0x0002_000b;
pub const LFS_VERSION_MAJOR: u32 = 0xffff & (LFS_VERSION >> 16);
pub const LFS_VERSION_MINOR: u32 = 0xffff & LFS_VERSION;

/// On-disk version. Per lfs.h LFS_DISK_VERSION.
pub const LFS_DISK_VERSION: u32 = 0x0002_0001;
pub const LFS_DISK_VERSION_MAJOR: u32 = 0xffff & (LFS_DISK_VERSION >> 16);
pub const LFS_DISK_VERSION_MINOR: u32 = 0xffff & LFS_DISK_VERSION;

/// Per lfs.c LFS_BLOCK_NULL, LFS_BLOCK_INLINE
pub const LFS_BLOCK_NULL: lfs_block_t = u32::MAX; // (lfs_block_t)-1
pub const LFS_BLOCK_INLINE: lfs_block_t = u32::MAX - 1; // -2
