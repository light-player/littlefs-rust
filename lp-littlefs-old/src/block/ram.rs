//! In-memory block device for testing.

use super::BlockDevice;
use crate::Error;
use alloc::vec::Vec;
use core::cell::UnsafeCell;

/// RAM-backed block device.
///
/// Used for tests. Storage is (block_count * block_size) bytes.
/// Uses UnsafeCell for interior mutability (trait uses &self).
pub struct RamBlockDevice {
    storage: UnsafeCell<Vec<u8>>,
    block_size: u32,
    block_count: u32,
}

impl RamBlockDevice {
    /// Create a new RAM block device.
    ///
    /// `block_size` and `block_count` define the geometry. Total size is
    /// block_size * block_count bytes. Storage is zeroed initially.
    pub fn new(block_size: u32, block_count: u32) -> Self {
        let size = (block_size as usize) * (block_count as usize);
        Self {
            storage: UnsafeCell::new(alloc::vec![0; size]),
            block_size,
            block_count,
        }
    }
}

impl BlockDevice for RamBlockDevice {
    fn read(&self, block: u32, off: u32, buffer: &mut [u8]) -> Result<(), Error> {
        if block >= self.block_count {
            return Err(Error::Inval);
        }
        let storage = unsafe { &*self.storage.get() };
        let start = (block as usize) * (self.block_size as usize) + (off as usize);
        let end = start + buffer.len();
        if end > storage.len() {
            return Err(Error::Inval);
        }
        buffer.copy_from_slice(&storage[start..end]);
        Ok(())
    }

    fn prog(&self, block: u32, off: u32, data: &[u8]) -> Result<(), Error> {
        if block >= self.block_count {
            return Err(Error::Inval);
        }
        let storage = unsafe { &mut *self.storage.get() };
        let start = (block as usize) * (self.block_size as usize) + (off as usize);
        let end = start + data.len();
        if end > storage.len() {
            return Err(Error::Inval);
        }
        storage[start..end].copy_from_slice(data);
        Ok(())
    }

    fn erase(&self, block: u32) -> Result<(), Error> {
        if block >= self.block_count {
            return Err(Error::Inval);
        }
        let storage = unsafe { &mut *self.storage.get() };
        let start = (block as usize) * (self.block_size as usize);
        let end = start + (self.block_size as usize);
        for b in &mut storage[start..end] {
            *b = 0xff;
        }
        Ok(())
    }

    fn sync(&self) -> Result<(), Error> {
        Ok(())
    }
}
