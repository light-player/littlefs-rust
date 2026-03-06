//! Shared storage and config builders for C ↔ Rust compat tests.

use core::cell::UnsafeCell;
use std::os::raw::c_void;

/// Filesystem geometry for tests.
pub struct TestGeometry {
    pub block_size: u32,
    pub block_count: u32,
    pub read_size: u32,
    pub prog_size: u32,
    pub cache_size: u32,
    pub lookahead_size: u32,
}

impl Default for TestGeometry {
    fn default() -> Self {
        Self {
            block_size: 512,
            block_count: 128,
            read_size: 16,
            prog_size: 16,
            cache_size: 512,
            lookahead_size: 512,
        }
    }
}

/// In-memory storage shared between C and Rust littlefs implementations.
///
/// Both sides operate on the same byte array via their respective configs.
/// Uses `UnsafeCell` for interior mutability (single-threaded test use only).
pub struct SharedStorage {
    data: UnsafeCell<Vec<u8>>,
    pub geo: TestGeometry,
}

// Safety: tests are single-threaded; no concurrent access to the storage.
unsafe impl Sync for SharedStorage {}

impl SharedStorage {
    pub fn new(geo: TestGeometry) -> Self {
        let size = (geo.block_size as usize) * (geo.block_count as usize);
        Self {
            data: UnsafeCell::new(vec![0u8; size]),
            geo,
        }
    }

    pub fn with_defaults() -> Self {
        Self::new(TestGeometry::default())
    }

    fn read_impl(&self, block: u32, off: u32, buffer: &mut [u8]) -> i32 {
        let storage = unsafe { &*self.data.get() };
        let start = (block as usize) * (self.geo.block_size as usize) + (off as usize);
        let end = start + buffer.len();
        if end > storage.len() {
            return -22;
        }
        buffer.copy_from_slice(&storage[start..end]);
        0
    }

    fn prog_impl(&self, block: u32, off: u32, data: &[u8]) -> i32 {
        let storage = unsafe { &mut *self.data.get() };
        let start = (block as usize) * (self.geo.block_size as usize) + (off as usize);
        let end = start + data.len();
        if end > storage.len() {
            return -22;
        }
        storage[start..end].copy_from_slice(data);
        0
    }

    fn erase_impl(&self, block: u32) -> i32 {
        let storage = unsafe { &mut *self.data.get() };
        let start = (block as usize) * (self.geo.block_size as usize);
        let end = start + (self.geo.block_size as usize);
        if end > storage.len() {
            return -22;
        }
        storage[start..end].fill(0xff);
        0
    }

    // ── C (littlefs2-sys) callbacks ─────────────────────────────────────

    unsafe extern "C" fn c_read(
        c: *const littlefs2_sys::lfs_config,
        block: littlefs2_sys::lfs_block_t,
        off: littlefs2_sys::lfs_off_t,
        buffer: *mut c_void,
        size: littlefs2_sys::lfs_size_t,
    ) -> std::os::raw::c_int {
        let storage = &*((*c).context as *const SharedStorage);
        let buf = std::slice::from_raw_parts_mut(buffer as *mut u8, size as usize);
        storage.read_impl(block, off, buf)
    }

    unsafe extern "C" fn c_prog(
        c: *const littlefs2_sys::lfs_config,
        block: littlefs2_sys::lfs_block_t,
        off: littlefs2_sys::lfs_off_t,
        buffer: *const c_void,
        size: littlefs2_sys::lfs_size_t,
    ) -> std::os::raw::c_int {
        let storage = &*((*c).context as *const SharedStorage);
        let buf = std::slice::from_raw_parts(buffer as *const u8, size as usize);
        storage.prog_impl(block, off, buf)
    }

    unsafe extern "C" fn c_erase(
        c: *const littlefs2_sys::lfs_config,
        block: littlefs2_sys::lfs_block_t,
    ) -> std::os::raw::c_int {
        let storage = &*((*c).context as *const SharedStorage);
        storage.erase_impl(block)
    }

    unsafe extern "C" fn c_sync(_c: *const littlefs2_sys::lfs_config) -> std::os::raw::c_int {
        0
    }

    // ── Rust (littlefs-rust-core) callbacks ────────────────────────────────────

    unsafe extern "C" fn rust_read(
        c: *const littlefs_rust_core::LfsConfig,
        block: u32,
        off: u32,
        buffer: *mut u8,
        size: u32,
    ) -> i32 {
        let storage = &*((*c).context as *const SharedStorage);
        let buf = std::slice::from_raw_parts_mut(buffer, size as usize);
        storage.read_impl(block, off, buf)
    }

    unsafe extern "C" fn rust_prog(
        c: *const littlefs_rust_core::LfsConfig,
        block: u32,
        off: u32,
        buffer: *const u8,
        size: u32,
    ) -> i32 {
        let storage = &*((*c).context as *const SharedStorage);
        let buf = std::slice::from_raw_parts(buffer, size as usize);
        storage.prog_impl(block, off, buf)
    }

    unsafe extern "C" fn rust_erase(c: *const littlefs_rust_core::LfsConfig, block: u32) -> i32 {
        let storage = &*((*c).context as *const SharedStorage);
        storage.erase_impl(block)
    }

    unsafe extern "C" fn rust_sync(_c: *const littlefs_rust_core::LfsConfig) -> i32 {
        0
    }

    // ── Config builders ─────────────────────────────────────────────────

    /// Build a `littlefs2_sys::lfs_config` pointing at this storage.
    /// With the `malloc` feature, buffers are allocated internally (null pointers).
    pub fn build_c_config(&self) -> littlefs2_sys::lfs_config {
        littlefs2_sys::lfs_config {
            context: self as *const SharedStorage as *mut c_void,
            read: Some(Self::c_read),
            prog: Some(Self::c_prog),
            erase: Some(Self::c_erase),
            sync: Some(Self::c_sync),
            read_size: self.geo.read_size as littlefs2_sys::lfs_size_t,
            prog_size: self.geo.prog_size as littlefs2_sys::lfs_size_t,
            block_size: self.geo.block_size as littlefs2_sys::lfs_size_t,
            block_count: self.geo.block_count as littlefs2_sys::lfs_size_t,
            block_cycles: -1,
            cache_size: self.geo.cache_size as littlefs2_sys::lfs_size_t,
            lookahead_size: self.geo.lookahead_size as littlefs2_sys::lfs_size_t,
            compact_thresh: 0,
            read_buffer: std::ptr::null_mut(),
            prog_buffer: std::ptr::null_mut(),
            lookahead_buffer: std::ptr::null_mut(),
            name_max: 255,
            file_max: 0,
            attr_max: 0,
            metadata_max: 0,
            inline_max: 0,
        }
    }

    /// Build an `littlefs_rust_core::LfsConfig` with owned buffers pointing at this storage.
    pub fn build_rust_env(&self) -> RustEnv {
        let cache_sz = self.geo.cache_size as usize;
        let la_sz = self.geo.lookahead_size as usize;
        let mut read_buf = vec![0u8; cache_sz];
        let mut prog_buf = vec![0u8; cache_sz];
        let mut lookahead_buf = vec![0u8; la_sz];

        let config = littlefs_rust_core::LfsConfig {
            context: self as *const SharedStorage as *mut c_void,
            read: Some(Self::rust_read),
            prog: Some(Self::rust_prog),
            erase: Some(Self::rust_erase),
            sync: Some(Self::rust_sync),
            read_size: self.geo.read_size,
            prog_size: self.geo.prog_size,
            block_size: self.geo.block_size,
            block_count: self.geo.block_count,
            block_cycles: -1,
            cache_size: self.geo.cache_size,
            lookahead_size: self.geo.lookahead_size,
            compact_thresh: u32::MAX,
            read_buffer: read_buf.as_mut_ptr() as *mut c_void,
            prog_buffer: prog_buf.as_mut_ptr() as *mut c_void,
            lookahead_buffer: lookahead_buf.as_mut_ptr() as *mut c_void,
            name_max: 255,
            file_max: 2_147_483_647,
            attr_max: 1022,
            metadata_max: 0,
            inline_max: 0,
        };

        RustEnv {
            config,
            _read_buf: read_buf,
            _prog_buf: prog_buf,
            _lookahead_buf: lookahead_buf,
        }
    }
}

/// Owned config + buffers for the Rust (littlefs-rust-core) side.
/// Buffers must outlive any lfs_* calls using this config.
pub struct RustEnv {
    pub config: littlefs_rust_core::LfsConfig,
    _read_buf: Vec<u8>,
    _prog_buf: Vec<u8>,
    _lookahead_buf: Vec<u8>,
}

// ── Utilities ───────────────────────────────────────────────────────────

/// xorshift32 PRNG matching the upstream C test runner exactly.
pub fn test_prng(state: &mut u32) -> u32 {
    let mut x = *state;
    x ^= x << 13;
    x ^= x >> 17;
    x ^= x << 5;
    *state = x;
    x
}

/// Verify that `buf` matches PRNG data for the given seed.
pub fn prng_verify(buf: &[u8], seed: u32) {
    let mut prng = seed;
    for (i, &actual) in buf.iter().enumerate() {
        let expected = (test_prng(&mut prng) & 0xff) as u8;
        assert_eq!(
            actual, expected,
            "PRNG mismatch at byte {i}: expected {expected:#04x}, got {actual:#04x} (seed={seed})"
        );
    }
}

/// Null-terminated path bytes from a string.
pub fn path_cstr(s: &str) -> Vec<u8> {
    let mut v: Vec<u8> = s.bytes().collect();
    v.push(0);
    v
}

pub fn check(err: i32) -> Result<(), i32> {
    if err != 0 {
        Err(err)
    } else {
        Ok(())
    }
}
