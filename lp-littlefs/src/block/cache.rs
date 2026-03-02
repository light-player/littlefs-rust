//! Block device caching layer.
//!
//! Matches upstream lfs_bd_read, lfs_bd_prog, lfs_bd_flush, lfs_bd_sync behavior.

use super::BlockDevice;
use crate::config::Config;
use crate::Error;
use alloc::vec::Vec;
use core::cell::RefCell;

/// Invalid cache block (lfs_block_t -1).
const BLOCK_NULL: u32 = 0xffff_ffff;

fn aligndown(a: u32, alignment: u32) -> u32 {
    a - (a % alignment)
}

fn alignup(a: u32, alignment: u32) -> u32 {
    if alignment == 0 {
        return a;
    }
    aligndown(a + alignment - 1, alignment)
}

struct ReadCache {
    block: u32,
    off: u32,
    size: u32,
    buffer: Vec<u8>,
}

struct ProgCache {
    block: u32,
    off: u32,
    size: u32,
    buffer: Vec<u8>,
}

impl ReadCache {
    fn drop_(&mut self) {
        self.block = BLOCK_NULL;
    }
}

impl ProgCache {
    fn zero(&mut self, cache_size: u32) {
        self.buffer[..cache_size as usize].fill(0xff);
        self.block = BLOCK_NULL;
    }
}

/// Block device wrapper that adds read and program caches.
///
/// All format/mount/dir/metadata operations should use this for cached I/O.
pub struct CachedBlockDevice<B: BlockDevice> {
    device: B,
    read_size: u32,
    prog_size: u32,
    block_size: u32,
    block_count: u32,
    cache_size: u32,
    rcache: RefCell<ReadCache>,
    pcache: RefCell<ProgCache>,
}

impl<B: BlockDevice> CachedBlockDevice<B> {
    /// Create a cached block device.
    ///
    /// Validates cache_size: must be multiple of read_size and prog_size,
    /// and a factor of block_size.
    pub fn new(device: B, config: &Config) -> Result<Self, Error> {
        let cache_size = config.cache_size;
        if cache_size == 0
            || !cache_size.is_multiple_of(config.read_size)
            || !cache_size.is_multiple_of(config.prog_size)
            || !config.block_size.is_multiple_of(cache_size)
        {
            return Err(Error::Inval);
        }

        let rcache = ReadCache {
            block: BLOCK_NULL,
            off: 0,
            size: 0,
            buffer: alloc::vec![0xff; cache_size as usize],
        };
        let pcache = ProgCache {
            block: BLOCK_NULL,
            off: 0,
            size: 0,
            buffer: alloc::vec![0xff; cache_size as usize],
        };

        Ok(Self {
            device,
            read_size: config.read_size,
            prog_size: config.prog_size,
            block_size: config.block_size,
            block_count: config.block_count,
            cache_size,
            rcache: RefCell::new(rcache),
            pcache: RefCell::new(pcache),
        })
    }

    fn flush(&self) -> Result<(), Error> {
        let mut pcache = self.pcache.borrow_mut();
        if pcache.block == BLOCK_NULL {
            return Ok(());
        }

        crate::trace!(
            "cache flush block={} off={} size={}",
            pcache.block,
            pcache.off,
            pcache.size
        );
        let flushed_block = pcache.block;
        let diff = alignup(pcache.size, self.prog_size);
        self.device
            .prog(pcache.block, pcache.off, &pcache.buffer[..diff as usize])?;

        pcache.zero(self.cache_size);
        drop(pcache);

        // Read cache for this block is now stale after prog.
        let mut rcache = self.rcache.borrow_mut();
        if rcache.block == flushed_block {
            crate::trace!("cache flush invalidate rcache block={}", flushed_block);
            rcache.drop_();
        }
        Ok(())
    }

    fn drop_rcache(&self) {
        let mut rcache = self.rcache.borrow_mut();
        rcache.drop_();
    }
}

impl<B: BlockDevice> BlockDevice for CachedBlockDevice<B> {
    fn read(&self, block: u32, off: u32, buffer: &mut [u8]) -> Result<(), Error> {
        let mut size = buffer.len() as u32;
        crate::trace!("cache read block={} off={} len={}", block, off, size);
        if off + size > self.block_size {
            return Err(Error::Corrupt);
        }
        if self.block_count != 0 && block >= self.block_count {
            return Err(Error::Corrupt);
        }

        let hint = size;
        let mut off = off;
        let mut pos = 0usize;

        while size > 0 {
            let mut diff = size;

            let pcache = self.pcache.borrow();
            if pcache.block != BLOCK_NULL && block == pcache.block && off < pcache.off + pcache.size
            {
                if off >= pcache.off {
                    crate::trace!("cache read HIT pcache block={}", block);
                    let in_cache = (off - pcache.off) as usize;
                    let avail = pcache.size as usize - in_cache;
                    diff = diff.min(avail as u32);
                    buffer[pos..][..diff as usize]
                        .copy_from_slice(&pcache.buffer[in_cache..in_cache + diff as usize]);
                    pos += diff as usize;
                    off += diff;
                    size -= diff;
                    continue;
                }
                diff = diff.min(pcache.off - off);
            }
            drop(pcache);

            let rcache = self.rcache.borrow();
            if rcache.block == block && off < rcache.off + rcache.size {
                if off >= rcache.off {
                    crate::trace!("cache read HIT rcache block={}", block);
                    let in_cache = (off - rcache.off) as usize;
                    let avail = rcache.size as usize - in_cache;
                    diff = diff.min(avail as u32);
                    buffer[pos..][..diff as usize]
                        .copy_from_slice(&rcache.buffer[in_cache..in_cache + diff as usize]);
                    pos += diff as usize;
                    off += diff;
                    size -= diff;
                    continue;
                }
                diff = diff.min(rcache.off - off);
            }
            drop(rcache);

            if size >= hint && off.is_multiple_of(self.read_size) && size >= self.read_size {
                crate::trace!("cache read MISS passthrough block={} off={}", block, off);
                diff = aligndown(diff, self.read_size);
                self.device
                    .read(block, off, &mut buffer[pos..][..diff as usize])?;
                pos += diff as usize;
                off += diff;
                size -= diff;
                continue;
            }

            let mut rcache = self.rcache.borrow_mut();
            rcache.block = block;
            rcache.off = aligndown(off, self.read_size);
            let align_up = alignup(off + hint, self.read_size).min(self.block_size);
            let load_size = (align_up - rcache.off).min(self.cache_size);
            rcache.size = load_size;
            crate::trace!(
                "cache read MISS load block={} off={} size={}",
                block,
                rcache.off,
                load_size
            );
            self.device.read(
                rcache.block,
                rcache.off,
                &mut rcache.buffer[..load_size as usize],
            )?;
        }
        Ok(())
    }

    fn prog(&self, block: u32, off: u32, data: &[u8]) -> Result<(), Error> {
        crate::trace!("cache prog block={} off={} len={}", block, off, data.len());
        let mut size = data.len() as u32;
        let mut off = off;
        let mut pos = 0usize;

        while size > 0 {
            let mut pcache = self.pcache.borrow_mut();
            if pcache.block == block && off >= pcache.off && off < pcache.off + self.cache_size {
                let in_cache = (off - pcache.off) as usize;
                let avail = self.cache_size as usize - in_cache;
                let diff = size.min(avail as u32);
                pcache.buffer[in_cache..in_cache + diff as usize]
                    .copy_from_slice(&data[pos..][..diff as usize]);
                off += diff;
                pos += diff as usize;
                size -= diff;
                pcache.size = pcache.size.max(off - pcache.off);
                if pcache.size == self.cache_size {
                    drop(pcache);
                    self.flush()?;
                    continue;
                }
                continue;
            }

            if pcache.block != BLOCK_NULL {
                drop(pcache);
                self.flush()?;
                continue;
            }

            // Load cache line: must read-before-write for partial updates
            // (e.g. appending to metadata block), per lfs_bd_prog behavior.
            pcache.block = block;
            pcache.off = aligndown(off, self.prog_size);
            let load_size = (self.cache_size as usize).min((self.block_size - pcache.off) as usize);
            self.device
                .read(block, pcache.off, &mut pcache.buffer[..load_size])?;
            pcache.size = load_size as u32;
            drop(pcache);
        }
        Ok(())
    }

    fn erase(&self, block: u32) -> Result<(), Error> {
        self.device.erase(block)
    }

    fn sync(&self) -> Result<(), Error> {
        crate::trace!("cache sync");
        self.drop_rcache();
        self.flush()?;
        self.device.sync()
    }
}
