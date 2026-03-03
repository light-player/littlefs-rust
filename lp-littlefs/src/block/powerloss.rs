//! Power-loss simulation for tests.
//!
//! Wraps a block device and can fail sync() or discard prog() at configurable
//! points to simulate power loss during write/sync.

use super::BlockDevice;
use crate::Error;

/// Block device wrapper that can simulate power loss.
///
/// Use `set_fail_before_sync(true)` to cause the next `sync()` to return an
/// error. Use `set_drop_prog(true)` to discard prog writes until cleared.
pub struct PowerLossBlockDevice<B: BlockDevice> {
    inner: B,
    fail_before_sync: core::cell::Cell<bool>,
    drop_prog: core::cell::Cell<bool>,
}

impl<B: BlockDevice> PowerLossBlockDevice<B> {
    pub fn new(inner: B) -> Self {
        Self {
            inner,
            fail_before_sync: core::cell::Cell::new(false),
            drop_prog: core::cell::Cell::new(false),
        }
    }

    /// Cause the next sync() to return Error::Io.
    pub fn set_fail_before_sync(&self, fail: bool) {
        self.fail_before_sync.set(fail);
    }

    /// When true, prog() is a no-op (writes are discarded).
    pub fn set_drop_prog(&self, drop: bool) {
        self.drop_prog.set(drop);
    }

    /// Access the inner block device for raw operations (e.g. corruption).
    pub fn inner(&self) -> &B {
        &self.inner
    }
}

impl<B: BlockDevice> BlockDevice for PowerLossBlockDevice<B> {
    fn read(&self, block: u32, off: u32, buffer: &mut [u8]) -> Result<(), Error> {
        self.inner.read(block, off, buffer)
    }

    fn prog(&self, block: u32, off: u32, data: &[u8]) -> Result<(), Error> {
        if self.drop_prog.get() {
            return Ok(());
        }
        self.inner.prog(block, off, data)
    }

    fn erase(&self, block: u32) -> Result<(), Error> {
        self.inner.erase(block)
    }

    fn sync(&self) -> Result<(), Error> {
        if self.fail_before_sync.get() {
            self.fail_before_sync.set(false);
            return Err(Error::Io);
        }
        self.inner.sync()
    }
}
