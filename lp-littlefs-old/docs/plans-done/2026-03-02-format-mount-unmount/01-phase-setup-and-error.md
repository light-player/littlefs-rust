# Phase 1: Crate setup and error type

## Scope of phase

- Add `alloc` feature to lp-littlefs
- Create `error.rs` with Error enum matching littlefs error codes
- Update `lib.rs` with module tree and re-exports

## Code organization reminders

- Place more abstract things and entry points first
- Keep related functionality grouped together
- One concept per file

## Implementation details

### 1. Update lp-littlefs/Cargo.toml

Add alloc feature (required; no optional alloc for now):

```toml
[features]
default = ["alloc"]
alloc = []
std = []
```

Add dev-dependencies for tests (std enabled in integration tests):

```toml
[dev-dependencies]
# None needed for phase 1
```

### 2. Create src/error.rs

```rust
//! Error types for lp_littlefs.
//!
//! Maps to littlefs error codes (lfs.h enum lfs_error).

#![cfg_attr(not(feature = "std"), no_std)]

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Error {
    /// No error
    Ok,
    /// Error during device operation (I/O)
    Io,
    /// Corrupted filesystem
    Corrupt,
    /// No directory entry
    Noent,
    /// Entry already exists
    Exist,
    /// Entry is not a directory
    NotDir,
    /// Entry is a directory
    IsDir,
    /// Directory is not empty
    NotEmpty,
    /// Bad file number
    Badf,
    /// File too large
    Fbig,
    /// Invalid parameter
    Inval,
    /// No space left on device
    Nospc,
    /// No more memory available
    Nomem,
    /// No data/attr available
    Noattr,
    /// File name too long
    Nametoolong,
}
```

For this plan we will use: `Io`, `Corrupt`, `Inval`, `Nomem`. Others are stubs for future API compatibility.

### 3. Update src/lib.rs

```rust
//! Pure Rust implementation of the LittleFS embedded filesystem.
//!
//! No C dependencies—avoids C compiler and cross-compilation issues on embedded targets.

#![no_std]

extern crate alloc;

mod error;
mod config;
mod block;
mod superblock;
mod fs;

pub use error::Error;
pub use config::Config;
pub use block::{BlockDevice, RamBlockDevice};
pub use fs::LittleFs;
```

Create stub modules so the crate compiles (minimal content for now):

- `config.rs`: `pub struct Config {}` or `pub struct Config { pub block_size: u32, pub block_count: u32 }` (minimal)
- `block/mod.rs`: `pub trait BlockDevice {}` (empty trait)
- `block/ram.rs`: `pub struct RamBlockDevice;` (minimal)
- `superblock.rs`: `pub const MAGIC: &[u8; 8] = b"littlefs";`
- `fs/mod.rs`: `pub struct LittleFs;`

Actually—for phase 1 we only add error and update lib. The stub modules would cause "unused" warnings. Better: only add modules we're creating in this phase. Create error.rs and update lib.rs. Add `mod config` etc. in later phases.

So for phase 1:
- Add error.rs
- Update lib.rs with `mod error` and `pub use error::Error`
- Add `extern crate alloc` and `alloc` feature gate
- Don't add config, block, superblock, fs yet—add those in their phases

Re-read the plan: Phase 1 is "Crate setup and error type". So we add alloc feature, error.rs, and update lib.rs. We need the crate to compile. If we add `mod config` without config.rs, it fails. So we either:
1. Add all stub files in phase 1 (config, block, superblock, fs as minimal stubs)
2. Or add only error and have lib.rs compile with just `mod error`

Option 2 is cleaner—each phase adds its modules. So lib.rs for phase 1:

```rust
#![no_std]

extern crate alloc;

mod error;

pub use error::Error;
```

Update the existing unit test to verify Error:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_variants() {
        assert_eq!(Error::Corrupt, Error::Corrupt);
    }
}
```

### 4. Verify alloc works

In lib.rs, we need `#![cfg(feature = "alloc")]` or similar? Actually `extern crate alloc` is part of core/alloc in no_std. The `alloc` crate is always available when we have the alloc feature—it's a standard library crate. We need to add it to Cargo.toml:

```toml
[features]
default = ["alloc"]
alloc = []
std = []
```

And in lib.rs for no_std with alloc:
```rust
#![no_std]

extern crate alloc;

mod error;
pub use error::Error;
```

The `alloc` crate is typically available in Rust's standard distribution. We need to ensure it's in [dependencies] when feature is enabled. Actually, `alloc` is a standard crate that's part of the Rust distribution—you don't add it to Cargo.toml, it's built-in. But for `no_std` you do `extern crate alloc` and it's automatically linked when you use alloc types. Let me check—in no_std projects you usually have `extern crate alloc` and that works. The feature flag would gate code that *uses* alloc (like Vec), not the crate itself.

So: we add `alloc` feature. When enabled, we'll use `alloc::vec::Vec` etc. The `alloc` crate is in the standard library, no need to add to dependencies.

## Validate

```bash
cd lp-littlefs && cargo build
```

Fix any warnings. The crate should compile with only `Error` exported.
