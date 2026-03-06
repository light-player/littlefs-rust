/// Filesystem configuration.
///
/// Only `block_size` and `block_count` are required; the rest have sensible
/// defaults. Create with [`Config::new`] and override fields as needed.
pub struct Config {
    /// Size of an erasable block in bytes.
    pub block_size: u32,
    /// Number of erasable blocks on the device.
    pub block_count: u32,
    /// Minimum read size in bytes. Defaults to 16.
    pub read_size: u32,
    /// Minimum program (write) size in bytes. Defaults to 16.
    pub prog_size: u32,
    /// Number of erase cycles before moving data to a new block.
    /// Set to `-1` to disable wear leveling.
    pub block_cycles: i32,
    /// Size of per-file caches in bytes. `0` (default) uses `block_size`.
    pub cache_size: u32,
    /// Size of the block allocator lookahead buffer in bytes.
    /// `0` (default) uses `block_size`. Must be a multiple of 8.
    pub lookahead_size: u32,
    /// Maximum file name length in bytes. Defaults to 255.
    pub name_max: u32,
    /// Maximum file size in bytes.
    pub file_max: u32,
    /// Maximum size of custom attributes in bytes.
    pub attr_max: u32,
}

impl Config {
    /// Create a configuration with the given block geometry and sensible
    /// defaults for everything else.
    pub fn new(block_size: u32, block_count: u32) -> Self {
        Self {
            block_size,
            block_count,
            read_size: 16,
            prog_size: 16,
            block_cycles: -1,
            cache_size: 0,
            lookahead_size: 0,
            name_max: 255,
            file_max: i32::MAX as u32,
            attr_max: 1022,
        }
    }

    pub(crate) fn resolve_cache_size(&self) -> u32 {
        if self.cache_size > 0 {
            self.cache_size
        } else {
            self.block_size
        }
    }

    pub(crate) fn resolve_lookahead_size(&self) -> u32 {
        if self.lookahead_size > 0 {
            self.lookahead_size
        } else {
            self.block_size
        }
    }
}
