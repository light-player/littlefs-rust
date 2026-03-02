# Phase 6: Format implementation

## Scope of phase

Implement `format()` in `fs/format.rs`. Erase blocks 0 and 1, write initial superblock commit to the metadata pair. Wire into `LittleFs::format`.

## Code organization reminders

- Format logic in dedicated module for future feature-gating of alloc
- Tag encoding helpers at bottom
- Reference lfs.c lfs_format_ and lfs_dir_commit flow

## Implementation details

### 1. Tag encoding

Per lfs.c LFS_MKTAG: `(type << 20) | (id << 10) | size`. Tags stored big-endian in commits. First tag XOR'd with 0xffffffff (previous); subsequent tags XOR'd with previous tag.

Add `crc` crate (no_std) for CRC-32: `crc = "2.0"` in Cargo.toml. Polynomial 0x04c11db7, init 0xffffffff per SPEC.

### 2. Create src/fs/mod.rs (skeleton)

```rust
mod format;
mod mount;

use crate::config::Config;
use crate::error::Error;

pub struct LittleFs {
    _private: (),
}

impl LittleFs {
    pub fn new() -> Self {
        Self { _private: () }
    }

    pub fn format<B: crate::block::BlockDevice>(
        &mut self,
        _bd: &B,
        _config: &Config,
    ) -> Result<(), Error> {
        format::format(_bd, _config)
    }

    pub fn mount<B: crate::block::BlockDevice>(
        &mut self,
        _bd: &B,
        _config: &Config,
    ) -> Result<(), Error> {
        mount::mount(_bd, _config)
    }

    pub fn unmount(&mut self) -> Result<(), Error> {
        Ok(())
    }
}
```

### 3. Create src/fs/format.rs

- Erase blocks 0 and 1
- Build commit: revision (1, LE) + CREATE(0,0) + SUPERBLOCK(0,8) + "littlefs" + INLINESTRUCT(0,24) + superblock struct (LE) + CRC tag + CRC (LE)
- Tags: big-endian, XOR chain (first with 0xffffffff)
- Pad to prog_size alignment before CRC
- Write to block 0 (or 1—upstream uses pair; for first commit both blocks get the same? Actually lfs_dir_alloc allocates a pair, then commit writes. For format, root pair is 0,1. Commit writes to one of them—the one being written. This is complex.)

Simplified approach for minimal format: write one metadata block to block 0. Layout:
- [0..4]: revision = 1 (LE)
- [4..8]: tag CREATE (BE, xor 0xffffffff)
- [8..16]: tag SUPERBLOCK (BE, xor prev) + "littlefs"
- [16..40]: tag INLINESTRUCT (BE) + superblock 24 bytes (LE)
- ... padding to prog alignment ...
- CRC tag + CRC

Upstream uses lfs_dir_commit which handles prog in chunks, alignment, etc. For our minimal format we build the block in a buffer and prog it. Block size is 512. We need to fit: 4 + 4 + 4+8 + 4+24 + padding + 4+4. That's 4+4+12+28 = 48 + 8 (crc+tag) = 56. Plenty of room.

Note: upstream may write to only one block of the pair initially; the other is a backup. For minimal format we write to block 0. Mount will read from block 0 (or 1 if 0 fails—for now we only write to 0).

### 4. Commit layout (one block)

Offset 0: revision u32 LE
Offset 4: tag (4 bytes BE) — CREATE, xored with 0xffffffff
Offset 8: tag (4 bytes BE) — SUPERBLOCK, xored with previous tag
Offset 12: "littlefs" (8 bytes)
Offset 20: tag (4 bytes BE) — INLINESTRUCT
Offset 24: superblock (24 bytes LE)
Offset 48: pad to multiple of prog_size (e.g. 16) — 48 is already multiple of 16
Offset 48: tag (4 bytes BE) — TYPE_CRC
Offset 52: crc (4 bytes LE)

Wait, CRC tag. What's the CRC tag format? LFS_TYPE_CRC = 0x500, LFS_TYPE_CCRC. Let me check—the CRC sits at end of commit. The CRC is over the commit contents (revision through the data before CRC). Need to verify exact CRC scope from SPEC/upstream.

From SPEC: "each commit contains a variable number of metadata entries followed by a 32-bit CRC". So CRC is over the commit. The CRC tag might just mark the end. LFS_TYPE_CRC or LFS_TYPE_CCRC.

Looking at lfs_dir_commitcrc, it adds a CRC tag and the CRC value. The CRC is computed over... the commit. I'll need to check the exact algorithm. For phase 6, implement a working format that passes our tests. We can refine CRC scope if needed.

### 5. Superblock defaults

Use LFS_NAME_MAX=255, LFS_FILE_MAX=2147483647, LFS_ATTR_MAX=1022 from lfs.h for name_max, file_max, attr_max in superblock.

### 6. Update lib.rs

Add fs module and LittleFs, format, mount to exports.

## Validate

```bash
cd lp-littlefs && cargo build
```

Tests will run in phase 8; format must compile and be callable.
