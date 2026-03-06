//! File types and flags. Per lfs.h enum lfs_type, lfs_open_flags, lfs_whence_flags.

#![allow(clippy::module_inception, non_camel_case_types)]

/// File types. Per lfs.h enum lfs_type.
pub mod lfs_type {
    pub const LFS_TYPE_REG: u32 = 0x001;
    pub const LFS_TYPE_DIR: u32 = 0x002;
    pub const LFS_TYPE_SPLICE: u32 = 0x400;
    pub const LFS_TYPE_NAME: u32 = 0x000;
    pub const LFS_TYPE_STRUCT: u32 = 0x200;
    pub const LFS_TYPE_USERATTR: u32 = 0x300;
    pub const LFS_TYPE_FROM: u32 = 0x100;
    pub const LFS_TYPE_TAIL: u32 = 0x600;
    pub const LFS_TYPE_GLOBALS: u32 = 0x700;
    pub const LFS_TYPE_CRC: u32 = 0x500;
    pub const LFS_TYPE_CREATE: u32 = 0x401;
    pub const LFS_TYPE_DELETE: u32 = 0x4ff;
    pub const LFS_TYPE_SUPERBLOCK: u32 = 0x0ff;
    pub const LFS_TYPE_DIRSTRUCT: u32 = 0x200;
    pub const LFS_TYPE_CTZSTRUCT: u32 = 0x202;
    pub const LFS_TYPE_INLINESTRUCT: u32 = 0x201;
    pub const LFS_TYPE_SOFTTAIL: u32 = 0x600;
    pub const LFS_TYPE_HARDTAIL: u32 = 0x601;
    pub const LFS_TYPE_MOVESTATE: u32 = 0x7ff;
    pub const LFS_TYPE_CCRC: u32 = 0x500;
    pub const LFS_TYPE_FCRC: u32 = 0x5ff;
    pub const LFS_FROM_NOOP: u32 = 0x000;
    pub const LFS_FROM_MOVE: u32 = 0x101;
    pub const LFS_FROM_USERATTRS: u32 = 0x102;
}

/// Open flags. Per lfs.h enum lfs_open_flags.
pub mod lfs_open_flags {
    pub const LFS_O_RDONLY: i32 = 1;
    pub const LFS_O_WRONLY: i32 = 2;
    pub const LFS_O_RDWR: i32 = 3;
    pub const LFS_O_CREAT: i32 = 0x0100;
    pub const LFS_O_EXCL: i32 = 0x0200;
    pub const LFS_O_TRUNC: i32 = 0x0400;
    pub const LFS_O_APPEND: i32 = 0x0800;
    pub const LFS_F_DIRTY: i32 = 0x010000;
    pub const LFS_F_WRITING: i32 = 0x020000;
    pub const LFS_F_READING: i32 = 0x040000;
    pub const LFS_F_ERRED: i32 = 0x080000;
    pub const LFS_F_INLINE: i32 = 0x100000;
}

/// Seek whence. Per lfs.h enum lfs_whence_flags.
pub mod lfs_whence_flags {
    pub const LFS_SEEK_SET: i32 = 0;
    pub const LFS_SEEK_CUR: i32 = 1;
    pub const LFS_SEEK_END: i32 = 2;
}
