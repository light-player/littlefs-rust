//! Configuration for littlefs.
//!
//! Maps to lfs_config (lfs.h).

/// Filesystem configuration (geometry and tuning).
///
/// Corresponds to struct lfs_config. Only fields needed for format/mount
/// are included for now.
#[derive(Clone, Debug)]
pub struct Config {
    /// Minimum read size in bytes (read alignment).
    pub read_size: u32,
    /// Minimum program size in bytes (prog alignment).
    pub prog_size: u32,
    /// Block size in bytes (usually == erase_size).
    pub block_size: u32,
    /// Number of blocks. 0 = read from disk (not yet supported).
    pub block_count: u32,
    /// Size of block caches in bytes. Must be a multiple of read_size and
    /// prog_size, and a factor of block_size.
    pub cache_size: u32,
    /// Optional statically allocated read buffer. Must be cache_size bytes.
    pub read_buffer: Option<&'static [u8]>,
    /// Optional statically allocated program buffer. Must be cache_size bytes.
    pub prog_buffer: Option<&'static [u8]>,
}

impl Config {
    /// Default geometry matching upstream "default".
    ///
    /// read=16, prog=16, block=512, block_count from argument.
    /// cache_size=max(64, max(read_size, prog_size)).
    /// For tests, use a small block_count (e.g. 128).
    pub fn default_for_tests(block_count: u32) -> Self {
        let read_size = 16;
        let prog_size = 16;
        let block_size = 512;
        let cache_size = 64.max(read_size.max(prog_size));
        Self {
            read_size,
            prog_size,
            block_size,
            block_count,
            cache_size,
            read_buffer: None,
            prog_buffer: None,
        }
    }
}
