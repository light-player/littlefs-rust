use crate::error::Error;

/// Block device storage backend.
///
/// Implement this trait to connect a flash chip, SD card, or any other block
/// device. See [`RamStorage`](crate::RamStorage) for a minimal example.
pub trait Storage {
    /// Read `buf.len()` bytes starting at `offset` within `block`.
    fn read(&mut self, block: u32, offset: u32, buf: &mut [u8]) -> Result<(), Error>;

    /// Write `data` starting at `offset` within `block`.
    ///
    /// The block must have been erased before writing.
    fn write(&mut self, block: u32, offset: u32, data: &[u8]) -> Result<(), Error>;

    /// Erase `block`, resetting all bytes to the erased state (typically `0xFF`).
    fn erase(&mut self, block: u32) -> Result<(), Error>;

    /// Flush pending writes. The default implementation is a no-op.
    fn sync(&mut self) -> Result<(), Error> {
        Ok(())
    }
}
