//! RAM block device for unit tests. Erase = 0xff; prog = copy; read = copy.

use crate::LfsConfig;

/// Magic string "littlefs" in superblock blocks. Per lfs.h.
pub const MAGIC: &[u8; 8] = b"littlefs";
/// Layout: [rev 4][CREATE tag 4][SUPERBLOCK tag 4]["littlefs" 8]
pub const MAGIC_OFFSET: u32 = 12;

/// RAM block device storage.
pub struct RamStorage {
    pub data: alloc::vec::Vec<u8>,
    pub block_size: u32,
    pub block_count: u32,
}

impl RamStorage {
    pub fn new(block_size: u32, block_count: u32) -> Self {
        let size = (block_size as usize)
            .checked_mul(block_count as usize)
            .expect("overflow");
        Self {
            data: alloc::vec![0u8; size],
            block_size,
            block_count,
        }
    }

    pub fn block_offset(&self, block: u32) -> usize {
        (block as usize)
            .checked_mul(self.block_size as usize)
            .expect("block overflow")
    }

    pub fn read(&mut self, block: u32, off: u32, buf: &mut [u8]) {
        let base = self.block_offset(block);
        let start = base + off as usize;
        let end = start + buf.len();
        buf.copy_from_slice(&self.data[start..end]);
    }

    pub fn prog(&mut self, block: u32, off: u32, buf: &[u8]) {
        let base = self.block_offset(block);
        let start = base + off as usize;
        let end = start + buf.len();
        self.data[start..end].copy_from_slice(buf);
    }

    pub fn erase(&mut self, block: u32) {
        let base = self.block_offset(block);
        let end = base + self.block_size as usize;
        self.data[base..end].fill(0xff);
    }
}

pub const BLOCK_SIZE: u32 = 512;

unsafe extern "C" fn ram_read(
    cfg: *const LfsConfig,
    block: u32,
    off: u32,
    buffer: *mut u8,
    size: u32,
) -> i32 {
    let ctx = (*cfg).context as *mut RamStorage;
    assert!(!ctx.is_null(), "ram_read: config.context is null");
    let ram = &mut *ctx;
    assert!(
        !ram.data.is_empty(),
        "ram_read: RamStorage.data is empty; config.context may be invalid"
    );
    let size = size as usize;
    let buf = core::slice::from_raw_parts_mut(buffer, size);
    ram.read(block, off, buf);
    0
}

unsafe extern "C" fn ram_prog(
    cfg: *const LfsConfig,
    block: u32,
    off: u32,
    buffer: *const u8,
    size: u32,
) -> i32 {
    let ctx = (*cfg).context as *mut RamStorage;
    let ram = &mut *ctx;
    let size = size as usize;
    let buf = core::slice::from_raw_parts(buffer, size);
    ram.prog(block, off, buf);
    0
}

unsafe extern "C" fn ram_erase(cfg: *const LfsConfig, block: u32) -> i32 {
    let ctx = (*cfg).context as *mut RamStorage;
    let ram = &mut *ctx;
    ram.erase(block);
    0
}

unsafe extern "C" fn ram_sync(_cfg: *const LfsConfig) -> i32 {
    0
}

/// Builds LfsConfig for the given RAM storage. Caller must set context after moving.
pub fn make_config(block_count: u32, ram: &RamStorage) -> LfsConfig {
    let block_size = BLOCK_SIZE;
    LfsConfig {
        context: core::ptr::null_mut(),
        read: Some(ram_read),
        prog: Some(ram_prog),
        erase: Some(ram_erase),
        sync: Some(ram_sync),
        read_size: 16,
        prog_size: 16,
        block_size,
        block_count,
        block_cycles: -1,
        cache_size: block_size,
        lookahead_size: block_size,
        compact_thresh: u32::MAX,
        read_buffer: core::ptr::null_mut(),
        prog_buffer: core::ptr::null_mut(),
        lookahead_buffer: core::ptr::null_mut(),
        name_max: 255,
        file_max: 2_147_483_647,
        attr_max: 1022,
        metadata_max: 0,
        inline_max: 0,
    }
}
