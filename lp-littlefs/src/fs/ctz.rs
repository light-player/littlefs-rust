//! CTZ skip-list operations for file block traversal.
//!
//! Per lfs.c lfs_ctz_index, lfs_ctz_find, lfs_ctz_extend. See SPEC.md "CTZ skip-lists" and DESIGN.md.

use crate::block::BlockDevice;
use crate::config::Config;
use crate::error::Error;

use super::alloc;

pub(super) const BLOCK_NULL: u32 = 0xffff_ffff;

/// Population count (number of set bits). Portable replacement for lfs_popc.
fn popc(a: u32) -> u32 {
    a.count_ones()
}

/// Smallest power of 2 >= a. For a=0 undefined; a>=1. Portable replacement for lfs_npw2.
fn npw2(a: u32) -> u32 {
    if a <= 1 {
        return a;
    }
    1u32 << (32 - (a - 1).leading_zeros())
}

/// Count trailing zeros. lfs_ctz(0) is undefined. Portable replacement for lfs_ctz.
fn ctz(a: u32) -> u32 {
    if a == 0 {
        return 32;
    }
    a.trailing_zeros()
}

/// Compute CTZ block index for logical offset.
///
/// Per lfs_ctz_index (lfs.c:2873). Given block_size and logical offset `size`,
/// returns the block index and updates `*off` to the offset within that block.
pub fn ctz_index(block_size: u32, off: &mut u64) -> u32 {
    let size = *off;
    let b = block_size as u64 - 2 * 4;
    let mut i = size / b;
    if i == 0 {
        return 0;
    }
    i = (size - 4 * (popc((i - 1) as u32) as u64 + 2)) / b;
    *off = size - b * i - 4 * (popc(i as u32) as u64);
    i as u32
}

/// Find the block and offset containing the byte at logical position `pos`.
///
/// Per lfs_ctz_find (lfs.c:2886). Traverses the CTZ skip-list from head to find
/// the block containing byte `pos`. Returns (block, off) where off is the
/// offset within that block.
pub fn ctz_find<B: BlockDevice>(
    bd: &B,
    config: &Config,
    head: u32,
    size: u64,
    pos: u64,
) -> Result<(u32, u32), Error> {
    if size == 0 {
        return Ok((BLOCK_NULL, 0));
    }

    let mut head = head;
    let mut current_off = size - 1;
    let mut target_off = pos;
    let mut current = ctz_index(config.block_size, &mut current_off);
    let target = ctz_index(config.block_size, &mut target_off);

    while current > target {
        let skip = (npw2(current - target + 1) - 1).min(ctz(current));
        let mut next_head = [0u8; 4];
        bd.read(head, 4 * skip, &mut next_head)?;
        head = u32::from_le_bytes(next_head);
        current -= 1 << skip;
    }

    Ok((head, target_off as u32))
}

/// Extend the CTZ skip-list by one block. Allocates, erases, and links the new block.
///
/// Per lfs.c lfs_ctz_extend. When size > 0, either copies the incomplete last block
/// or appends a new block with skip pointers.
pub fn ctz_extend<B: BlockDevice>(
    bd: &B,
    config: &Config,
    root: [u32; 2],
    lookahead: &mut alloc::Lookahead,
    head: u32,
    size: u64,
) -> Result<(u32, u32), Error> {
    let block_size = config.block_size;
    let nblock = alloc::alloc(bd, config, root, lookahead)?;
    bd.erase(nblock)?;

    if size == 0 {
        return Ok((nblock, 0));
    }

    let mut noff = size - 1;
    let index = ctz_index(config.block_size, &mut noff);
    let noff = noff + 1;

    if noff != block_size as u64 {
        for i in 0..noff {
            let mut data = [0u8; 1];
            bd.read(head, i as u32, &mut data)?;
            bd.prog(nblock, i as u32, &data)?;
        }
        return Ok((nblock, noff as u32));
    }

    let index = index + 1;
    let skips = ctz(index) + 1;
    let mut nhead = head;

    for i in 0..skips {
        let nhead_le = nhead.to_le_bytes();
        bd.prog(nblock, 4 * i, &nhead_le)?;
        if i != skips - 1 {
            let mut buf = [0u8; 4];
            bd.read(nhead, 4 * i, &mut buf)?;
            nhead = u32::from_le_bytes(buf);
        }
    }

    Ok((nblock, 4 * skips))
}

/// Traverse CTZ skip-list blocks, calling cb for each block.
/// Per lfs.c lfs_ctz_traverse.
pub fn ctz_traverse<B: BlockDevice, F>(
    bd: &B,
    config: &Config,
    head: u32,
    size: u64,
    mut cb: F,
) -> Result<(), Error>
where
    F: FnMut(u32) -> Result<(), Error>,
{
    if size == 0 {
        return Ok(());
    }

    let mut head = head;
    let mut current_off = size - 1;
    let mut index = ctz_index(config.block_size, &mut current_off);

    loop {
        cb(head)?;
        if index == 0 {
            return Ok(());
        }

        let count = (2 - (index & 1)) as usize;
        let mut heads = [0u32; 2];
        let mut buf = [0u8; 8];
        bd.read(head, 0, &mut buf[..count * 4])?;
        heads[0] = u32::from_le_bytes(buf[0..4].try_into().unwrap());
        if count > 1 {
            heads[1] = u32::from_le_bytes(buf[4..8].try_into().unwrap());
        }

        for head in heads.iter().take(count - 1) {
            cb(*head)?;
        }
        head = heads[count - 1];
        index -= count as u32;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::block::RamBlockDevice;
    use crate::config::Config;
    use crate::fs::alloc::Lookahead;
    use crate::fs::format;

    #[test]
    fn ctz_extend_empty_file() {
        let config = Config::default_for_tests(64);
        let bd = RamBlockDevice::new(config.block_size, config.block_count);
        format::format(&bd, &config).unwrap();

        let root = [0u32, 1];
        let mut lookahead = Lookahead::new(&config);
        lookahead.alloc_drop(config.block_count);

        let (block, off) = ctz_extend(&bd, &config, root, &mut lookahead, BLOCK_NULL, 0).unwrap();
        assert!(block >= 2);
        assert_eq!(off, 0);
    }

    #[test]
    fn ctz_index_empty() {
        let mut off = 0u64;
        let idx = ctz_index(128, &mut off);
        assert_eq!(idx, 0);
        assert_eq!(off, 0);
    }

    #[test]
    fn ctz_index_first_block() {
        let b = 128 - 8;
        for size in 1..=b {
            let mut off = size as u64;
            let idx = ctz_index(128, &mut off);
            assert_eq!(idx, 0, "size={} off={}", size, off);
            assert_eq!(off, size as u64, "size={}", size);
        }
    }

    #[test]
    fn ctz_index_second_block() {
        let b = 128 - 8;
        let mut off = (2 * b) as u64;
        let idx = ctz_index(128, &mut off);
        assert_eq!(idx, 1, "size={} b={} off={}", 2 * b, b, off);
        assert!(off <= b as u64, "off should be within block");
    }

    #[test]
    fn popc_values() {
        assert_eq!(popc(0), 0);
        assert_eq!(popc(1), 1);
        assert_eq!(popc(3), 2);
        assert_eq!(popc(0xffff_ffff), 32);
    }

    #[test]
    fn npw2_values() {
        assert_eq!(npw2(1), 1);
        assert_eq!(npw2(2), 2);
        assert_eq!(npw2(3), 4);
        assert_eq!(npw2(4), 4);
        assert_eq!(npw2(5), 8);
    }
}
