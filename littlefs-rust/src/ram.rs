use alloc::vec;
use alloc::vec::Vec;

use crate::error::Error;
use crate::storage::Storage;

/// In-memory block device for testing and examples.
///
/// Simulates flash: erased blocks are `0xFF`, writes overwrite bytes, and
/// erase resets a block to `0xFF`. Use with [`Config`](crate::Config) and
/// [`Filesystem`](crate::Filesystem).
pub struct RamStorage {
    data: Vec<u8>,
    block_size: u32,
    block_count: u32,
}

impl RamStorage {
    /// Create a new RAM-backed storage with the given block geometry.
    pub fn new(block_size: u32, block_count: u32) -> Self {
        let size = (block_size as usize)
            .checked_mul(block_count as usize)
            .expect("block_size * block_count overflow");
        Self {
            data: vec![0xFFu8; size],
            block_size,
            block_count,
        }
    }

    /// Return the raw storage bytes (for inspection or persistence).
    pub fn data(&self) -> &[u8] {
        &self.data
    }

    pub fn block_size(&self) -> u32 {
        self.block_size
    }

    pub fn block_count(&self) -> u32 {
        self.block_count
    }

    fn offset(&self, block: u32, off: u32) -> usize {
        (block as usize) * (self.block_size as usize) + (off as usize)
    }
}

impl Storage for RamStorage {
    fn read(&mut self, block: u32, offset: u32, buf: &mut [u8]) -> Result<(), Error> {
        let start = self.offset(block, offset);
        let end = start + buf.len();
        if end > self.data.len() {
            return Err(Error::Io);
        }
        buf.copy_from_slice(&self.data[start..end]);
        Ok(())
    }

    fn write(&mut self, block: u32, offset: u32, data: &[u8]) -> Result<(), Error> {
        let start = self.offset(block, offset);
        let end = start + data.len();
        if end > self.data.len() {
            return Err(Error::Io);
        }
        self.data[start..end].copy_from_slice(data);
        Ok(())
    }

    fn erase(&mut self, block: u32) -> Result<(), Error> {
        let start = self.offset(block, 0);
        let end = start + self.block_size as usize;
        if end > self.data.len() {
            return Err(Error::Io);
        }
        self.data[start..end].fill(0xFF);
        Ok(())
    }
}
