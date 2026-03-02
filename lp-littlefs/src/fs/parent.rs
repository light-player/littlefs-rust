//! Find predecessor and parent in the metadata chain.
//!
//! Per lfs_fs_pred (lfs.c:4796-4825), lfs_fs_parent (lfs.c:4856-4886).

use crate::block::BlockDevice;
use crate::config::Config;
use crate::error::Error;
use crate::info::FileType;

use super::metadata::{fetch_metadata_pair, get_entry_info, get_struct, MdDir};

const BLOCK_NULL: u32 = 0xffff_ffff;

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
        tortoise.2 = tortoise.2.saturating_mul(2).max(1);
    }
    tortoise.1 = tortoise.1.saturating_add(1);
    Ok(())
}

/// Find the metadata pair whose tail points to `pair`.
///
/// Walks the metadata chain from root via tail. Returns the MdDir whose
/// `tail == pair`, or `None` if no such pair exists (e.g. pair is root).
///
/// Per lfs_fs_pred (lfs.c:4796-4825).
pub fn fs_pred<B: BlockDevice>(
    bd: &B,
    config: &Config,
    root: [u32; 2],
    pair: [u32; 2],
) -> Result<Option<MdDir>, Error> {
    let mut tail = root;
    let mut tortoise = ([BLOCK_NULL, BLOCK_NULL], 1u32, 1u32);

    while !pair_isnull(tail) {
        tortoise_detectcycles(&mut tortoise, tail)?;

        let dir = fetch_metadata_pair(bd, config, tail)?;
        if pair_issync(dir.tail, pair) {
            return Ok(Some(dir));
        }
        tail = dir.tail;
    }

    Ok(None)
}

/// Find the parent metadata pair and id that has a DIRSTRUCT pointing to `pair`.
///
/// Walks the metadata chain from root. For each mdir, checks if any directory
/// entry has a DIRSTRUCT whose data (8-byte pair) equals `pair`. Returns
/// `Some((parent_mdir, tag_id))` if found, `None` otherwise.
///
/// Per lfs_fs_parent (lfs.c:4856-4886).
pub fn fs_parent<B: BlockDevice>(
    bd: &B,
    config: &Config,
    root: [u32; 2],
    pair: [u32; 2],
    name_max: u32,
) -> Result<Option<(MdDir, u16)>, Error> {
    let mut tail = root;
    let mut tortoise = ([BLOCK_NULL, BLOCK_NULL], 1u32, 1u32);

    while !pair_isnull(tail) {
        tortoise_detectcycles(&mut tortoise, tail)?;

        let dir = fetch_metadata_pair(bd, config, tail)?;

        for id in 0..dir.count {
            let info = match get_entry_info(&dir, id, name_max) {
                Ok(i) => i,
                Err(Error::Noent) => continue,
                Err(e) => return Err(e),
            };
            if info.typ != FileType::Dir {
                continue;
            }
            let bytes = match get_struct(&dir, id) {
                Ok(b) => b,
                Err(Error::Noent) => continue,
                Err(e) => return Err(e),
            };
            let child_pair = [
                u32::from_le_bytes(bytes[0..4].try_into().unwrap()),
                u32::from_le_bytes(bytes[4..8].try_into().unwrap()),
            ];
            if pair_issync(child_pair, pair) {
                return Ok(Some((dir, id)));
            }
        }

        tail = dir.tail;
    }

    Ok(None)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::block::RamBlockDevice;
    use crate::config::Config;
    use crate::fs::alloc::Lookahead;
    use crate::fs::commit;
    use crate::fs::format;

    fn formatted_bd() -> (RamBlockDevice, Config) {
        let config = Config::default_for_tests(128);
        let bd = RamBlockDevice::new(config.block_size, config.block_count);
        format::format(&bd, &config).unwrap();
        (bd, config)
    }

    #[test]
    fn fs_pred_root_has_no_predecessor() {
        let (bd, config) = formatted_bd();
        let r = fs_pred(&bd, &config, [0, 1], [0, 1]).unwrap();
        assert!(r.is_none());
    }

    #[test]
    fn fs_pred_finds_predecessor_after_mkdir() {
        let (bd, config) = formatted_bd();
        let root = [0u32, 1];
        let mut lookahead = Lookahead::new(&config);
        lookahead.alloc_drop(128);

        let mut new_dir = commit::dir_alloc(&bd, &config, root, &mut lookahead).unwrap();
        let mut root_mut = fetch_metadata_pair(&bd, &config, root).unwrap();

        let pred_tail = root_mut.tail;
        commit::dir_commit_append(
            &bd,
            &config,
            &mut new_dir,
            &[commit::CommitAttr::soft_tail(pred_tail)],
            &mut None,
        )
        .unwrap();

        let new_pair = new_dir.pair;
        let attrs = [
            commit::CommitAttr::create(1),
            commit::CommitAttr::name_dir(1, b"child"),
            commit::CommitAttr::dir_struct(1, new_pair),
            commit::CommitAttr::soft_tail(new_pair),
        ];
        commit::dir_commit_append(&bd, &config, &mut root_mut, &attrs, &mut None).unwrap();
        bd.sync().unwrap();

        let pred = fs_pred(&bd, &config, root, new_pair).unwrap();
        assert!(pred.is_some());
        let p = pred.unwrap();
        assert!(pair_issync(p.tail, new_pair));
    }

    #[test]
    fn fs_parent_finds_parent_after_mkdir() {
        let (bd, config) = formatted_bd();
        let root = [0u32, 1];
        let mut lookahead = Lookahead::new(&config);
        lookahead.alloc_drop(128);

        let mut new_dir = commit::dir_alloc(&bd, &config, root, &mut lookahead).unwrap();
        let mut root_mut = fetch_metadata_pair(&bd, &config, root).unwrap();

        let pred_tail = root_mut.tail;
        commit::dir_commit_append(
            &bd,
            &config,
            &mut new_dir,
            &[commit::CommitAttr::soft_tail(pred_tail)],
            &mut None,
        )
        .unwrap();

        let new_pair = new_dir.pair;
        let attrs = [
            commit::CommitAttr::create(1),
            commit::CommitAttr::name_dir(1, b"child"),
            commit::CommitAttr::dir_struct(1, new_pair),
            commit::CommitAttr::soft_tail(new_pair),
        ];
        commit::dir_commit_append(&bd, &config, &mut root_mut, &attrs, &mut None).unwrap();
        bd.sync().unwrap();

        let parent = fs_parent(&bd, &config, root, new_pair, 255).unwrap();
        assert!(parent.is_some());
        let (p, id) = parent.unwrap();
        assert_eq!(id, 1);
        assert!(pair_issync(p.pair, root));
    }
}
