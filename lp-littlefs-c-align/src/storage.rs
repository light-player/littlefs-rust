//! Shared storage bridge for C ↔ Rust alignment tests.

use core::cell::UnsafeCell;
use std::os::raw::c_void;

use lp_littlefs::{BlockDevice, Config, Error};

/// Shared storage that both C (littlefs2-sys) and Rust (lp-littlefs) can use.
///
/// Implements BlockDevice for Rust. Provides lfs_config with callbacks for C.
pub struct AlignStorage {
    storage: UnsafeCell<Vec<u8>>,
    block_size: u32,
    block_count: u32,
}

impl AlignStorage {
    /// Create storage matching the given config geometry.
    pub fn new(config: &Config) -> Self {
        let size = (config.block_size as usize) * (config.block_count as usize);
        Self {
            storage: UnsafeCell::new(vec![0; size]),
            block_size: config.block_size,
            block_count: config.block_count,
        }
    }

    /// Reset storage to erased state (0xff) for re-use between tests.
    pub fn erase_all(&self) {
        let storage = unsafe { &mut *self.storage.get() };
        for b in storage.iter_mut() {
            *b = 0xff;
        }
    }

    fn read_impl(&self, block: u32, off: u32, buffer: &mut [u8]) -> Result<(), Error> {
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

    fn prog_impl(&self, block: u32, off: u32, data: &[u8]) -> Result<(), Error> {
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

    fn erase_impl(&self, block: u32) -> Result<(), Error> {
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

    fn sync_impl(&self) -> Result<(), Error> {
        Ok(())
    }

    /// Build lfs_config for use with littlefs2-sys.
    ///
    /// Caller must ensure storage outlives any lfs operations using this config.
    /// Buffers are allocated; with malloc feature, read/prog/lookahead_buffer can be null.
    pub fn build_lfs_config(
        &self,
        config: &Config,
        read_buf: Option<&mut [u8]>,
        prog_buf: Option<&mut [u8]>,
        lookahead_buf: Option<&mut [u8]>,
    ) -> littlefs2_sys::lfs_config {
        use std::os::raw::c_void;

        let context = self as *const AlignStorage as *mut c_void;

        let cfg = littlefs2_sys::lfs_config {
            context,
            read: Some(Self::c_read),
            prog: Some(Self::c_prog),
            erase: Some(Self::c_erase),
            sync: Some(Self::c_sync),
            read_size: config.read_size as littlefs2_sys::lfs_size_t,
            prog_size: config.prog_size as littlefs2_sys::lfs_size_t,
            block_size: config.block_size as littlefs2_sys::lfs_size_t,
            block_count: config.block_count as littlefs2_sys::lfs_size_t,
            block_cycles: config.block_cycles,
            cache_size: config.cache_size as littlefs2_sys::lfs_size_t,
            lookahead_size: config.lookahead_size as littlefs2_sys::lfs_size_t,
            compact_thresh: 0,
            read_buffer: read_buf
                .map(|b| b.as_mut_ptr() as *mut c_void)
                .unwrap_or(std::ptr::null_mut()),
            prog_buffer: prog_buf
                .map(|b| b.as_mut_ptr() as *mut c_void)
                .unwrap_or(std::ptr::null_mut()),
            lookahead_buffer: lookahead_buf
                .map(|b| b.as_mut_ptr() as *mut c_void)
                .unwrap_or(std::ptr::null_mut()),
            name_max: 255,
            file_max: 0,
            attr_max: 0,
            metadata_max: config.metadata_max as littlefs2_sys::lfs_size_t,
            inline_max: config.inline_max as littlefs2_sys::lfs_size_t,
        };

        cfg
    }

    unsafe extern "C" fn c_read(
        c: *const littlefs2_sys::lfs_config,
        block: littlefs2_sys::lfs_block_t,
        off: littlefs2_sys::lfs_off_t,
        buffer: *mut c_void,
        size: littlefs2_sys::lfs_size_t,
    ) -> std::os::raw::c_int {
        let storage = &*(*c).context as *const _ as *const AlignStorage;
        let buf = std::slice::from_raw_parts_mut(buffer as *mut u8, size as usize);
        match (*storage).read_impl(block, off, buf) {
            Ok(()) => 0,
            Err(_) => -5, // LFS_ERR_IO
        }
    }

    unsafe extern "C" fn c_prog(
        c: *const littlefs2_sys::lfs_config,
        block: littlefs2_sys::lfs_block_t,
        off: littlefs2_sys::lfs_off_t,
        buffer: *const c_void,
        size: littlefs2_sys::lfs_size_t,
    ) -> std::os::raw::c_int {
        let storage = &*(*c).context as *const _ as *const AlignStorage;
        let buf = std::slice::from_raw_parts(buffer as *const u8, size as usize);
        match (*storage).prog_impl(block, off, buf) {
            Ok(()) => 0,
            Err(_) => -5, // LFS_ERR_IO
        }
    }

    unsafe extern "C" fn c_erase(
        c: *const littlefs2_sys::lfs_config,
        block: littlefs2_sys::lfs_block_t,
    ) -> std::os::raw::c_int {
        let storage = &*(*c).context as *const _ as *const AlignStorage;
        match (*storage).erase_impl(block) {
            Ok(()) => 0,
            Err(_) => -5, // LFS_ERR_IO
        }
    }

    unsafe extern "C" fn c_sync(c: *const littlefs2_sys::lfs_config) -> std::os::raw::c_int {
        let storage = &*(*c).context as *const _ as *const AlignStorage;
        match (*storage).sync_impl() {
            Ok(()) => 0,
            Err(_) => -5, // LFS_ERR_IO
        }
    }
}

impl BlockDevice for AlignStorage {
    fn read(&self, block: u32, off: u32, buffer: &mut [u8]) -> Result<(), Error> {
        self.read_impl(block, off, buffer)
    }

    fn prog(&self, block: u32, off: u32, data: &[u8]) -> Result<(), Error> {
        self.prog_impl(block, off, data)
    }

    fn erase(&self, block: u32) -> Result<(), Error> {
        self.erase_impl(block)
    }

    fn sync(&self) -> Result<(), Error> {
        self.sync_impl()
    }
}
