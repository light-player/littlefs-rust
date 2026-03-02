//! Path resolution for littlefs.
//!
//! Per lfs_dir_find (lfs.c 1483-1590).

use crate::block::BlockDevice;
use crate::config::Config;
use crate::error::Error;
use crate::info::FileType;

use super::metadata;

/// Find entry at path. Returns (MdDir, id) where id is 0x3ff for root.
pub fn dir_find<B: BlockDevice>(
    bd: &B,
    config: &Config,
    root: [u32; 2],
    path: &str,
    name_max: u32,
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
        let dir = metadata::fetch_metadata_pair(bd, config, root)?;
        return Ok((dir, 0x3ff));
    }

    let mut stack: alloc::vec::Vec<(metadata::MdDir, u16)> = alloc::vec![];
    let mut cwd = metadata::fetch_metadata_pair(bd, config, root)?;
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

        let found_id = find_name_in_dir(bd, config, &cwd, seg, name_max)?;
        let info = metadata::get_entry_info(&cwd, found_id, name_max)?;

        if seg_idx + 1 >= segments.len() {
            tag_id = found_id;
            break;
        }

        if info.typ != FileType::Dir {
            return Err(Error::NotDir);
        }

        stack.push((cwd.clone(), found_id));
        if found_id == 0x3ff {
            cwd = metadata::fetch_metadata_pair(bd, config, root)?;
        } else {
            let pair = get_dir_struct(&cwd, found_id)?;
            cwd = metadata::fetch_metadata_pair(bd, config, pair)?;
        }
        tag_id = found_id;
        seg_idx += 1;
    }

    Ok((cwd, tag_id))
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
    bd: &B,
    config: &Config,
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

    match find_name_in_dir_pair(bd, config, dir, name_bytes, name_max, start_id) {
        Ok(id) => Ok(id),
        Err(Error::Noent) if dir.split => {
            let next_dir = metadata::fetch_metadata_pair(bd, config, dir.tail)?;
            find_name_in_dir(bd, config, &next_dir, name, name_max)
        }
        other => other,
    }
}

fn find_name_in_dir_pair<B: BlockDevice>(
    _bd: &B,
    _config: &Config,
    dir: &metadata::MdDir,
    name_bytes: &[u8],
    name_max: u32,
    start_id: u16,
) -> Result<u16, Error> {
    for id in start_id..dir.count {
        match metadata::get_entry_info(dir, id, name_max) {
            Ok(info) => {
                if info.name_bytes() == name_bytes {
                    return Ok(id);
                }
            }
            Err(Error::Noent) => continue,
            Err(e) => return Err(e),
        }
    }
    Err(Error::Noent)
}

fn get_dir_struct(dir: &metadata::MdDir, id: u16) -> Result<[u32; 2], Error> {
    let bytes = metadata::get_struct(dir, id)?;
    Ok([
        u32::from_le_bytes(bytes[0..4].try_into().unwrap()),
        u32::from_le_bytes(bytes[4..8].try_into().unwrap()),
    ])
}
