//! Pure Rust implementation of the LittleFS embedded filesystem.
//!
//! No C dependencies—avoids C compiler and cross-compilation issues on embedded targets.

#![no_std]
#![allow(clippy::too_many_arguments)]

extern crate alloc;

mod block;
mod config;
mod crc;
mod error;
mod fs;
mod info;
mod superblock;
mod trace;

pub use block::{BlockDevice, PowerLossBlockDevice, RamBlockDevice};
pub use config::Config;
pub use error::Error;
pub use fs::{create_inline_file, Dir, File, LittleFs};
pub use info::{FileOpenConfig, FileType, FsInfo, Info, OpenFlags, SeekWhence, ATTR_MAX};
pub use superblock::{MAGIC, MAGIC_OFFSET};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_variants() {
        assert_eq!(Error::Corrupt, Error::Corrupt);
    }
}
