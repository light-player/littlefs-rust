# Phase 4: Config and geometry

## Scope of phase

Create `config.rs` with `Config` struct holding geometry parameters (read_size, prog_size, block_size, block_count, etc.) and a helper for default test geometry.

## Code organization reminders

- Config struct and defaults first
- Helper for test geometry at bottom
- Keep config focused—no block device logic

## Implementation details

### 1. Create src/config.rs

Match upstream lfs_config fields needed for format/mount. Minimal set:

```rust
//! Configuration for littlefs.
//!
//! Maps to lfs_config (lfs.h).

#![cfg_attr(not(feature = "std"), no_std)]

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
    /// read=16, prog=16, block=512, block_count from erase_count.
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
```

### 2. Update src/lib.rs

```rust
mod config;
pub use config::Config;
```

## Validate

```bash
cd lp-littlefs && cargo build
```
