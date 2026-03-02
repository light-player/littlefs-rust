# Phase 5: Superblock layout

## Scope of phase

Create `superblock.rs` with the on-disk superblock structure, tag constants, magic string, and layout details per SPEC.md. No format/mount logic yet—just the data structures and constants.

## Code organization reminders

- Constants and struct first
- Helper/utility at bottom
- Reference SPEC.md and lfs.h for layout

## Implementation details

### 1. Create src/superblock.rs

Per SPEC.md and lfs.h:

```rust
//! Superblock and metadata layout.
//!
//! Per SPEC.md and lfs.h. Tags are big-endian in commits.

#![cfg_attr(not(feature = "std"), no_std)]

/// Magic string at offset 8 in a valid superblock metadata block.
/// SPEC: "littlefs" (8 bytes).
pub const MAGIC: &[u8; 8] = b"littlefs";

/// Disk version 2.1 (0x00020001).
/// Matches LFS_DISK_VERSION in lfs.h.
pub const DISK_VERSION: u32 = 0x0002_0001;

/// Metadata tag types (lfs.h enum lfs_type).
/// Tags in commits are stored big-endian.
pub mod tag {
    /// Superblock name tag (id 0, size 8 for magic).
    pub const TYPE_SUPERBLOCK: u32 = 0x0ff;
    /// Inline struct tag (superblock struct).
    pub const TYPE_INLINESTRUCT: u32 = 0x201;
    /// Create entry (id 0 for superblock).
    pub const TYPE_CREATE: u32 = 0x401;
}

/// On-disk superblock struct (little-endian).
/// Matches lfs_superblock_t in lfs.h.
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct Superblock {
    pub version: u32,
    pub block_size: u32,
    pub block_count: u32,
    pub name_max: u32,
    pub file_max: u32,
    pub attr_max: u32,
}

impl Superblock {
    pub const SIZE: usize = 24; // 6 * 4
}

/// Revision count at start of metadata block (4 bytes, little-endian).
pub const REVISION_OFFSET: u32 = 0;

/// Magic is at offset 8 in block. SPEC: first tag after revision is
/// LFS_TYPE_SUPERBLOCK name tag; tag is 4 bytes, so data starts at 4.
/// Structure: [rev:4][tag:4][magic:8][tag:4][struct:24]...
/// So magic starts at byte 8 (0-indexed).
pub const MAGIC_OFFSET: u32 = 8;
```

Note: Full tag encoding (LFS_MKTAG with valid bit, type, id, length) is deferred to format.rs. Here we only need the constants for layout reference.

### 2. Update src/lib.rs

```rust
mod superblock;
// Superblock is internal for now; re-export MAGIC if tests need it
pub(crate) use superblock::{MAGIC, MAGIC_OFFSET, Superblock, DISK_VERSION};
```

Or make superblock pub for tests that read raw blocks:

```rust
pub use superblock::{MAGIC, MAGIC_OFFSET, Superblock, DISK_VERSION};
```

Tests (test_superblocks_magic) need to read raw block and assert magic at offset 8. So we need MAGIC and MAGIC_OFFSET public. Superblock can stay crate-internal if mount uses it internally.

## Validate

```bash
cd lp-littlefs && cargo build
```
