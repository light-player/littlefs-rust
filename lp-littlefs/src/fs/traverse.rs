//! Filesystem traversal for block allocation.
//!
//! Per lfs_fs_traverse_ (lfs.c:4693-4794). Walks metadata pairs via the
//! threaded linked list (softtail chain) and marks used blocks.

use super::bdcache::BdContext;
use super::ctz;
use super::metadata;
use crate::block::BlockDevice;
use crate::error::Error;

/// Block address meaning "null" or unused.
pub const BLOCK_NULL: u32 = 0xffff_ffff;

fn pair_isnull(pair: [u32; 2]) -> bool {
    pair[0] == BLOCK_NULL || pair[1] == BLOCK_NULL
}

fn pair_issync(a: [u32; 2], b: [u32; 2]) -> bool {
    (a[0] == b[0] && a[1] == b[1]) || (a[0] == b[1] && a[1] == b[0])
}

/// Brent's algorithm cycle detection. Returns Err(Corrupt) if cycle detected.
fn tortoise_detectcycles(
    tortoise: &mut ([u32; 2], u32, u32),
    current: [u32; 2],
) -> Result<(), Error> {
    if pair_issync(current, tortoise.0) {
        return Err(Error::Corrupt);
    }
    if tortoise.1 == tortoise.2 {
        tortoise.0 = current;
        tortoise.1 = 0;
        tortoise.2 *= 2;
    }
    tortoise.1 += 1;
    Ok(())
}

/// Traverse all used blocks in the filesystem. Calls `cb` for each block.
///
/// When `include_orphans` is true, also traverses into DIRSTRUCT entries
/// (orphaned directories that may not be in the threaded list).
pub fn fs_traverse<B, F>(
    ctx: &BdContext<'_, B>,
    root: [u32; 2],
    include_orphans: bool,
    mut cb: F,
) -> Result<(), Error>
where
    B: BlockDevice,
    F: FnMut(u32) -> Result<(), Error>,
{
    let mut tail = root;
    let mut tortoise = (
        [BLOCK_NULL, BLOCK_NULL],
        1u32, // i
        1u32, // period
    );

    while !pair_isnull(tail) {
        tortoise_detectcycles(&mut tortoise, tail)?;

        cb(tail[0])?;
        cb(tail[1])?;

        let dir = metadata::fetch_metadata_pair(ctx, tail)?;

        for id in 0..dir.count {
            let info = match metadata::get_entry_info(&dir, id, 255) {
                Ok(i) => i,
                Err(Error::Noent) => continue,
                Err(e) => return Err(e),
            };
            if info.typ == crate::info::FileType::Dir {
                if !include_orphans {
                    continue;
                }
                let bytes = match metadata::get_struct(&dir, id) {
                    Ok(b) => b,
                    Err(Error::Noent) => continue,
                    Err(e) => return Err(e),
                };
                let b0 = u32::from_le_bytes(bytes[0..4].try_into().unwrap());
                let b1 = u32::from_le_bytes(bytes[4..8].try_into().unwrap());
                if !pair_isnull([b0, b1]) {
                    cb(b0)?;
                    cb(b1)?;
                }
            } else {
                let (inline_, head, size) = match metadata::get_file_struct(&dir, id) {
                    Ok(x) => x,
                    Err(Error::Noent) => continue,
                    Err(e) => return Err(e),
                };
                if !inline_ && head != 0xffff_ffff && size > 0 {
                    ctz::ctz_traverse(ctx, head, size, |block| {
                        cb(block)?;
                        Ok(())
                    })?;
                }
            }
        }

        tail = dir.tail;
    }

    Ok(())
}
