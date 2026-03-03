//! Block device caching layer (FS-owned).
//!
//! Matches upstream lfs_bd_read, lfs_bd_prog, lfs_bd_flush, lfs_bd_sync behavior.
//! Caches live in MountState and are discarded on unmount.

use crate::block::BlockDevice;
use crate::config::Config;
use crate::error::Error;
use alloc::vec::Vec;
use core::cell::RefCell;

/// Invalid cache block (lfs_block_t -1).
pub(crate) const BLOCK_NULL: u32 = 0xffff_ffff;

fn aligndown(a: u32, alignment: u32) -> u32 {
    a - (a % alignment)
}

fn alignup(a: u32, alignment: u32) -> u32 {
    if alignment == 0 {
        return a;
    }
    aligndown(a + alignment - 1, alignment)
}

/// Read cache. Per upstream lfs_cache_t.
pub(crate) struct ReadCache {
    pub block: u32,
    pub off: u32,
    pub size: u32,
    pub buffer: Vec<u8>,
}

/// Program cache. Per upstream lfs_cache_t.
pub(crate) struct ProgCache {
    pub block: u32,
    pub off: u32,
    pub size: u32,
    pub buffer: Vec<u8>,
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

/// Create ReadCache for the given config.
pub fn new_read_cache(config: &Config) -> Result<ReadCache, Error> {
    let cache_size = config.cache_size;
    if cache_size == 0
        || !cache_size.is_multiple_of(config.read_size)
        || !cache_size.is_multiple_of(config.prog_size)
        || !config.block_size.is_multiple_of(cache_size)
    {
        return Err(Error::Inval);
    }
    Ok(ReadCache {
        block: BLOCK_NULL,
        off: 0,
        size: 0,
        buffer: alloc::vec![0xff; cache_size as usize],
    })
}

/// Create ProgCache for the given config.
pub fn new_prog_cache(config: &Config) -> Result<ProgCache, Error> {
    let cache_size = config.cache_size;
    if cache_size == 0
        || !cache_size.is_multiple_of(config.read_size)
        || !cache_size.is_multiple_of(config.prog_size)
        || !config.block_size.is_multiple_of(cache_size)
    {
        return Err(Error::Inval);
    }
    Ok(ProgCache {
        block: BLOCK_NULL,
        off: 0,
        size: 0,
        buffer: alloc::vec![0xff; cache_size as usize],
    })
}

/// Flush pcache to device, invalidate rcache for flushed block.
pub fn bd_flush<B: BlockDevice>(
    bd: &B,
    config: &Config,
    rcache: &RefCell<ReadCache>,
    pcache: &RefCell<ProgCache>,
) -> Result<(), Error> {
    let cache_size = config.cache_size;
    let prog_size = config.prog_size;

    let mut pcache_mut = pcache.borrow_mut();
    if pcache_mut.block == BLOCK_NULL {
        return Ok(());
    }

    crate::trace!(
        "cache flush block={} off={} size={}",
        pcache_mut.block,
        pcache_mut.off,
        pcache_mut.size
    );
    let flushed_block = pcache_mut.block;
    let diff = alignup(pcache_mut.size, prog_size);
    bd.prog(
        pcache_mut.block,
        pcache_mut.off,
        &pcache_mut.buffer[..diff as usize],
    )?;

    pcache_mut.zero(cache_size);
    drop(pcache_mut);

    let mut rcache_mut = rcache.borrow_mut();
    if rcache_mut.block == flushed_block {
        crate::trace!("cache flush invalidate rcache block={}", flushed_block);
        rcache_mut.drop_();
    }
    Ok(())
}

/// Drop rcache.
pub fn bd_drop_rcache(rcache: &RefCell<ReadCache>) {
    let mut r = rcache.borrow_mut();
    r.drop_();
}

/// Cached read. Checks pcache, rcache, then device.
pub fn bd_read<B: BlockDevice>(
    bd: &B,
    config: &Config,
    rcache: &RefCell<ReadCache>,
    pcache: &RefCell<ProgCache>,
    block: u32,
    off: u32,
    buffer: &mut [u8],
) -> Result<(), Error> {
    let block_size = config.block_size;
    let block_count = config.block_count;
    let read_size = config.read_size;
    let cache_size = config.cache_size;

    let mut size = buffer.len() as u32;
    crate::trace_cache!("cache read block={} off={} len={}", block, off, size);
    if off + size > block_size {
        return Err(Error::Corrupt);
    }
    if block_count != 0 && block >= block_count {
        return Err(Error::Corrupt);
    }

    let hint = size;
    let mut off = off;
    let mut pos = 0usize;

    while size > 0 {
        let mut diff = size;

        let pcache_ref = pcache.borrow();
        if pcache_ref.block != BLOCK_NULL
            && block == pcache_ref.block
            && off < pcache_ref.off + pcache_ref.size
        {
            if off >= pcache_ref.off {
                crate::trace_cache!("cache read HIT pcache block={}", block);
                let in_cache = (off - pcache_ref.off) as usize;
                let avail = pcache_ref.size as usize - in_cache;
                diff = diff.min(avail as u32);
                buffer[pos..][..diff as usize]
                    .copy_from_slice(&pcache_ref.buffer[in_cache..in_cache + diff as usize]);
                pos += diff as usize;
                off += diff;
                size -= diff;
                continue;
            }
            diff = diff.min(pcache_ref.off - off);
        }
        drop(pcache_ref);

        let rcache_ref = rcache.borrow();
        if rcache_ref.block == block && off < rcache_ref.off + rcache_ref.size {
            if off >= rcache_ref.off {
                crate::trace_cache!("cache read HIT rcache block={}", block);
                let in_cache = (off - rcache_ref.off) as usize;
                let avail = rcache_ref.size as usize - in_cache;
                diff = diff.min(avail as u32);
                buffer[pos..][..diff as usize]
                    .copy_from_slice(&rcache_ref.buffer[in_cache..in_cache + diff as usize]);
                pos += diff as usize;
                off += diff;
                size -= diff;
                continue;
            }
            diff = diff.min(rcache_ref.off - off);
        }
        drop(rcache_ref);

        if size >= hint && off.is_multiple_of(read_size) && size >= read_size {
            crate::trace_cache!("cache read MISS passthrough block={} off={}", block, off);
            diff = aligndown(diff, read_size);
            bd.read(block, off, &mut buffer[pos..][..diff as usize])?;
            pos += diff as usize;
            off += diff;
            size -= diff;
            continue;
        }

        let mut rcache_mut = rcache.borrow_mut();
        rcache_mut.block = block;
        rcache_mut.off = aligndown(off, read_size);
        let align_up = alignup(off + hint, read_size).min(block_size);
        let load_size = (align_up - rcache_mut.off).min(cache_size);
        rcache_mut.size = load_size;
        crate::trace!(
            "cache read MISS load block={} off={} size={}",
            block,
            rcache_mut.off,
            load_size
        );
        bd.read(
            rcache_mut.block,
            rcache_mut.off,
            &mut rcache_mut.buffer[..load_size as usize],
        )?;
    }
    Ok(())
}

/// Cached prog. Read-before-write for partial updates.
pub fn bd_prog<B: BlockDevice>(
    bd: &B,
    config: &Config,
    rcache: &RefCell<ReadCache>,
    pcache: &RefCell<ProgCache>,
    block: u32,
    off: u32,
    data: &[u8],
) -> Result<(), Error> {
    let block_size = config.block_size;
    let prog_size = config.prog_size;
    let cache_size = config.cache_size;

    crate::trace_cache!("cache prog block={} off={} len={}", block, off, data.len());
    let mut size = data.len() as u32;
    let mut off = off;
    let mut pos = 0usize;

    while size > 0 {
        let mut pcache_mut = pcache.borrow_mut();
        if pcache_mut.block == block && off >= pcache_mut.off && off < pcache_mut.off + cache_size {
            let in_cache = (off - pcache_mut.off) as usize;
            let avail = cache_size as usize - in_cache;
            let diff = size.min(avail as u32);
            pcache_mut.buffer[in_cache..in_cache + diff as usize]
                .copy_from_slice(&data[pos..][..diff as usize]);
            off += diff;
            pos += diff as usize;
            size -= diff;
            pcache_mut.size = pcache_mut.size.max(off - pcache_mut.off);
            if pcache_mut.size == cache_size {
                crate::trace_cache!("cache prog FULL flush pcache block={}", pcache_mut.block);
                drop(pcache_mut);
                bd_flush(bd, config, rcache, pcache)?;
                continue;
            }
            continue;
        }

        if pcache_mut.block != BLOCK_NULL {
            crate::trace_cache!(
                "cache prog EVICT flush pcache block={} (switching to block={})",
                pcache_mut.block,
                block
            );
            drop(pcache_mut);
            bd_flush(bd, config, rcache, pcache)?;
            continue;
        }

        crate::trace_cache!(
            "cache prog LOAD block={} off={}",
            block,
            aligndown(off, prog_size)
        );
        pcache_mut.block = block;
        pcache_mut.off = aligndown(off, prog_size);
        let load_size = (cache_size as usize).min((block_size - pcache_mut.off) as usize);
        bd.read(block, pcache_mut.off, &mut pcache_mut.buffer[..load_size])?;
        pcache_mut.size = load_size as u32;
        drop(pcache_mut);
    }
    Ok(())
}

/// Sync: drop rcache, flush pcache, call bd.sync().
pub fn bd_sync<B: BlockDevice>(
    bd: &B,
    config: &Config,
    rcache: &RefCell<ReadCache>,
    pcache: &RefCell<ProgCache>,
) -> Result<(), Error> {
    {
        let _pcache = pcache.borrow();
        crate::trace_cache!("bd_sync pcache_block={}", _pcache.block);
    }
    bd_drop_rcache(rcache);
    bd_flush(bd, config, rcache, pcache)?;
    bd.sync()
}

/// Block device + cache context. Threads caches through FS operations.
pub struct BdContext<'a, B: BlockDevice> {
    pub bd: &'a B,
    pub config: &'a Config,
    rcache: &'a RefCell<ReadCache>,
    pcache: &'a RefCell<ProgCache>,
}

impl<'a, B: BlockDevice> BdContext<'a, B> {
    pub(crate) fn new(
        bd: &'a B,
        config: &'a Config,
        rcache: &'a RefCell<ReadCache>,
        pcache: &'a RefCell<ProgCache>,
    ) -> Self {
        Self {
            bd,
            config,
            rcache,
            pcache,
        }
    }

    pub fn read(&self, block: u32, off: u32, buffer: &mut [u8]) -> Result<(), Error> {
        bd_read(
            self.bd,
            self.config,
            self.rcache,
            self.pcache,
            block,
            off,
            buffer,
        )
    }

    pub fn prog(&self, block: u32, off: u32, data: &[u8]) -> Result<(), Error> {
        bd_prog(
            self.bd,
            self.config,
            self.rcache,
            self.pcache,
            block,
            off,
            data,
        )
    }

    pub fn erase(&self, block: u32) -> Result<(), Error> {
        self.bd.erase(block)
    }

    pub fn sync(&self) -> Result<(), Error> {
        bd_sync(self.bd, self.config, self.rcache, self.pcache)
    }
}
