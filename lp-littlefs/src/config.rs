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
}

impl Config {
    /// Default geometry matching upstream "default".
    ///
    /// read=16, prog=16, block=512, block_count from argument.
    /// For tests, use a small block_count (e.g. 128).
    pub fn default_for_tests(block_count: u32) -> Self {
        Self {
            read_size: 16,
            prog_size: 16,
            block_size: 512,
            block_count,
        }
    }
}
