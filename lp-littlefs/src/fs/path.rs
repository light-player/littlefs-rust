//! Path resolution for littlefs.
//!
//! Per lfs_dir_find (lfs.c 1483-1590).

use crate::block::BlockDevice;
use crate::error::Error;
use crate::info::FileType;
use crate::trace;

use super::bdcache::BdContext;
use super::metadata;

/// Find entry at path. Returns (MdDir, id) where id is 0x3ff for root.
pub fn dir_find<B: BlockDevice>(
    ctx: &BdContext<'_, B>,
    root: [u32; 2],
    path: &str,
    name_max: u32,
) -> Result<(metadata::MdDir, u16), Error> {
    trace!("dir_find path={:?} root={:?}", path, root);
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
        trace!("dir_find segments empty -> root");
        let dir = metadata::fetch_metadata_pair(ctx, root)?;
        return Ok((dir, 0x3ff));
    }

    trace!("dir_find segments={:?}", segments);
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

        let found_id = find_name_in_dir(ctx, &cwd, seg, name_max)?;
        trace!("dir_find seg={:?} found_id={}", seg, found_id);
        let info = metadata::get_entry_info(&cwd, found_id, name_max)?;

        if seg_idx + 1 >= segments.len() {
            tag_id = found_id;
            trace!("dir_find done -> ({:?}, {})", cwd.pair, tag_id);
            break;
        }

        if info.typ != FileType::Dir {
            return Err(Error::NotDir);
        }

        stack.push((cwd.clone(), found_id));
        if found_id == 0x3ff {
            cwd = metadata::fetch_metadata_pair(ctx, root)?;
        } else {
            let pair = get_dir_struct(&cwd, found_id)?;
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
pub fn dir_find_for_create<'a, B: BlockDevice>(
    ctx: &BdContext<'_, B>,
    root: [u32; 2],
    path: &'a str,
    name_max: u32,
) -> Result<(metadata::MdDir, u16, &'a str), Error> {
    trace!("dir_find_for_create path={:?} root={:?}", path, root);
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

        let found_id = match find_name_in_dir(ctx, &cwd, seg, name_max) {
            Ok(id) => id,
            Err(Error::Noent) if seg_idx + 1 >= segments.len() => {
                let id = find_insertion_id(&cwd, seg, name_max)?;
                trace!(
                    "dir_find_for_create noent last seg -> insert id={} name={:?}",
                    id,
                    seg
                );
                return Ok((cwd, id, seg));
            }
            Err(Error::Noent) => {
                trace!(
                    "dir_find_for_create noent parent seg={:?} seg_idx={} cwd.pair={:?} cwd.count={}",
                    seg,
                    seg_idx,
                    cwd.pair,
                    cwd.count
                );
                return Err(Error::Noent);
            }
            Err(e) => return Err(e),
        };
        let info = metadata::get_entry_info(&cwd, found_id, name_max)?;

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
            let pair = get_dir_struct(&cwd, found_id)?;
            cwd = metadata::fetch_metadata_pair(ctx, pair)?;
        }
        tag_id = found_id;
        seg_idx += 1;
    }

    Err(Error::Noent)
}

/// Find the id slot where a new name would be inserted to maintain alphabetical order.
fn find_insertion_id(dir: &metadata::MdDir, name: &str, name_max: u32) -> Result<u16, Error> {
    let name_bytes = name.as_bytes();
    let start_id = if dir.pair[0] == 0 || dir.pair[0] == 1 {
        1
    } else {
        0
    };

    trace!(
        "find_insertion_id name={:?} dir.count={} start_id={}",
        name,
        dir.count,
        start_id
    );
    for id in start_id..dir.count {
        match metadata::get_entry_info(dir, id, name_max) {
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
    trace!("find_insertion_id -> dir.count={}", dir.count);
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
    dir: &metadata::MdDir,
    name: &str,
    name_max: u32,
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

    trace!(
        "find_name_in_dir name={:?} dir.pair={:?} count={} start_id={}",
        name,
        dir.pair,
        dir.count,
        start_id
    );
    match find_name_in_dir_pair(dir, name_bytes, name_max, start_id) {
        Ok(id) => {
            trace!("find_name_in_dir found id={}", id);
            Ok(id)
        }
        Err(Error::Noent) if dir.split => {
            let next_dir = metadata::fetch_metadata_pair(ctx, dir.tail)?;
            trace!("find_name_in_dir split, trying tail");
            find_name_in_dir(ctx, &next_dir, name, name_max)
        }
        other => {
            trace!("find_name_in_dir Noent (no split)");
            other
        }
    }
}

fn find_name_in_dir_pair(
    dir: &metadata::MdDir,
    name_bytes: &[u8],
    name_max: u32,
    start_id: u16,
) -> Result<u16, Error> {
    trace!(
        "find_name_in_dir_pair iterating ids {}..{}",
        start_id,
        dir.count
    );
    for id in start_id..dir.count {
        match metadata::get_entry_info(dir, id, name_max) {
            Ok(info) => {
                trace!(
                    "find_name_in_dir_pair id={} name={:?} cmp={}",
                    id,
                    info.name().ok(),
                    info.name_bytes() == name_bytes
                );
                if info.name_bytes() == name_bytes {
                    trace!("find_name_in_dir_pair match id={}", id);
                    return Ok(id);
                }
            }
            Err(Error::Noent) => continue,
            Err(e) => return Err(e),
        }
    }
    trace!(
        "find_name_in_dir_pair Noent after scanning {}-{}",
        start_id,
        dir.count
    );
    Err(Error::Noent)
}

fn get_dir_struct(dir: &metadata::MdDir, id: u16) -> Result<[u32; 2], Error> {
    let bytes = metadata::get_struct(dir, id)?;
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
