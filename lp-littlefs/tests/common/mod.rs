//! Shared test helpers for lp-littlefs integration tests.
//!
//! Reduces duplication across test files. Provides RAM block device, config builder,
//! and assert helpers. Upstream reference: tests/test_superblocks.toml

#![allow(dead_code)]

use lp_littlefs::LfsConfig;

/// Initialize env_logger for tests that use logging. Idempotent.
pub fn init_logger() {
    let _ = env_logger::try_init();
}

/// Magic string "littlefs" in superblock blocks. Per lfs.h.
/// Layout: [rev 4][CREATE tag 4]["littlefs" 8] — C format_and_dump shows magic at 8
pub const MAGIC: &[u8; 8] = b"littlefs";
pub const MAGIC_OFFSET: u32 = 8;

/// RAM block device storage. Erase = 0xff; prog = copy; read = copy.
pub struct RamStorage {
    pub data: Vec<u8>,
    pub block_size: u32,
    pub block_count: u32,
}

impl RamStorage {
    pub fn new(block_size: u32, block_count: u32) -> Self {
        let size = (block_size as usize)
            .checked_mul(block_count as usize)
            .expect("overflow");
        Self {
            data: vec![0u8; size],
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

unsafe extern "C" fn ram_read(
    cfg: *const LfsConfig,
    block: u32,
    off: u32,
    buffer: *mut u8,
    size: u32,
) -> i32 {
    let ctx = (*cfg).context as *mut RamStorage;
    let ram = &mut *ctx;
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

/// Holds RAM storage, config, and buffers. Keeps pointers valid for lfs_* calls.
pub struct TestEnv {
    pub ram: RamStorage,
    pub config: LfsConfig,
    pub _read_buf: Vec<u8>,
    pub _prog_buf: Vec<u8>,
    pub _lookahead_buf: Vec<u8>,
}

/// Default block size. Matches upstream.
const BLOCK_SIZE: u32 = 512;

/// Build test environment with RAM BD. block_count defaults to 128 (upstream).
pub fn default_config(block_count: u32) -> TestEnv {
    let block_size = BLOCK_SIZE;
    let ram = RamStorage::new(block_size, block_count);
    let read_buf = vec![0u8; block_size as usize];
    let prog_buf = vec![0u8; block_size as usize];
    let lookahead_buf = vec![0u8; block_size as usize];

    let config = LfsConfig {
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
        compact_thresh: u32::MAX, // -1 in C
        read_buffer: read_buf.as_ptr() as *mut core::ffi::c_void,
        prog_buffer: prog_buf.as_ptr() as *mut core::ffi::c_void,
        lookahead_buffer: lookahead_buf.as_ptr() as *mut core::ffi::c_void,
        name_max: 255,
        file_max: 2_147_483_647,
        attr_max: 1022,
        metadata_max: 0,
        inline_max: 0,
    };

    // Build TestEnv, then set context/buffers. We must set context after returning
    // to the caller, since env moves and the stored &mut env.ram would otherwise
    // be a dangling pointer. Use TestEnv::init_context() after default_config().
    let mut env = TestEnv {
        ram,
        config,
        _read_buf: read_buf,
        _prog_buf: prog_buf,
        _lookahead_buf: lookahead_buf,
    };
    env.config.read_buffer = env._read_buf.as_mut_ptr() as *mut core::ffi::c_void;
    env.config.prog_buffer = env._prog_buf.as_mut_ptr() as *mut core::ffi::c_void;
    env.config.lookahead_buffer = env._lookahead_buf.as_mut_ptr() as *mut core::ffi::c_void;
    env
}

/// Call after default_config() to set context to ram. Required because context
/// must point to env.ram at its final address (after the env has been moved).
pub fn init_context(env: &mut TestEnv) {
    env.config.context = &mut env.ram as *mut RamStorage as *mut core::ffi::c_void;
}

/// Panic if result is not 0.
pub fn assert_ok(result: i32) {
    if result != 0 {
        panic!("expected 0, got {}", result);
    }
}

/// Panic if result is not 0, with step name for debugging.
pub fn assert_ok_at(step: &str, result: i32) {
    if result != 0 {
        panic!("{} failed: {} (expected 0)", step, result);
    }
}

/// Panic if actual is not expected error code.
pub fn assert_err(expected: i32, actual: i32) {
    if actual != expected {
        panic!("expected error {}, got {}", expected, actual);
    }
}

/// Check if block has "littlefs" at offset 8 or 12 (layout varies by commit path).
fn block_has_magic(config: *const LfsConfig, block: u32) -> bool {
    let mut buf = [0u8; 24];
    let err = unsafe {
        let read = (*config).read.expect("read callback");
        read(config, block, 0, buf.as_mut_ptr(), 24)
    };
    if err != 0 {
        return false;
    }
    &buf[8..16] == MAGIC || &buf[12..20] == MAGIC
}

/// Assert "littlefs" in blocks 0 and 1 (at offset 8 or 12 depending on commit path).
/// Both blocks must have magic per upstream.
pub fn assert_superblock_magic(config: *const LfsConfig) {
    let has_0 = block_has_magic(config, 0);
    let has_1 = block_has_magic(config, 1);
    assert!(
        has_0 && has_1,
        "both blocks 0 and 1 must have MAGIC: block 0={} block 1={}",
        has_0,
        has_1
    );
}

/// Invoke config read callback for raw block access (e.g. test_superblocks_magic).
pub fn read_block_raw(config: *const LfsConfig, block: u32, off: u32, buf: &mut [u8]) -> i32 {
    unsafe {
        let read = (*config).read.expect("read callback");
        read(config, block, off, buf.as_mut_ptr(), buf.len() as u32)
    }
}

/// Pretty-print first `len` bytes of block for inspection. Used when debugging layout.
pub fn dump_block_hex(block: &[u8], label: &str, len: usize) {
    let len = len.min(block.len());
    eprintln!("{} (first {} bytes):", label, len);
    for (i, chunk) in block[..len].chunks(16).enumerate() {
        let hex: String = chunk
            .iter()
            .map(|b| format!("{:02x}", b))
            .collect::<Vec<_>>()
            .join(" ");
        let ascii: String = chunk
            .iter()
            .map(|&b| {
                if b.is_ascii_graphic() || b == b' ' {
                    b as char
                } else {
                    '.'
                }
            })
            .collect();
        eprintln!("  {:04x}  {}  |{}|", i * 16, hex, ascii);
    }
}

/// Null-terminated path for lfs_* calls. Caller keeps vec in scope while using pointer.
pub fn path_bytes(s: &str) -> Vec<u8> {
    let mut v: Vec<u8> = s.bytes().collect();
    v.push(0);
    v
}

/// Read directory entry names (excluding "." and "..") from path. For use in dir tests.
pub fn dir_entry_names(
    lfs: *mut lp_littlefs::Lfs,
    _config: *const LfsConfig,
    path_str: &str,
) -> Result<Vec<String>, i32> {
    use lp_littlefs::{lfs_dir_close, lfs_dir_open, lfs_dir_read, LfsDir, LfsInfo};

    let path = path_bytes(path_str);
    let mut dir = core::mem::MaybeUninit::<LfsDir>::zeroed();
    let err = lfs_dir_open(lfs, dir.as_mut_ptr(), path.as_ptr());
    if err != 0 {
        return Err(err);
    }

    let mut names = Vec::new();
    let mut info = core::mem::MaybeUninit::<LfsInfo>::zeroed();
    loop {
        let n = lfs_dir_read(lfs, dir.as_mut_ptr(), info.as_mut_ptr());
        if n == 0 {
            break;
        }
        if n < 0 {
            let _ = lfs_dir_close(lfs, dir.as_mut_ptr());
            return Err(n);
        }
        let info_ref = unsafe { &*info.as_ptr() };
        let nul = info_ref.name.iter().position(|&b| b == 0).unwrap_or(256);
        let name = core::str::from_utf8(&info_ref.name[..nul])
            .unwrap_or("")
            .to_string();
        if name != "." && name != ".." {
            names.push(name);
        }
    }
    let _ = lfs_dir_close(lfs, dir.as_mut_ptr());
    Ok(names)
}

/// Open flags for lfs_file_open. Per lfs.h LFS_O_*.
pub const LFS_O_RDONLY: i32 = 1;
pub const LFS_O_WRONLY: i32 = 2;
pub const LFS_O_RDWR: i32 = 3;
pub const LFS_O_CREAT: i32 = 0x0100;
pub const LFS_O_EXCL: i32 = 0x0200;
pub const LFS_O_TRUNC: i32 = 0x0400;
pub const LFS_O_APPEND: i32 = 0x0800;

/// Format, mount, create "hello" file with "Hello World!\0", unmount.
/// Returns env. Caller mounts again before reading.
pub fn fs_with_hello(env: &mut TestEnv) -> Result<(), i32> {
    use lp_littlefs::{
        lfs_file_close, lfs_file_open, lfs_file_write, lfs_format, lfs_mount, lfs_unmount, Lfs,
        LfsConfig, LfsFile,
    };

    init_context(env);
    let mut lfs = core::mem::MaybeUninit::<Lfs>::zeroed();
    let err = lfs_format(lfs.as_mut_ptr(), &env.config as *const LfsConfig);
    if err != 0 {
        return Err(err);
    }
    let err = lfs_mount(lfs.as_mut_ptr(), &env.config as *const LfsConfig);
    if err != 0 {
        return Err(err);
    }

    let path = path_bytes("hello");
    let data = b"Hello World!\0";
    let mut file = core::mem::MaybeUninit::<LfsFile>::zeroed();
    let err = lfs_file_open(
        lfs.as_mut_ptr(),
        file.as_mut_ptr(),
        path.as_ptr(),
        0x0100 | 2,
    );
    if err != 0 {
        let _ = lfs_unmount(lfs.as_mut_ptr());
        return Err(err);
    }
    let n = lfs_file_write(
        lfs.as_mut_ptr(),
        file.as_mut_ptr(),
        data.as_ptr() as *const core::ffi::c_void,
        data.len() as u32,
    );
    if n != data.len() as i32 {
        let _ = lfs_file_close(lfs.as_mut_ptr(), file.as_mut_ptr());
        let _ = lfs_unmount(lfs.as_mut_ptr());
        return Err(if n < 0 { n } else { -1 });
    }
    let err = lfs_file_close(lfs.as_mut_ptr(), file.as_mut_ptr());
    if err != 0 {
        let _ = lfs_unmount(lfs.as_mut_ptr());
        return Err(err);
    }
    let err = lfs_unmount(lfs.as_mut_ptr());
    if err != 0 {
        return Err(err);
    }
    Ok(())
}

/// Format fs, sync, return raw content of superblock blocks 0 and 1.
/// Helper for debug tests. Caller must init_context before.
pub fn format_and_read_superblock_blocks(env: &mut TestEnv) -> Result<(Vec<u8>, Vec<u8>), i32> {
    use lp_littlefs::{lfs_format, Lfs};

    let mut lfs = core::mem::MaybeUninit::<Lfs>::zeroed();
    let err = lfs_format(lfs.as_mut_ptr(), &env.config as *const LfsConfig);
    if err != 0 {
        return Err(err);
    }

    let block_size = env.config.block_size as usize;
    let mut block0 = vec![0u8; block_size];
    let mut block1 = vec![0u8; block_size];
    let err0 = read_block_raw(&env.config as *const LfsConfig, 0, 0, &mut block0);
    let err1 = read_block_raw(&env.config as *const LfsConfig, 1, 0, &mut block1);
    if err0 != 0 || err1 != 0 {
        return Err(if err0 != 0 { err0 } else { err1 });
    }
    Ok((block0, block1))
}
