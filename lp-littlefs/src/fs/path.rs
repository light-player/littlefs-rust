//! Path resolution for littlefs.
//!
//! Per lfs_dir_find (lfs.c:1483–1590).

use crate::block::BlockDevice;
use crate::error::Error;
use crate::info::FileType;
use crate::trace;

use super::bdcache::BdContext;
use super::gstate::GState;
use super::metadata;

/// Find entry at path. Returns (MdDir, id) where id is 0x3ff for root.
/// Per lfs_dir_find (lfs.c:1483–1590).
pub fn dir_find<B: BlockDevice>(
    ctx: &BdContext<'_, B>,
    root: [u32; 2],
    path: &str,
    name_max: u32,
    gdisk: Option<&GState>,
) -> Result<(metadata::MdDir, u16), Error> {
    if path.is_empty() {
        return Err(Error::Inval);
    }

    let trimmed = path.trim_matches('/');
    let segments: alloc::vec::Vec<&str> = if trimmed.is_empty() {
        alloc::vec![]
    } else {
        trimmed.split('/').filter(|s| !s.is_empty()).collect()
    };

    if segments.is_empty() {
        let dir = metadata::fetch_metadata_pair(ctx, root)?;
        return Ok((dir, 0x3ff));
    }

    let mut stack: alloc::vec::Vec<(metadata::MdDir, u16)> = alloc::vec![];
    let mut cwd = metadata::fetch_metadata_pair(ctx, root)?;
    let mut tag_id: u16 = 0x3ff;
    let mut seg_idx = 0usize;

    while seg_idx < segments.len() {
        let seg = segments[seg_idx];

        if seg == "." {
            seg_idx += 1;
            continue;
        }

        if seg == ".." {
            if tag_id == 0x3ff {
                return Err(Error::Inval);
            }
            let cancel_count = find_dotdot_cancel_count(&segments[seg_idx + 1..]);
            if cancel_count > 0 {
                seg_idx += 1 + cancel_count;
                if seg_idx >= segments.len() {
                    if let Some((parent_dir, parent_id)) = stack.pop() {
                        cwd = parent_dir;
                        tag_id = parent_id;
                    }
                    break;
                }
                continue;
            }
            if stack.is_empty() {
                return Err(Error::Inval);
            }
            seg_idx += 1;
            let (parent_dir, parent_id) = stack.pop().unwrap();
            cwd = parent_dir;
            tag_id = parent_id;
            seg_idx += 1;
            continue;
        }

        let cancel_count = find_dotdot_cancel_count(&segments[seg_idx + 1..]);
        if cancel_count > 0 {
            seg_idx += 1 + cancel_count;
            if seg_idx >= segments.len() {
                break;
            }
            continue;
        }

        let found_id = find_name_in_dir(ctx, &mut cwd, seg, name_max, gdisk)?;
        let info = metadata::get_entry_info(&cwd, found_id, name_max, gdisk, false)?;

        if seg_idx + 1 >= segments.len() {
            tag_id = found_id;
            trace!(
                "dir_find path={:?} -> pair={:?} id={}",
                path,
                cwd.pair,
                found_id
            );
            break;
        }

        if info.typ != FileType::Dir {
            return Err(Error::NotDir);
        }

        stack.push((cwd.clone(), found_id));
        if found_id == 0x3ff {
            cwd = metadata::fetch_metadata_pair(ctx, root)?;
        } else {
            let pair = get_dir_struct(&cwd, found_id, gdisk)?;
            cwd = metadata::fetch_metadata_pair(ctx, pair)?;
        }
        tag_id = found_id;
        seg_idx += 1;
    }

    Ok((cwd, tag_id))
}

/// Find parent directory and insertion id for creating a new entry at path.
/// Returns (parent_dir, id, name) when the path does not exist and can be created.
/// Returns Err(Exist) when the final component already exists.
/// Returns Err(Noent) when a parent component does not exist.
/// C: lfs_dir_find + lfs_path_islast for create path.
pub fn dir_find_for_create<'a, B: BlockDevice>(
    ctx: &BdContext<'_, B>,
    root: [u32; 2],
    path: &'a str,
    name_max: u32,
    gdisk: Option<&GState>,
) -> Result<(metadata::MdDir, u16, &'a str), Error> {
    if path.is_empty() {
        return Err(Error::Inval);
    }

    let trimmed = path.trim_matches('/');
    let segments: alloc::vec::Vec<&str> = if trimmed.is_empty() {
        alloc::vec![]
    } else {
        trimmed.split('/').filter(|s| !s.is_empty()).collect()
    };

    if segments.is_empty() {
        return Err(Error::Inval);
    }

    let mut stack: alloc::vec::Vec<(metadata::MdDir, u16)> = alloc::vec![];
    let mut cwd = metadata::fetch_metadata_pair(ctx, root)?;
    let mut tag_id: u16 = 0x3ff;
    let mut seg_idx = 0usize;

    while seg_idx < segments.len() {
        let seg = segments[seg_idx];

        if seg == "." {
            seg_idx += 1;
            continue;
        }

        if seg == ".." {
            if tag_id == 0x3ff {
                return Err(Error::Inval);
            }
            let cancel_count = find_dotdot_cancel_count(&segments[seg_idx + 1..]);
            if cancel_count > 0 {
                seg_idx += 1 + cancel_count;
                if seg_idx >= segments.len() {
                    let _ = stack.pop();
                    return Err(Error::Inval);
                }
                continue;
            }
            if stack.is_empty() {
                return Err(Error::Inval);
            }
            seg_idx += 1;
            let (parent_dir, parent_id) = stack.pop().unwrap();
            cwd = parent_dir;
            tag_id = parent_id;
            seg_idx += 1;
            continue;
        }

        let cancel_count = find_dotdot_cancel_count(&segments[seg_idx + 1..]);
        if cancel_count > 0 {
            seg_idx += 1 + cancel_count;
            if seg_idx >= segments.len() {
                break;
            }
            continue;
        }

        let found_id = match find_name_in_dir(ctx, &mut cwd, seg, name_max, gdisk) {
            Ok(id) => id,
            Err(Error::Noent) if seg_idx + 1 >= segments.len() => {
                let id = find_insertion_id(&cwd, seg, name_max, gdisk)?;
                trace!(
                    "dir_find_for_create path={:?} -> insert id={} name={:?}",
                    path,
                    id,
                    seg
                );
                return Ok((cwd, id, seg));
            }
            Err(Error::Noent) => return Err(Error::Noent),
            Err(e) => return Err(e),
        };
        let info = metadata::get_entry_info(&cwd, found_id, name_max, gdisk, false)?;

        if seg_idx + 1 >= segments.len() {
            return Err(Error::Exist);
        }

        if info.typ != FileType::Dir {
            return Err(Error::NotDir);
        }

        stack.push((cwd.clone(), found_id));
        if found_id == 0x3ff {
            cwd = metadata::fetch_metadata_pair(ctx, root)?;
        } else {
            let pair = get_dir_struct(&cwd, found_id, gdisk)?;
            cwd = metadata::fetch_metadata_pair(ctx, pair)?;
        }
        tag_id = found_id;
        seg_idx += 1;
    }

    Err(Error::Noent)
}

/// Find the id slot where a new name would be inserted to maintain alphabetical order.
/// C: lfs_min(lfs_tag_id(besttag), dir->count) from lfs_dir_fetchmatch (lfs.c:1370).
fn find_insertion_id(
    dir: &metadata::MdDir,
    name: &str,
    name_max: u32,
    gdisk: Option<&GState>,
) -> Result<u16, Error> {
    let name_bytes = name.as_bytes();
    let start_id = if dir.pair[0] == 0 || dir.pair[0] == 1 {
        1
    } else {
        0
    };

    for id in start_id..dir.count {
        match metadata::get_entry_info(dir, id, name_max, gdisk, false) {
            Ok(info) => {
                let cmp = info.name_bytes().cmp(name_bytes);
                if cmp == core::cmp::Ordering::Greater || cmp == core::cmp::Ordering::Equal {
                    return Ok(id);
                }
            }
            Err(Error::Noent) => continue,
            Err(e) => return Err(e),
        }
    }
    Ok(dir.count)
}

/// If suffix cancels the current segment via "..", return the number of suffix segments to skip.
fn find_dotdot_cancel_count(remaining: &[&str]) -> usize {
    let mut depth = 1usize;
    for (i, seg) in remaining.iter().enumerate() {
        if *seg == ".." {
            if depth == 0 {
                return 0;
            }
            depth -= 1;
            if depth == 0 {
                return i + 1;
            }
        } else if *seg != "." {
            depth += 1;
        }
    }
    0
}

fn find_name_in_dir<B: BlockDevice>(
    ctx: &BdContext<'_, B>,
    dir: &mut metadata::MdDir,
    name: &str,
    name_max: u32,
    gdisk: Option<&GState>,
) -> Result<u16, Error> {
    let name_bytes = name.as_bytes();
    if name_bytes.len() > name_max as usize {
        return Err(Error::Nametoolong);
    }

    let start_id = if dir.pair[0] == 0 || dir.pair[0] == 1 {
        1
    } else {
        0
    };

    match find_name_in_dir_pair(dir, name_bytes, name_max, start_id, gdisk) {
        Ok(id) => Ok(id),
        Err(Error::Noent) if dir.split => {
            let mut next_dir = metadata::fetch_metadata_pair(ctx, dir.tail)?;
            match find_name_in_dir(ctx, &mut next_dir, name, name_max, gdisk) {
                Ok(id) => {
                    *dir = next_dir;
                    Ok(id)
                }
                Err(Error::Noent) => {
                    // Per C lfs_dir_fetchmatch: when not found, dir is left at the block we
                    // searched last (the tail). Callers like find_insertion_id need the tail's
                    // count for correct insertion ids when creating in a split dir.
                    *dir = next_dir;
                    Err(Error::Noent)
                }
                Err(e) => Err(e),
            }
        }
        other => other,
    }
}

/// Per lfs_dir_find_match (lfs.c:1453).
fn find_name_in_dir_pair(
    dir: &metadata::MdDir,
    name_bytes: &[u8],
    name_max: u32,
    start_id: u16,
    gdisk: Option<&GState>,
) -> Result<u16, Error> {
    for id in start_id..dir.count {
        match metadata::get_entry_info(dir, id, name_max, gdisk, false) {
            Ok(info) if info.name_bytes() == name_bytes => return Ok(id),
            Ok(_info) => {
                trace!(
                    "find_name_in_dir_pair pair={:?} id={} name={:?} (no match)",
                    dir.pair,
                    id,
                    _info.name_bytes()
                );
            }
            Err(Error::Noent) => {
                trace!(
                    "find_name_in_dir_pair pair={:?} id={} get_entry_info Noent",
                    dir.pair,
                    id
                );
                continue;
            }
            Err(e) => return Err(e),
        }
    }
    Err(Error::Noent)
}

fn get_dir_struct(
    dir: &metadata::MdDir,
    id: u16,
    gdisk: Option<&GState>,
) -> Result<[u32; 2], Error> {
    let bytes = metadata::get_struct(dir, id, gdisk)?;
    Ok([
        u32::from_le_bytes(bytes[0..4].try_into().unwrap()),
        u32::from_le_bytes(bytes[4..8].try_into().unwrap()),
    ])
}

/// True if new_path is a descendant of old_path (e.g. "a/b" under "a").
/// Prevents rename-dir-into-itself bug (littlefs#1162).
pub fn path_is_descendant(old_path: &str, new_path: &str) -> bool {
    let old_norm = old_path.trim_matches('/');
    let new_norm = new_path.trim_matches('/');
    if old_norm.is_empty() || new_norm.len() <= old_norm.len() {
        return false;
    }
    new_norm.starts_with(old_norm) && new_norm.as_bytes().get(old_norm.len()) == Some(&b'/')
}

/// Last path component. Returns None if path is "/" or empty.
pub fn path_last_component(path: &str) -> Option<&str> {
    let trimmed = path.trim_matches('/');
    if trimmed.is_empty() {
        return None;
    }
    trimmed.rsplit('/').next()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::block::RamBlockDevice;
    use crate::config::Config;
    use crate::fs::bdcache::{self, BdContext};
    use crate::fs::commit;
    use crate::fs::format;
    use core::cell::RefCell;

    fn formatted_bd() -> (RamBlockDevice, Config) {
        let config = Config::default_for_tests(128);
        let bd = RamBlockDevice::new(config.block_size, config.block_count);
        format::format(&bd, &config).unwrap();
        (bd, config)
    }

    #[test]
    fn dir_find_root() {
        let (bd, config) = formatted_bd();
        let rcache = RefCell::new(bdcache::new_read_cache(&config).unwrap());
        let pcache = RefCell::new(bdcache::new_prog_cache(&config).unwrap());
        let ctx = BdContext::new(&bd, &config, &rcache, &pcache);
        let (dir, id) = dir_find(&ctx, [0, 1], "/", 255, None).unwrap();
        assert_eq!(id, 0x3ff);
        assert_eq!(dir.pair, [0, 1]);
    }

    #[test]
    fn dir_find_single() {
        let (bd, config) = formatted_bd();
        let rcache = RefCell::new(bdcache::new_read_cache(&config).unwrap());
        let pcache = RefCell::new(bdcache::new_prog_cache(&config).unwrap());
        let ctx = BdContext::new(&bd, &config, &rcache, &pcache);
        let mut root = metadata::fetch_metadata_pair(&ctx, [0, 1]).unwrap();
        let d0_pair = [2u32, 3];
        let attrs = [
            commit::CommitAttr::create(1),
            commit::CommitAttr::name_dir(1, b"d0"),
            commit::CommitAttr::dir_struct(1, d0_pair),
            commit::CommitAttr::soft_tail(d0_pair),
        ];
        commit::dir_commit_append(
            &ctx,
            &mut root,
            &attrs,
            &mut None,
            crate::superblock::DISK_VERSION,
        )
        .unwrap();
        bdcache::bd_sync(&bd, &config, &rcache, &pcache).unwrap();

        let (dir, id) = dir_find(&ctx, [0, 1], "d0", 255, None).unwrap();
        assert_eq!(id, 1);
        assert_eq!(dir.pair, [0, 1]);
    }

    #[test]
    fn dir_find_nested() {
        let (bd, config) = formatted_bd();
        let rcache = RefCell::new(bdcache::new_read_cache(&config).unwrap());
        let pcache = RefCell::new(bdcache::new_prog_cache(&config).unwrap());
        let ctx = BdContext::new(&bd, &config, &rcache, &pcache);
        let mut root = metadata::fetch_metadata_pair(&ctx, [0, 1]).unwrap();
        let p_pair = [2u32, 3];
        let c_pair = [4u32, 5];
        let mkdir_p = [
            commit::CommitAttr::create(1),
            commit::CommitAttr::name_dir(1, b"p"),
            commit::CommitAttr::dir_struct(1, p_pair),
            commit::CommitAttr::soft_tail(p_pair),
        ];
        commit::dir_commit_append(
            &ctx,
            &mut root,
            &mkdir_p,
            &mut None,
            crate::superblock::DISK_VERSION,
        )
        .unwrap();
        bdcache::bd_sync(&bd, &config, &rcache, &pcache).unwrap();

        let block_size = config.block_size as usize;
        let mut p_dir = metadata::MdDir::alloc_empty(p_pair, block_size);
        let mkdir_c = [
            commit::CommitAttr::create(1),
            commit::CommitAttr::name_dir(1, b"c"),
            commit::CommitAttr::dir_struct(1, c_pair),
            commit::CommitAttr::soft_tail(c_pair),
        ];
        commit::dir_commit_append(
            &ctx,
            &mut p_dir,
            &mkdir_c,
            &mut None,
            crate::superblock::DISK_VERSION,
        )
        .unwrap();
        bdcache::bd_sync(&bd, &config, &rcache, &pcache).unwrap();

        let (dir, id) = dir_find(&ctx, [0, 1], "p/c", 255, None).unwrap();
        assert_eq!(id, 1);
        assert_eq!(dir.pair, p_pair);
    }

    #[test]
    fn dir_find_dotdot_cancel() {
        let (bd, config) = formatted_bd();
        let rcache = RefCell::new(bdcache::new_read_cache(&config).unwrap());
        let pcache = RefCell::new(bdcache::new_prog_cache(&config).unwrap());
        let ctx = BdContext::new(&bd, &config, &rcache, &pcache);
        let mut root = metadata::fetch_metadata_pair(&ctx, [0, 1]).unwrap();
        let a_pair = [2u32, 3];
        let b_pair = [4u32, 5];
        let attrs_a = [
            commit::CommitAttr::create(1),
            commit::CommitAttr::name_dir(1, b"a"),
            commit::CommitAttr::dir_struct(1, a_pair),
            commit::CommitAttr::soft_tail(a_pair),
        ];
        commit::dir_commit_append(
            &ctx,
            &mut root,
            &attrs_a,
            &mut None,
            crate::superblock::DISK_VERSION,
        )
        .unwrap();
        let mut root = metadata::fetch_metadata_pair(&ctx, [0, 1]).unwrap();
        let attrs_b = [
            commit::CommitAttr::create(2),
            commit::CommitAttr::name_dir(2, b"b"),
            commit::CommitAttr::dir_struct(2, b_pair),
            commit::CommitAttr::soft_tail(b_pair),
        ];
        commit::dir_commit_append(
            &ctx,
            &mut root,
            &attrs_b,
            &mut None,
            crate::superblock::DISK_VERSION,
        )
        .unwrap();
        bdcache::bd_sync(&bd, &config, &rcache, &pcache).unwrap();

        let (dir, id) = dir_find(&ctx, [0, 1], "a/../b", 255, None).unwrap();
        assert_eq!(id, 2);
        assert_eq!(dir.pair, [0, 1]);
    }

    #[test]
    fn dir_find_after_rename() {
        let (bd, config) = formatted_bd();
        let rcache = RefCell::new(bdcache::new_read_cache(&config).unwrap());
        let pcache = RefCell::new(bdcache::new_prog_cache(&config).unwrap());
        let ctx = BdContext::new(&bd, &config, &rcache, &pcache);
        let mut root = metadata::fetch_metadata_pair(&ctx, [0, 1]).unwrap();
        let d0_pair = [2u32, 3];
        let mkdir_attrs = [
            commit::CommitAttr::create(1),
            commit::CommitAttr::name_dir(1, b"d0"),
            commit::CommitAttr::dir_struct(1, d0_pair),
            commit::CommitAttr::soft_tail(d0_pair),
        ];
        commit::dir_commit_append(
            &ctx,
            &mut root,
            &mkdir_attrs,
            &mut None,
            crate::superblock::DISK_VERSION,
        )
        .unwrap();
        root = metadata::fetch_metadata_pair(&ctx, [0, 1]).unwrap();
        let rename_attrs = [
            commit::CommitAttr::create(2),
            commit::CommitAttr::name_dir(2, b"x0"),
            commit::CommitAttr::dir_struct(2, d0_pair),
            commit::CommitAttr::delete(1),
        ];
        commit::dir_commit_append(
            &ctx,
            &mut root,
            &rename_attrs,
            &mut None,
            crate::superblock::DISK_VERSION,
        )
        .unwrap();
        bdcache::bd_sync(&bd, &config, &rcache, &pcache).unwrap();

        let (_dir, id) = dir_find(&ctx, [0, 1], "x0", 255, None).unwrap();
        assert_eq!(id, 2);
        assert!(dir_find(&ctx, [0, 1], "d0", 255, None).is_err());
    }

    #[test]
    fn dir_find_for_create_new_append() {
        let (bd, config) = formatted_bd();
        let rcache = RefCell::new(bdcache::new_read_cache(&config).unwrap());
        let pcache = RefCell::new(bdcache::new_prog_cache(&config).unwrap());
        let ctx = BdContext::new(&bd, &config, &rcache, &pcache);
        let mut root = metadata::fetch_metadata_pair(&ctx, [0, 1]).unwrap();
        let a_pair = [2u32, 3];
        let b_pair = [4u32, 5];
        let attrs_a = [
            commit::CommitAttr::create(1),
            commit::CommitAttr::name_dir(1, b"a"),
            commit::CommitAttr::dir_struct(1, a_pair),
            commit::CommitAttr::soft_tail(a_pair),
        ];
        commit::dir_commit_append(
            &ctx,
            &mut root,
            &attrs_a,
            &mut None,
            crate::superblock::DISK_VERSION,
        )
        .unwrap();
        root = metadata::fetch_metadata_pair(&ctx, [0, 1]).unwrap();
        let attrs_b = [
            commit::CommitAttr::create(2),
            commit::CommitAttr::name_dir(2, b"b"),
            commit::CommitAttr::dir_struct(2, b_pair),
            commit::CommitAttr::soft_tail(b_pair),
        ];
        commit::dir_commit_append(
            &ctx,
            &mut root,
            &attrs_b,
            &mut None,
            crate::superblock::DISK_VERSION,
        )
        .unwrap();
        bdcache::bd_sync(&bd, &config, &rcache, &pcache).unwrap();

        let (dir, id, name) = dir_find_for_create(&ctx, [0, 1], "z", 255, None).unwrap();
        assert_eq!(id, 3);
        assert_eq!(name, "z");
        assert_eq!(dir.pair, [0, 1]);
    }

    #[test]
    fn dir_find_for_create_new_insert() {
        let (bd, config) = formatted_bd();
        let rcache = RefCell::new(bdcache::new_read_cache(&config).unwrap());
        let pcache = RefCell::new(bdcache::new_prog_cache(&config).unwrap());
        let ctx = BdContext::new(&bd, &config, &rcache, &pcache);
        let mut root = metadata::fetch_metadata_pair(&ctx, [0, 1]).unwrap();
        let a_pair = [2u32, 3];
        let c_pair = [4u32, 5];
        let attrs_a = [
            commit::CommitAttr::create(1),
            commit::CommitAttr::name_dir(1, b"a"),
            commit::CommitAttr::dir_struct(1, a_pair),
            commit::CommitAttr::soft_tail(a_pair),
        ];
        commit::dir_commit_append(
            &ctx,
            &mut root,
            &attrs_a,
            &mut None,
            crate::superblock::DISK_VERSION,
        )
        .unwrap();
        root = metadata::fetch_metadata_pair(&ctx, [0, 1]).unwrap();
        let attrs_c = [
            commit::CommitAttr::create(2),
            commit::CommitAttr::name_dir(2, b"c"),
            commit::CommitAttr::dir_struct(2, c_pair),
            commit::CommitAttr::soft_tail(c_pair),
        ];
        commit::dir_commit_append(
            &ctx,
            &mut root,
            &attrs_c,
            &mut None,
            crate::superblock::DISK_VERSION,
        )
        .unwrap();
        bdcache::bd_sync(&bd, &config, &rcache, &pcache).unwrap();

        let (dir, id, name) = dir_find_for_create(&ctx, [0, 1], "b", 255, None).unwrap();
        assert_eq!(id, 2, "b inserts between a and c at id 2");
        assert_eq!(name, "b");
        assert_eq!(dir.pair, [0, 1]);
    }

    #[test]
    fn dir_find_for_create_exists() {
        let (bd, config) = formatted_bd();
        let rcache = RefCell::new(bdcache::new_read_cache(&config).unwrap());
        let pcache = RefCell::new(bdcache::new_prog_cache(&config).unwrap());
        let ctx = BdContext::new(&bd, &config, &rcache, &pcache);
        let mut root = metadata::fetch_metadata_pair(&ctx, [0, 1]).unwrap();
        let a_pair = [2u32, 3];
        let attrs = [
            commit::CommitAttr::create(1),
            commit::CommitAttr::name_dir(1, b"a"),
            commit::CommitAttr::dir_struct(1, a_pair),
            commit::CommitAttr::soft_tail(a_pair),
        ];
        commit::dir_commit_append(
            &ctx,
            &mut root,
            &attrs,
            &mut None,
            crate::superblock::DISK_VERSION,
        )
        .unwrap();
        bdcache::bd_sync(&bd, &config, &rcache, &pcache).unwrap();

        assert!(matches!(
            dir_find_for_create(&ctx, [0, 1], "a", 255, None),
            Err(Error::Exist)
        ));
    }

    #[test]
    fn dir_find_for_create_parent_noent() {
        let (bd, config) = formatted_bd();
        let rcache = RefCell::new(bdcache::new_read_cache(&config).unwrap());
        let pcache = RefCell::new(bdcache::new_prog_cache(&config).unwrap());
        let ctx = BdContext::new(&bd, &config, &rcache, &pcache);

        assert!(matches!(
            dir_find_for_create(&ctx, [0, 1], "nonexistent/x", 255, None),
            Err(Error::Noent)
        ));
    }
}
