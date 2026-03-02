//! File and filesystem info types.
//!
//! Per lfs.h struct lfs_info (lines 297–310) and lfs_fsinfo (lines 313–332).

/// Open flags for file_open. Per lfs.h LFS_O_RDONLY etc.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct OpenFlags(pub u32);

impl OpenFlags {
    pub const RDONLY: u32 = 1;

    pub fn new(flags: u32) -> Self {
        Self(flags)
    }

    pub fn contains(&self, flag: u32) -> bool {
        (self.0 & flag) != 0
    }
}

/// Seek whence. Per lfs.h LFS_SEEK_SET etc.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SeekWhence {
    /// From start of file
    Set = 0,
    /// From current position
    Cur = 1,
    /// From end of file
    End = 2,
}

/// File type: regular file or directory.
/// Per lfs.h enum lfs_type LFS_TYPE_REG (0x001), LFS_TYPE_DIR (0x002).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum FileType {
    /// Regular file
    Reg = 0x01,
    /// Directory
    Dir = 0x02,
}

impl FileType {
    pub fn from_type3(type3: u32) -> Option<Self> {
        match type3 {
            0x001 => Some(FileType::Reg),
            0x002 => Some(FileType::Dir),
            _ => None,
        }
    }
}

/// File or directory info.
/// Per lfs.h struct lfs_info.
#[derive(Clone, Debug)]
pub struct Info {
    /// Type of the file (Reg or Dir).
    pub typ: FileType,
    /// Size in bytes. Only valid for Reg files.
    pub size: u32,
    /// Name, NUL-terminated. LFS_NAME_MAX+1 = 256 bytes.
    name: [u8; 256],
}

impl Info {
    pub fn new(typ: FileType, size: u32) -> Self {
        Self {
            typ,
            size,
            name: [0; 256],
        }
    }

    /// Set name from bytes. Copies up to 255 bytes plus NUL.
    pub fn set_name(&mut self, bytes: &[u8]) {
        let len = core::cmp::min(bytes.len(), 255);
        self.name[..len].copy_from_slice(&bytes[..len]);
        self.name[len] = 0;
    }

    /// Name as str, up to first NUL.
    pub fn name(&self) -> Result<&str, core::str::Utf8Error> {
        let nul = self.name.iter().position(|&b| b == 0).unwrap_or(256);
        core::str::from_utf8(&self.name[..nul])
    }

    /// Name as bytes slice, up to first NUL.
    pub fn name_bytes(&self) -> &[u8] {
        let nul = self.name.iter().position(|&b| b == 0).unwrap_or(256);
        &self.name[..nul]
    }
}

/// Filesystem info from superblock.
/// Per lfs.h struct lfs_fsinfo.
#[derive(Debug, Clone, Copy)]
pub struct FsInfo {
    pub disk_version: u32,
    pub block_size: u32,
    pub block_count: u32,
    pub name_max: u32,
    pub file_max: u32,
    pub attr_max: u32,
}
