//! TestContext: env + Lfs, ready for format/mount. Single setup for unit tests.

use crate::test::ram::{make_config, RamStorage, BLOCK_SIZE};
use crate::{lfs_format, lfs_mount, lfs_unmount, Lfs, LfsConfig};
use core::mem::MaybeUninit;

const DEFAULT_BLOCK_COUNT: u32 = 128;

/// Env + Lfs in one. Owns RAM BD, config, buffers. Use new(), then format_fs/mount_fs.
pub struct TestContext {
    pub ram: RamStorage,
    pub config: LfsConfig,
    lfs: MaybeUninit<Lfs>,
    _read_buf: alloc::vec::Vec<u8>,
    _prog_buf: alloc::vec::Vec<u8>,
    _lookahead_buf: alloc::vec::Vec<u8>,
}

impl TestContext {
    /// Fresh env with block_count blocks. Ready for format.
    pub fn new(block_count: u32) -> Self {
        let block_size = BLOCK_SIZE;
        let ram = RamStorage::new(block_size, block_count);
        let read_buf = alloc::vec![0u8; block_size as usize];
        let prog_buf = alloc::vec![0u8; block_size as usize];
        let lookahead_buf = alloc::vec![0u8; block_size as usize];

        let mut config = make_config(block_count, &ram);
        config.read_buffer = read_buf.as_ptr() as *mut core::ffi::c_void;
        config.prog_buffer = prog_buf.as_ptr() as *mut core::ffi::c_void;
        config.lookahead_buffer = lookahead_buf.as_ptr() as *mut core::ffi::c_void;

        let mut ctx = Self {
            ram,
            config,
            lfs: MaybeUninit::zeroed(),
            _read_buf: read_buf,
            _prog_buf: prog_buf,
            _lookahead_buf: lookahead_buf,
        };
        ctx.config.context = &mut ctx.ram as *mut RamStorage as *mut core::ffi::c_void;
        ctx.config.read_buffer = ctx._read_buf.as_mut_ptr() as *mut core::ffi::c_void;
        ctx.config.prog_buffer = ctx._prog_buf.as_mut_ptr() as *mut core::ffi::c_void;
        ctx.config.lookahead_buffer = ctx._lookahead_buf.as_mut_ptr() as *mut core::ffi::c_void;
        ctx
    }

    /// Default block_count 128. Matches upstream.
    pub fn default_blocks() -> Self {
        Self::new(DEFAULT_BLOCK_COUNT)
    }

    pub fn config(&self) -> *const LfsConfig {
        &self.config as *const LfsConfig
    }

    pub fn lfs_mut(&mut self) -> *mut Lfs {
        self.lfs.as_mut_ptr() as *mut Lfs
    }

    /// Format the filesystem. Panics on error.
    pub fn format(&mut self) {
        let err = lfs_format(self.lfs_mut(), self.config());
        assert_eq!(err, 0, "lfs_format failed: {}", err);
    }

    /// Mount the filesystem. Panics on error.
    pub fn mount(&mut self) {
        let err = lfs_mount(self.lfs_mut(), self.config());
        assert_eq!(err, 0, "lfs_mount failed: {}", err);
    }

    /// Unmount. Panics on error.
    pub fn unmount(&mut self) {
        let err = lfs_unmount(self.lfs_mut());
        assert_eq!(err, 0, "lfs_unmount failed: {}", err);
    }
}
