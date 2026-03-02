//! Block device abstraction for littlefs.
//!
//! Maps to lfs_config read/prog/erase/sync callbacks.

mod cache;
mod ram;

pub use cache::CachedBlockDevice;
pub use ram::RamBlockDevice;

use crate::crc;
use crate::Error;

/// Compute CRC32 of a block region. Per lfs_bd_crc. Used for FCRC and fs_gc.
#[allow(dead_code)]
pub fn block_crc<B: BlockDevice>(
    bd: &B,
    block: u32,
    off: u32,
    size: usize,
    init: u32,
) -> Result<u32, Error> {
    let mut buf = [0u8; 64];
    let mut c = init;
    let mut pos = 0u32;
    while pos < size as u32 {
        let n = (size - pos as usize).min(buf.len());
        bd.read(block, off + pos, &mut buf[..n])?;
        c = crc::crc32(c, &buf[..n]);
        pos += n as u32;
    }
    Ok(c)
}

/// Block device interface for littlefs storage.
///
/// All operations use caller-provided buffers. Block indices and offsets
/// follow littlefs semantics: block_size units, prog_size alignment for writes.
pub trait BlockDevice {
    /// Read data from a block. Buffer length must be multiple of read_size.
    fn read(&self, block: u32, off: u32, buffer: &mut [u8]) -> Result<(), Error>;

    /// Program data to a block. Block must be erased first. Data length
    /// must be multiple of prog_size.
    fn prog(&self, block: u32, off: u32, data: &[u8]) -> Result<(), Error>;

    /// Erase a block. Must be called before programming.
    fn erase(&self, block: u32) -> Result<(), Error>;

    /// Sync any cached state to the device.
    fn sync(&self) -> Result<(), Error>;
}
