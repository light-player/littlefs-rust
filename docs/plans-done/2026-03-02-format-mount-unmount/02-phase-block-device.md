# Phase 2: Block device abstraction

## Scope of phase

- Create `BlockDevice` trait with read, prog, erase, sync
- Create `RamBlockDevice` implementation
- Add `block` module to lib.rs

## Code organization reminders

- Place trait definition first in block/mod.rs, then re-exports
- RamBlockDevice in separate file for focus
- Helper/utility functions at bottom of files

## Implementation details

### 1. Create src/block/mod.rs

```rust
//! Block device abstraction for littlefs.
//!
//! Maps to lfs_config read/prog/erase/sync callbacks.

mod ram;

pub use ram::RamBlockDevice;

use crate::Error;

/// Block device interface for littlefs storage.
///
/// All operations use caller-provided buffers. Block indices and offsets
/// follow littlefs semantics: block_size units, prog_size alignment for writes.
pub trait BlockDevice {
    /// Read data from a block. Buffer length must be multiple of read_size.
    fn read(
        &self,
        block: u32,
        off: u32,
        buffer: &mut [u8],
    ) -> Result<(), Error>;

    /// Program data to a block. Block must be erased first. Data length
    /// must be multiple of prog_size.
    fn prog(
        &self,
        block: u32,
        off: u32,
        data: &[u8],
    ) -> Result<(), Error>;

    /// Erase a block. Must be called before programming.
    fn erase(&self, block: u32) -> Result<(), Error>;

    /// Sync any cached state to the device.
    fn sync(&self) -> Result<(), Error>;
}
```

### 2. Create src/block/ram.rs

RamBlockDevice needs Config or geometry to know block_size, block_count. But Config is in phase 4. For phase 2 we have two options:
- Pass geometry (block_size, block_count) into RamBlockDevice::new
- Or define a minimal Geometry struct in block/ram.rs for now

To keep phase 2 self-contained, have RamBlockDevice take explicit parameters:

```rust
//! In-memory block device for testing.

use super::BlockDevice;
use crate::Error;
use alloc::vec::Vec;

/// RAM-backed block device.
///
/// Used for tests. Storage is (block_count * block_size) bytes.
pub struct RamBlockDevice {
    storage: Vec<u8>,
    block_size: u32,
    block_count: u32,
}

impl RamBlockDevice {
    /// Create a new RAM block device.
    ///
    /// `block_size` and `block_count` define the geometry. Total size is
    /// block_size * block_count bytes.
    pub fn new(block_size: u32, block_count: u32) -> Self {
        let size = (block_size as usize) * (block_count as usize);
        Self {
            storage: alloc::vec![0; size],
            block_size,
            block_count,
        }
    }
}

impl BlockDevice for RamBlockDevice {
    fn read(
        &self,
        block: u32,
        off: u32,
        buffer: &mut [u8],
    ) -> Result<(), Error> {
        if block >= self.block_count {
            return Err(Error::Inval);
        }
        let start = (block as usize) * (self.block_size as usize) + (off as usize);
        let end = start + buffer.len();
        if end > self.storage.len() {
            return Err(Error::Inval);
        }
        buffer.copy_from_slice(&self.storage[start..end]);
        Ok(())
    }

    fn prog(
        &self,
        block: u32,
        off: u32,
        data: &[u8],
    ) -> Result<(), Error> {
        if block >= self.block_count {
            return Err(Error::Inval);
        }
        let start = (block as usize) * (self.block_size as usize) + (off as usize);
        let end = start + data.len();
        if end > self.storage.len() {
            return Err(Error::Inval);
        }
        // In RAM we can "prog" by copying. Real flash would require erase first.
        self.storage[start..end].copy_from_slice(data);
        Ok(())
    }

    fn erase(&self, block: u32) -> Result<(), Error> {
        if block >= self.block_count {
            return Err(Error::Inval);
        }
        let start = (block as usize) * (self.block_size as usize);
        let end = start + (self.block_size as usize);
        // In RAM, "erase" = fill with 0xff to match flash semantics
        for b in &mut self.storage[start..end] {
            *b = 0xff;
        }
        Ok(())
    }

    fn sync(&self) -> Result<(), Error> {
        Ok(())
    }
}
```

Note: RamBlockDevice allows prog without prior erase (RAM has no erase-before-write requirement). For format we will erase first per SPEC.

### 3. Update src/lib.rs

Add block module and re-exports:

```rust
#![no_std]

extern crate alloc;

mod error;
mod block;

pub use error::Error;
pub use block::{BlockDevice, RamBlockDevice};
```

## Validate

```bash
cd lp-littlefs && cargo build
```

Fix any warnings.

Wait—prog and erase take `&self` but we need to mutate storage. RamBlockDevice needs `UnsafeCell` or we use `RefCell` (std) or we use `&mut self` for the trait. But the trait is `&self` to match read-only access for sync. The BlockDevice trait is typically called with mutable access to the device when it needs to write. In littlefs, the config has a context pointer—the read/prog/erase functions receive the config and can access mutable state through context.

So BlockDevice with `&self` assumes interior mutability. For no_std we have `UnsafeCell`. Or we could use `&mut self` for prog and erase—but that would require the filesystem to hold `&mut dyn BlockDevice` which is awkward. The usual pattern is: the block device holds its own storage and the trait uses `&self`; the storage is behind `UnsafeCell` or in a `RefCell` (if std). For no_std, `UnsafeCell<Vec<u8>>`.

Let me update:

```rust
use core::cell::UnsafeCell;

pub struct RamBlockDevice {
    storage: UnsafeCell<Vec<u8>>,
    block_size: u32,
    block_count: u32,
}
```

Then in prog/erase we need unsafe to get mutable ref. That works.

Or—simpler for a test-only device—we could use `RefCell` but that requires std. For no_std, `UnsafeCell` is the way.

Actually, `Vec` itself requires alloc. So we're good. Use `UnsafeCell<Vec<u8>>` and in prog/erase we do:
```rust
let storage = unsafe { &mut *self.storage.get() };
storage[start..end].copy_from_slice(data);
```

We need to ensure we don't implement Sync for RamBlockDevice if we use UnsafeCell in a way that could cause data races. Actually, for single-threaded tests we're fine. Let me add `impl !Sync for RamBlockDevice` if needed—or we could just not implement Sync. By default a struct doesn't implement Sync unless it's derived. UnsafeCell<Vec<u8>> - Vec is Send, UnsafeCell isn't Sync by default. Good.

One more thing: in `erase` we're taking `&self` but we need to mutate. So we need the UnsafeCell for the storage. And in `prog` we need to mutate too. So the UnsafeCell approach is correct.
</think>
Updating the ram.rs section to use UnsafeCell:
<｜tool▁calls▁begin｜><｜tool▁call▁begin｜>
StrReplace