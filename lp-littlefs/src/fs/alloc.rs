//! Block allocation using lookahead bitmap.
//!
//! Per lfs_alloc, lfs_alloc_scan, lfs_alloc_lookahead (lfs.c:614-715).

use crate::block::BlockDevice;
use crate::config::Config;
use crate::error::Error;

use super::traverse;

/// Lookahead allocator state.
#[derive(Clone)]
pub struct Lookahead {
    pub start: u32,
    pub size: u32,
    pub next: u32,
    pub ckpoint: u32,
    buffer: alloc::vec::Vec<u8>,
}

impl Lookahead {
    pub fn new(config: &Config) -> Self {
        let lookahead_size = config.lookahead_size as usize;
        let buffer = config
            .lookahead_buffer
            .map(|s| s.to_vec())
            .unwrap_or_else(|| alloc::vec![0u8; lookahead_size]);
        Self {
            start: 0,
            size: 0,
            next: 0,
            ckpoint: config.block_count,
            buffer,
        }
    }

    pub fn alloc_ckpoint(&mut self, block_count: u32) {
        self.ckpoint = block_count;
    }

    pub fn alloc_drop(&mut self, block_count: u32) {
        self.size = 0;
        self.next = 0;
        self.alloc_ckpoint(block_count);
    }
}

/// Allocate a free block. Uses lookahead buffer and fs_traverse for scanning.
pub fn alloc<B: BlockDevice>(
    bd: &B,
    config: &Config,
    root: [u32; 2],
    lookahead: &mut Lookahead,
) -> Result<u32, Error> {
    let block_count = config.block_count;

    loop {
        while lookahead.next < lookahead.size {
            let byte_idx = (lookahead.next / 8) as usize;
            let bit_idx = lookahead.next % 8;
            if byte_idx >= lookahead.buffer.len() {
                break;
            }
            let used = (lookahead.buffer[byte_idx] & (1 << bit_idx)) != 0;
            if !used {
                let block = (lookahead.start + lookahead.next) % block_count;

                loop {
                    lookahead.next += 1;
                    lookahead.ckpoint = lookahead.ckpoint.saturating_sub(1);
                    if lookahead.next >= lookahead.size {
                        break;
                    }
                    let ni = (lookahead.next / 8) as usize;
                    let bi = lookahead.next % 8;
                    if ni >= lookahead.buffer.len() || (lookahead.buffer[ni] & (1 << bi)) == 0 {
                        break;
                    }
                }

                return Ok(block);
            }

            lookahead.next += 1;
            lookahead.ckpoint = lookahead.ckpoint.saturating_sub(1);
        }

        if lookahead.ckpoint == 0 {
            return Err(Error::Nospc);
        }

        alloc_scan(bd, config, root, lookahead)?;
    }
}

fn alloc_scan<B: BlockDevice>(
    bd: &B,
    config: &Config,
    root: [u32; 2],
    lookahead: &mut Lookahead,
) -> Result<(), Error> {
    let block_count = config.block_count;
    let lookahead_bits = config.lookahead_size.saturating_mul(8);

    lookahead.start = (lookahead.start + lookahead.next) % block_count;
    lookahead.next = 0;
    lookahead.size = lookahead_bits.min(lookahead.ckpoint);

    lookahead.buffer.fill(0);

    traverse::fs_traverse(bd, config, root, true, |block| {
        let off = (block
            .wrapping_sub(lookahead.start)
            .wrapping_add(block_count))
            % block_count;
        if off < lookahead.size {
            let byte_idx = (off / 8) as usize;
            let bit_idx = off % 8;
            if byte_idx < lookahead.buffer.len() {
                lookahead.buffer[byte_idx] |= 1 << bit_idx;
            }
        }
        Ok(())
    })?;

    Ok(())
}
