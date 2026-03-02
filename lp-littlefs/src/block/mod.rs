//! Block device abstraction for littlefs.
//!
//! Maps to lfs_config read/prog/erase/sync callbacks.

mod ram;

pub use ram::RamBlockDevice;

use crate::Error;

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
