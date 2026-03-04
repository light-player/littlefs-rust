//! SuperblockSnapshot: capture and dump block content for debugging.

#[cfg(test)]
extern crate std;

use crate::test::ram::MAGIC_OFFSET;
use crate::LfsConfig;

/// Captured superblock blocks. Use dump() to pretty-print.
pub struct SuperblockSnapshot {
    pub block0: alloc::vec::Vec<u8>,
    pub block1: alloc::vec::Vec<u8>,
    pub root_pair: [u32; 2],
}

impl SuperblockSnapshot {
    /// Read blocks 0 and 1 from config. root_pair from mounted Lfs.
    pub fn capture(config: *const LfsConfig, root_pair: [u32; 2]) -> Result<Self, i32> {
        let block_size = unsafe { (*config).block_size } as usize;
        let mut block0 = alloc::vec![0u8; block_size];
        let mut block1 = alloc::vec![0u8; block_size];

        let err0 = unsafe {
            let read = (*config).read.expect("read callback");
            read(config, 0, 0, block0.as_mut_ptr(), block_size as u32)
        };
        let err1 = unsafe {
            let read = (*config).read.expect("read callback");
            read(config, 1, 0, block1.as_mut_ptr(), block_size as u32)
        };
        if err0 != 0 {
            return Err(err0);
        }
        if err1 != 0 {
            return Err(err1);
        }

        Ok(Self {
            block0,
            block1,
            root_pair,
        })
    }

    /// Pretty-print blocks to stderr. For debug output.
    pub fn dump(&self, len: usize) {
        let len = len.min(self.block0.len()).min(self.block1.len());
        std::eprintln!("root.pair = {:?}", self.root_pair);
        std::eprintln!("block 0 (first {} bytes):", len);
        dump_block_hex(&self.block0[..len]);
        std::eprintln!("block 1 (first {} bytes):", len);
        dump_block_hex(&self.block1[..len]);
        let o = MAGIC_OFFSET as usize;
        let mag0 = self.block0.get(o..o + 8).map(|s| s as &[u8]).unwrap_or(&[]);
        let mag1 = self.block1.get(o..o + 8).map(|s| s as &[u8]).unwrap_or(&[]);
        std::eprintln!("bytes 12..20 block0: {:?}", mag0);
        std::eprintln!("bytes 12..20 block1: {:?}", mag1);
    }
}

fn dump_block_hex(block: &[u8]) {
    for (i, chunk) in block.chunks(16).enumerate() {
        let hex: alloc::string::String = chunk
            .iter()
            .map(|b| alloc::format!("{:02x}", b))
            .collect::<alloc::vec::Vec<_>>()
            .join(" ");
        let ascii: alloc::string::String = chunk
            .iter()
            .map(|&b| {
                if b.is_ascii_graphic() || b == b' ' {
                    b as char
                } else {
                    '.'
                }
            })
            .collect();
        std::eprintln!("  {:04x}  {}  |{}|", i * 16, hex, ascii);
    }
}
