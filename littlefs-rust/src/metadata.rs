use alloc::string::String;
use bitflags::bitflags;

/// Type of a filesystem entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileType {
    File,
    Dir,
}

/// Metadata for a file or directory, as returned by [`Filesystem::stat`](crate::Filesystem::stat).
#[derive(Debug, Clone)]
pub struct Metadata {
    pub file_type: FileType,
    pub size: u32,
    pub name: String,
}

/// A single entry from a directory listing.
#[derive(Debug, Clone)]
pub struct DirEntry {
    pub name: String,
    pub file_type: FileType,
    pub size: u32,
}

bitflags! {
    /// Flags for opening a file. Combine with `|`.
    ///
    /// Common combinations:
    /// - Read-only: `OpenFlags::READ`
    /// - Create or overwrite: `OpenFlags::WRITE | OpenFlags::CREATE | OpenFlags::TRUNC`
    /// - Append: `OpenFlags::WRITE | OpenFlags::CREATE | OpenFlags::APPEND`
    /// - Create only (fail if exists): `OpenFlags::WRITE | OpenFlags::CREATE | OpenFlags::EXCL`
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct OpenFlags: u32 {
        const READ   = 0x1;
        const WRITE  = 0x2;
        const CREATE = 0x100;
        const EXCL   = 0x200;
        const TRUNC  = 0x400;
        const APPEND = 0x800;
    }
}

/// Position for [`File::seek`](crate::File::seek).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SeekFrom {
    /// Start of file.
    Start(u32),
    /// Current position (offset can be negative).
    Current(i32),
    /// End of file (offset can be negative).
    End(i32),
}
