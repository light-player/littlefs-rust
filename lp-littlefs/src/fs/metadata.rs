//! Metadata block parsing.
//!
//! Per SPEC.md and lfs.c lfs_dir_fetchmatch, lfs_dir_getslice.

use crate::block::BlockDevice;
use crate::crc;
use crate::error::Error;

use super::bdcache::BdContext;
use crate::info::{FileType, Info};
use crate::superblock::{tag, Superblock};
use crate::trace;

const BLOCK_NULL: u32 = 0xffff_ffff;

/// FCRC (forward CRC) data from TYPE_FCRC tag. Per lfs_fcrc.
#[derive(Clone, Copy)]
struct Fcrc {
    size: u32,
    crc: u32,
}

/// Parsed metadata pair state. Holds block data for backward iteration.
#[derive(Clone)]
pub struct MdDir {
    /// The block we're reading from (pair[0] or pair[1], chosen by revision).
    pub pair: [u32; 2],
    /// Block content (from the chosen block).
    block: alloc::vec::Vec<u8>,
    /// Revision of the current block (for wear-leveling).
    pub rev: u32,
    /// Offset of end of last valid commit.
    pub off: usize,
    /// Last tag before CRC (for backward iteration).
    pub etag: u32,
    /// Number of entries (max id + 1, adjusted by splice).
    pub count: u16,
    /// Next metadata pair (tail).
    pub tail: [u32; 2],
    /// True if tail is hard (more entries in this dir).
    pub split: bool,
    /// True if the region after off is erased (valid for next commit). Set from FCRC check. Used by fs_gc.
    #[allow(dead_code)]
    pub erased: bool,
}

impl MdDir {
    /// Create an empty MdDir for a newly allocated pair. Used by dir_alloc.
    pub fn alloc_empty(pair: [u32; 2], block_size: usize) -> Self {
        Self {
            pair,
            block: alloc::vec![0; block_size],
            rev: 0,
            off: 4,
            etag: 0xffff_ffff,
            count: 0,
            tail: [BLOCK_NULL, BLOCK_NULL],
            split: false,
            erased: true,
        }
    }

    /// True if tail is null (no next pair).
    pub fn tail_is_null(&self) -> bool {
        self.tail[0] == BLOCK_NULL && self.tail[1] == BLOCK_NULL
    }
}

fn tag_type1(t: u32) -> u32 {
    (t & 0x7000_0000) >> 20
}
fn tag_type2(t: u32) -> u32 {
    (t & 0x7800_0000) >> 20
}
fn tag_type3(t: u32) -> u32 {
    (t & 0x7ff0_0000) >> 20
}
fn tag_id(t: u32) -> u16 {
    ((t >> 10) & 0x3ff) as u16
}
fn tag_size(t: u32) -> u32 {
    t & 0x3ff
}
fn tag_isdelete(t: u32) -> bool {
    ((tag_size(t) as i32) << 22 >> 22) == -1
}
fn tag_chunk(t: u32) -> u8 {
    ((t & 0x0ff0_0000) >> 20) as u8
}
fn tag_splice(t: u32) -> i32 {
    tag_chunk(t) as i8 as i32
}
fn tag_dsize(t: u32) -> usize {
    let size = tag_size(t);
    let data_size = if size == 0x3ff { 0 } else { size as usize };
    4 + data_size
}
fn tag_isvalid(t: u32) -> bool {
    (t & 0x8000_0000) == 0
}

/// Fetch and parse a metadata pair. Picks block by revision, scans commits.
/// If the higher-rev block fails CRC (e.g. partial write), tries the other block.
/// Per lfs.c lfs_dir_fetchmatch.
pub fn fetch_metadata_pair<B: BlockDevice>(
    ctx: &BdContext<'_, B>,
    pair: [u32; 2],
) -> Result<MdDir, Error> {
    fetch_metadata_pair_ext(ctx, pair, None, None)
}

/// Extended fetch with optional seed accumulation and disk version for FCRC/erased check.
pub fn fetch_metadata_pair_ext<B: BlockDevice>(
    ctx: &BdContext<'_, B>,
    pair: [u32; 2],
    mut seed_out: Option<&mut u32>,
    disk_version: Option<u32>,
) -> Result<MdDir, Error> {
    use crate::superblock::DISK_VERSION;
    let config = ctx.config;
    trace!("fetch_metadata_pair pair={:?}", pair);
    let block_size = config.block_size as usize;
    if config.block_count != 0 && (pair[0] >= config.block_count || pair[1] >= config.block_count) {
        return Err(Error::Corrupt);
    }

    let mut revs = [0u32; 2];
    let mut blocks = [alloc::vec![0u8; block_size], alloc::vec![0u8; block_size]];
    ctx.read(pair[0], 0, &mut blocks[0])?;
    ctx.read(pair[1], 0, &mut blocks[1])?;
    revs[0] = u32::from_le_bytes(blocks[0][0..4].try_into().unwrap());
    revs[1] = u32::from_le_bytes(blocks[1][0..4].try_into().unwrap());

    // Pick block with higher revision (sequence compare). When equal, prefer block 0
    // so we consistently append to the same block. Per lfs_dir_fetchmatch.
    let r = if (revs[0] as i32).wrapping_sub(revs[1] as i32) > 0 {
        0
    } else if (revs[1] as i32).wrapping_sub(revs[0] as i32) > 0 {
        1
    } else {
        trace!(
            "fetch_metadata_pair revs equal revs={:?}, using block 0",
            revs
        );
        0
    };

    for attempt in 0..2 {
        let r_use = if attempt == 0 { r } else { 1 - r };
        let block = &blocks[r_use];
        let block_idx = pair[r_use];
        let dir_rev = revs[r_use];
        trace!(
            "fetch_metadata_pair attempt={} r_use={} block_idx={} revs={:?}",
            attempt,
            r_use,
            block_idx,
            revs
        );

        let mut tempcount: i32 = 0;
        let mut max_id: u16 = 0;
        let mut seen_name_or_splice = false;
        let mut temptail: [u32; 2] = [0xffff_ffff, 0xffff_ffff];
        let mut tempsplit = false;
        let mut last_off = 0usize;
        let mut last_etag = 0u32;
        let mut last_tail: [u32; 2] = [0xffff_ffff, 0xffff_ffff];
        let mut last_split = false;
        let mut hasfcrc = false;
        let mut fcrc = Fcrc { size: 0, crc: 0 };

        let mut ptag: u32 = 0xffff_ffff;
        let mut off = 4usize;
        let mut crc = crc::crc32(0xffff_ffff, &dir_rev.to_le_bytes());

        loop {
            if off + 4 > block_size {
                break;
            }
            let stored_tag =
                u32::from_be_bytes(block[off..off + 4].try_into().map_err(|_| Error::Corrupt)?);
            crc = crc::crc32(crc, &block[off..off + 4]);
            let tag = (stored_tag ^ ptag) & 0x7fff_ffff;

            if !tag_isvalid(tag) {
                break;
            }
            let dsize = tag_dsize(tag);
            if off + dsize > block_size {
                break;
            }

            ptag = tag;

            if tag_type2(tag) == tag::TYPE_CCRC {
                let mut dcrc_bytes = [0u8; 4];
                dcrc_bytes.copy_from_slice(&block[off + 4..off + 8]);
                let dcrc_val = u32::from_le_bytes(dcrc_bytes);
                // CRC covers revision + all tag headers (including this one); stored value is expected crc
                if crc != dcrc_val {
                    break;
                }
                if let Some(seed) = seed_out.as_deref_mut() {
                    *seed = crc::crc32(*seed, &dcrc_val.to_le_bytes());
                }
                ptag ^= ((tag_chunk(tag) & 1) as u32) << 31;

                last_off = off + dsize;
                last_etag = ptag;
                last_tail = temptail;
                last_split = tempsplit;
                // Per lfs.c lfs_dir_fetchmatch: splice gives live count. For dir_read/find
                // we iterate ids 0..count; count must be >= max_id+1 to reach all entries.
                // Only apply max_id when we've seen NAME/SPLICE (avoid count=1 for empty child dirs).
                if seen_name_or_splice {
                    tempcount = tempcount.max(0).max(max_id as i32 + 1);
                } else {
                    tempcount = tempcount.max(0);
                }
                temptail = [0xffff_ffff, 0xffff_ffff];
                tempsplit = false;

                crc = 0xffff_ffff;
                off += dsize;
                continue;
            }

            crc = crc::crc32(crc, &block[off + 4..off + dsize]);

            if tag_type1(tag) == tag::TYPE_NAME {
                let id = tag_id(tag);
                seen_name_or_splice = true;
                max_id = max_id.max(id);
                if id as i32 >= tempcount {
                    tempcount = id as i32 + 1;
                }
            } else if tag_type1(tag) == tag::TYPE_SPLICE {
                seen_name_or_splice = true;
                max_id = max_id.max(tag_id(tag));
                tempcount += tag_splice(tag);
            } else if tag_type1(tag) == tag::TYPE_TAIL {
                tempsplit = (tag_chunk(tag) & 1) != 0;
                temptail[0] = u32::from_le_bytes(block[off + 4..off + 8].try_into().unwrap());
                temptail[1] = u32::from_le_bytes(block[off + 8..off + 12].try_into().unwrap());
            } else if tag_type3(tag) == tag::TYPE_FCRC && dsize >= 12 {
                fcrc.size = u32::from_le_bytes(block[off + 4..off + 8].try_into().unwrap());
                fcrc.crc = u32::from_le_bytes(block[off + 8..off + 12].try_into().unwrap());
                hasfcrc = true;
            }

            off += dsize;
        }

        if last_off > 0 {
            let count = if tempcount < 0 { 0 } else { tempcount as u16 };
            let prog_size = config.prog_size as usize;
            let dv = disk_version.unwrap_or(DISK_VERSION);
            let erased = if last_off.is_multiple_of(prog_size)
                && dv >= 0x0002_0001
                && hasfcrc
                && last_off + fcrc.size as usize <= block_size
            {
                let fcrc_actual =
                    crc::crc32(0xffff_ffff, &block[last_off..last_off + fcrc.size as usize]);
                fcrc_actual == fcrc.crc
            } else {
                last_off.is_multiple_of(prog_size) && dv < 0x0002_0001
            };
            trace!(
                "fetch_metadata_pair done block_idx={} count={} off={} tail={:?} split={} erased={}",
                block_idx,
                count,
                last_off,
                last_tail,
                last_split,
                erased
            );
            return Ok(MdDir {
                pair: [block_idx, pair[1 - r_use]],
                block: block.to_vec(),
                rev: dir_rev,
                off: last_off,
                etag: last_etag,
                count,
                tail: last_tail,
                split: last_split,
                erased,
            });
        }
    }
    trace!(
        "fetch_metadata_pair Corrupt pair={:?} revs={:?} (no valid commit in either block)",
        pair,
        revs
    );
    Err(Error::Corrupt)
}

/// Iterate backward to find tag matching (gmask, gtag). Returns (tag, data_offset, size) or None.
/// Applies splice gdiff per lfs_dir_getslice: when we see SPLICE (type1=4), adjust effective
/// search tag so logical ids shift correctly (e.g. CREATE at id 1 shifts older id 1 to id 2).
pub fn get_tag_backwards(
    dir: &MdDir,
    gmask: u32,
    gtag: u32,
) -> Result<Option<(u32, usize, u32)>, Error> {
    let block = &dir.block;
    let mut off = dir.off;
    let mut ntag = dir.etag;
    let mut gdiff = 0u32;

    while off >= 4 + tag_dsize(ntag) {
        off -= tag_dsize(ntag);
        let tag = ntag;
        if off < 4 {
            break;
        }
        let stored =
            u32::from_be_bytes(block[off..off + 4].try_into().map_err(|_| Error::Corrupt)?);
        ntag = (stored ^ tag) & 0x7fff_ffff;

        // Per lfs_dir_getslice (lfs.c:750-761). Synthetic moves (726-734) omitted—
        // would require gstate/gdisk context.
        if tag_id(gmask) != 0
            && tag_type1(tag) == tag::TYPE_SPLICE
            && tag_id(tag) <= tag_id(gtag.wrapping_sub(gdiff))
        {
            // "Found where we were created" (lfs.c:753-756): return before gdiff add so
            // CREATE never corrupts gdiff. Only add for pure SPLICE (0x400); CREATE and
            // DELETE use chunk differently and would corrupt the id search.
            if tag_type3(tag) == tag::TYPE_SPLICE {
                gdiff = gdiff.wrapping_add((tag_splice(tag) as u32) << 10);
            }
        }

        if (gmask & tag) == (gmask & gtag.wrapping_sub(gdiff)) {
            // Found where we were created: matched CREATE, no content (lfs.c:753-756).
            let eff_id = tag_id(gtag.wrapping_sub(gdiff));
            if tag == ((tag::TYPE_CREATE << 20) | ((eff_id as u32) << 10)) {
                return Ok(None);
            }
            if tag_isdelete(tag) {
                return Ok(None);
            }
            let data_off = off + 4;
            let size = tag_size(tag);
            return Ok(Some((tag.wrapping_add(gdiff), data_off, size)));
        }
    }
    Ok(None)
}

/// Get entry info for id. Returns NAME (name+type) and STRUCT (size).
/// Skips entries that have a DELETE tag (most recent in backward order).
pub fn get_entry_info(dir: &MdDir, id: u16, name_max: u32) -> Result<Info, Error> {
    if id == 0x3ff {
        let mut info = Info::new(FileType::Dir, 0);
        info.set_name(b"/");
        return Ok(info);
    }

    // If entry was deleted, return Noent. Check DELETE before NAME since
    // backward iteration sees most-recent tags first.
    let delete_gtag = (tag::TYPE_DELETE << 20) | ((id as u32) << 10);
    if get_tag_backwards(dir, 0x7ff0_fc00, delete_gtag)?.is_some() {
        return Err(Error::Noent);
    }

    // Per lfs_dir_get: NAME (REG/DIR) or SUPERBLOCK for root id 0.
    // LFS_TYPE_NAME with mask 0x780 matches both REG (0x001) and DIR (0x002) name tags.
    let gmask = 0x780ffc00;
    let name_gtag = (tag::TYPE_NAME << 20) | ((id as u32) << 10) | (name_max + 1);
    let name_result = get_tag_backwards(dir, gmask, name_gtag)?;

    let (name_tag, name_off, name_tag_size) = match name_result {
        Some((tag, off, size)) => (tag, off, size),
        None if id == 0 => {
            // Root superblock: name comes from SUPERBLOCK tag, type is Dir
            let sb_gtag = (tag::TYPE_SUPERBLOCK << 20) | 8;
            let sb_result = get_tag_backwards(dir, 0x7ff0_fc00, sb_gtag)?;
            let (sb_tag, sb_off, sb_size) = match sb_result {
                Some(x) => x,
                None => return Err(Error::Noent),
            };
            if tag_type3(sb_tag) != tag::TYPE_SUPERBLOCK || sb_size != 8 {
                return Err(Error::Corrupt);
            }
            let block = &dir.block;
            let mut name_buf = [0u8; 256];
            let copy_len = core::cmp::min(8, 255);
            name_buf[..copy_len].copy_from_slice(&block[sb_off..sb_off + copy_len]);
            let nul = name_buf.iter().position(|&b| b == 0).unwrap_or(256);
            let mut info = Info::new(FileType::Dir, 0);
            info.set_name(&name_buf[..nul]);
            return Ok(info);
        }
        None => return Err(Error::Noent),
    };

    let block = &dir.block;
    let copy_len = core::cmp::min(name_tag_size as usize, (name_max + 1) as usize);
    let mut name_buf = [0u8; 256];
    let copy_len = core::cmp::min(copy_len, 255);
    name_buf[..copy_len].copy_from_slice(&block[name_off..name_off + copy_len]);

    let typ = FileType::from_type3(tag_type3(name_tag)).ok_or(Error::Corrupt)?;

    let struct_gtag = (tag::TYPE_STRUCT << 20) | ((id as u32) << 10) | 8;
    let struct_result = get_tag_backwards(dir, 0x700f_fc00, struct_gtag)?;
    let (struct_tag, struct_off, _) = match struct_result {
        Some(x) => x,
        None => return Err(Error::Noent),
    };

    let size = if tag_type3(struct_tag) == tag::TYPE_CTZSTRUCT {
        u32::from_le_bytes(
            block[struct_off + 4..struct_off + 8]
                .try_into()
                .map_err(|_| Error::Corrupt)?,
        )
    } else if tag_type3(struct_tag) == tag::TYPE_INLINESTRUCT {
        tag_size(struct_tag)
    } else {
        0
    };

    let nul = name_buf.iter().position(|&b| b == 0).unwrap_or(256);
    let mut info = Info::new(typ, size);
    info.set_name(&name_buf[..nul]);
    Ok(info)
}

/// Try to read superblock from an already-fetched metadata dir.
/// Returns Some if the dir contains the superblock (INLINESTRUCT id 0, size 24).
/// Per LittleFS, only the root metadata pair [0,1] contains the superblock.
pub fn get_superblock_from_dir(dir: &MdDir) -> Option<Superblock> {
    if dir.pair[0] > 1 && dir.pair[1] > 1 {
        return None;
    }
    let gmask = 0x7ff0_fc00;
    let gtag = (tag::TYPE_INLINESTRUCT << 20) | Superblock::SIZE as u32;
    let (_, off, size) = get_tag_backwards(dir, gmask, gtag).ok()??;
    if size != Superblock::SIZE as u32 {
        return None;
    }
    let block = &dir.block;
    if off + Superblock::SIZE > block.len() {
        return None;
    }
    Some(Superblock {
        version: u32::from_le_bytes(block[off..off + 4].try_into().unwrap()),
        block_size: u32::from_le_bytes(block[off + 4..off + 8].try_into().unwrap()),
        block_count: u32::from_le_bytes(block[off + 8..off + 12].try_into().unwrap()),
        name_max: u32::from_le_bytes(block[off + 12..off + 16].try_into().unwrap()),
        file_max: u32::from_le_bytes(block[off + 16..off + 20].try_into().unwrap()),
        attr_max: u32::from_le_bytes(block[off + 20..off + 24].try_into().unwrap()),
    })
}

/// Read superblock from root metadata pair.
#[allow(dead_code)]
pub fn read_superblock<B: BlockDevice>(
    ctx: &BdContext<'_, B>,
    root: [u32; 2],
) -> Result<Superblock, Error> {
    let dir = fetch_metadata_pair(ctx, root)?;
    let gmask = 0x7ff0_fc00;
    let gtag = (tag::TYPE_INLINESTRUCT << 20) | Superblock::SIZE as u32;
    let result = get_tag_backwards(&dir, gmask, gtag)?;
    let (_, off, _) = result.ok_or(Error::Corrupt)?;

    let block = &dir.block;
    if off + Superblock::SIZE > block.len() {
        return Err(Error::Corrupt);
    }
    let sb = Superblock {
        version: u32::from_le_bytes(block[off..off + 4].try_into().unwrap()),
        block_size: u32::from_le_bytes(block[off + 4..off + 8].try_into().unwrap()),
        block_count: u32::from_le_bytes(block[off + 8..off + 12].try_into().unwrap()),
        name_max: u32::from_le_bytes(block[off + 12..off + 16].try_into().unwrap()),
        file_max: u32::from_le_bytes(block[off + 16..off + 20].try_into().unwrap()),
        attr_max: u32::from_le_bytes(block[off + 20..off + 24].try_into().unwrap()),
    };
    Ok(sb)
}

/// Resolve file struct (inline or CTZ) for a regular file.
///
/// Returns (is_inline, head_block, size). For inline: head is BLOCK_INLINE (0xffff_fffe).
pub fn get_file_struct(dir: &MdDir, id: u16) -> Result<(bool, u32, u64), Error> {
    let gmask = 0x700f_fc00;
    let gtag = (tag::TYPE_STRUCT << 20) | ((id as u32) << 10);
    let result = get_tag_backwards(dir, gmask, gtag)?;
    let (struct_tag, struct_off, struct_size) = result.ok_or(Error::Noent)?;
    let block = &dir.block;
    let type3 = tag_type3(struct_tag);

    if type3 == tag::TYPE_INLINESTRUCT {
        return Ok((true, 0xffff_fffe, struct_size as u64));
    }
    if type3 == tag::TYPE_CTZSTRUCT {
        if struct_size != 8 || struct_off + 8 > block.len() {
            return Err(Error::Corrupt);
        }
        let head = u32::from_le_bytes(block[struct_off..struct_off + 4].try_into().unwrap());
        let size = u32::from_le_bytes(block[struct_off + 4..struct_off + 8].try_into().unwrap());
        return Ok((false, head, size as u64));
    }
    Err(Error::Noent)
}

/// Read a slice of inline file data.
///
/// Per lfs_dir_getslice (lfs.c:719). Finds INLINESTRUCT tag for id and copies
/// up to `buffer.len()` bytes from offset `offset` into `buffer`. Returns bytes read.
pub fn get_inline_slice(
    dir: &MdDir,
    id: u16,
    offset: usize,
    buffer: &mut [u8],
) -> Result<usize, Error> {
    let gmask = 0x7ff0_fc00;
    let gtag = (tag::TYPE_INLINESTRUCT << 20) | ((id as u32) << 10);
    let result = get_tag_backwards(dir, gmask, gtag)?;
    let (_, data_off, size) = result.ok_or(Error::Noent)?;
    let data_size = size as usize;
    if offset >= data_size {
        return Ok(0);
    }
    let avail = data_size - offset;
    let to_read = core::cmp::min(buffer.len(), avail);
    let block = &dir.block;
    if data_off + offset + to_read > block.len() {
        return Err(Error::Corrupt);
    }
    buffer[..to_read].copy_from_slice(&block[data_off + offset..data_off + offset + to_read]);
    Ok(to_read)
}

/// Callback result for dir_traverse_tags. Continue or stop.
#[derive(Clone, Copy)]
pub enum TraverseAction {
    Continue,
    #[allow(dead_code)]
    Stop,
}

/// Iterate tags forward through a metadata block, optionally filtered by id range [begin, end).
/// Calls `cb` with (tag, data_slice) for each tag. Used for commit size and compact.
/// Per lfs_dir_traverse (lfs.c:912). Simplified: no attrs merge, no FROM_MOVE/FROM_USERATTRS.
///
/// When begin < end, only yields tags whose id is in [begin, end). The diff is added to the
/// id field of the output tag (for compact, diff = -begin so ids are remapped to 0..).
/// When begin == end, yields nothing (empty range).
pub fn dir_traverse_tags<F>(
    dir: &MdDir,
    begin: u16,
    end: u16,
    diff: i32,
    mut cb: F,
) -> Result<(), Error>
where
    F: FnMut(u32, &[u8]) -> Result<TraverseAction, Error>,
{
    let block = &dir.block;
    let block_size = block.len();

    let mut off = 4usize;
    let mut ptag: u32 = 0xffff_ffff;

    while off + 4 <= block_size {
        let stored_tag =
            u32::from_be_bytes(block[off..off + 4].try_into().map_err(|_| Error::Corrupt)?);
        let tag = (stored_tag ^ ptag) & 0x7fff_ffff;

        if !tag_isvalid(tag) {
            break;
        }

        let dsize = tag_dsize(tag);
        if off + dsize > block_size {
            break;
        }

        let data = if dsize > 4 {
            &block[off + 4..off + dsize]
        } else {
            &[]
        };

        ptag = tag;

        if tag_type2(tag) == tag::TYPE_CCRC {
            off += dsize;
            continue;
        }

        let id = tag_id(tag);
        if tag_type1(tag) == tag::TYPE_SPLICE {
            off += dsize;
            continue;
        }

        let id_in_range = begin < end && id >= begin && id < end;

        if !id_in_range {
            off += dsize;
            continue;
        }

        let out_id = (id as i32 + diff).clamp(0, 0x3ff) as u16;
        let out_tag = (tag & 0xffff_fc00) | ((out_id as u32) << 10) | (tag & 0x3ff);

        match cb(out_tag, data)? {
            TraverseAction::Continue => {}
            TraverseAction::Stop => return Ok(()),
        }

        off += dsize;
    }

    Ok(())
}

/// Compute byte size of tags in id range [begin, end) from source.
/// Per lfs_dir_commit_size (lfs.c:1915).
pub fn dir_traverse_size(dir: &MdDir, begin: u16, end: u16) -> Result<usize, Error> {
    let mut size = 0usize;
    dir_traverse_tags(dir, begin, end, 0, |tag, _data| {
        size += tag_dsize(tag);
        Ok(TraverseAction::Continue)
    })?;
    Ok(size)
}

/// Get 8-byte struct data (e.g. dir pair) for an id.
pub fn get_struct(dir: &MdDir, id: u16) -> Result<[u8; 8], Error> {
    let gmask = 0x700f_fc00;
    let gtag = (tag::TYPE_STRUCT << 20) | ((id as u32) << 10) | 8;
    let result = get_tag_backwards(dir, gmask, gtag)?;
    let (_, off, _) = result.ok_or(Error::Noent)?;

    let block = &dir.block;
    if off + 8 > block.len() {
        return Err(Error::Corrupt);
    }
    let mut out = [0u8; 8];
    out.copy_from_slice(&block[off..off + 8]);
    Ok(out)
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
    fn fetch_metadata_pair_after_format() {
        let (bd, config) = formatted_bd();
        let rcache = RefCell::new(bdcache::new_read_cache(&config).unwrap());
        let pcache = RefCell::new(bdcache::new_prog_cache(&config).unwrap());
        let ctx = BdContext::new(&bd, &config, &rcache, &pcache);
        let dir = fetch_metadata_pair(&ctx, [0, 1]).unwrap();
        assert_eq!(dir.pair[0], 0);
        assert_eq!(dir.count, 1);
        assert!(!dir.split);
    }

    #[test]
    fn get_tag_backwards_inlinestruct_id0() {
        let (bd, config) = formatted_bd();
        let rcache = RefCell::new(bdcache::new_read_cache(&config).unwrap());
        let pcache = RefCell::new(bdcache::new_prog_cache(&config).unwrap());
        let ctx = BdContext::new(&bd, &config, &rcache, &pcache);
        let dir = fetch_metadata_pair(&ctx, [0, 1]).unwrap();
        let gmask = 0x7ff0_fc00;
        let gtag = (tag::TYPE_INLINESTRUCT << 20) | Superblock::SIZE as u32;
        let r = get_tag_backwards(&dir, gmask, gtag).unwrap();
        assert!(r.is_some());
        let (_t, off, size) = r.unwrap();
        assert_eq!(size, Superblock::SIZE as u32);
        assert!(off + Superblock::SIZE <= dir.block.len());
    }

    #[test]
    fn get_entry_info_superblock_id0() {
        let (bd, config) = formatted_bd();
        let rcache = RefCell::new(bdcache::new_read_cache(&config).unwrap());
        let pcache = RefCell::new(bdcache::new_prog_cache(&config).unwrap());
        let ctx = BdContext::new(&bd, &config, &rcache, &pcache);
        let dir = fetch_metadata_pair(&ctx, [0, 1]).unwrap();
        let info = get_entry_info(&dir, 0, 255).unwrap();
        assert_eq!(info.name().unwrap(), "littlefs");
    }

    #[test]
    fn read_superblock_after_format() {
        let (bd, config) = formatted_bd();
        let rcache = RefCell::new(bdcache::new_read_cache(&config).unwrap());
        let pcache = RefCell::new(bdcache::new_prog_cache(&config).unwrap());
        let ctx = BdContext::new(&bd, &config, &rcache, &pcache);
        let sb = read_superblock(&ctx, [0, 1]).unwrap();
        assert_eq!(sb.block_size, config.block_size);
        assert_eq!(sb.block_count, config.block_count);
        assert_eq!(sb.name_max, 255);
    }

    /// Append a second commit (mkdir d0) and verify parse. Validates commit
    /// machinery produces C-compliant format per lfs.c lfs_dir_commitattr,
    /// lfs_dir_commitcrc (tags contiguous, CRC covers tag headers + data).
    #[test]
    fn fetch_after_append_commit() {
        let (bd, config) = formatted_bd();
        let rcache = RefCell::new(bdcache::new_read_cache(&config).unwrap());
        let pcache = RefCell::new(bdcache::new_prog_cache(&config).unwrap());
        let ctx = BdContext::new(&bd, &config, &rcache, &pcache);
        let mut root = fetch_metadata_pair(&ctx, [0, 1]).unwrap();
        let new_pair = [2u32, 3];
        let attrs = [
            commit::CommitAttr::create(1),
            commit::CommitAttr::name_dir(1, b"d0"),
            commit::CommitAttr::dir_struct(1, new_pair),
            commit::CommitAttr::soft_tail(new_pair),
        ];
        commit::dir_commit_append(&ctx, &mut root, &attrs, &mut None).unwrap();
        bdcache::bd_sync(&bd, &config, &rcache, &pcache).unwrap();

        let dir = fetch_metadata_pair(&ctx, [0, 1]).unwrap();
        assert!(
            dir.count >= 2,
            "expected count >= 2, got {} (off={})",
            dir.count,
            dir.off
        );

        let name_gtag = (tag::TYPE_REG << 20) | (1 << 10) | 256;
        let name_result = get_tag_backwards(&dir, 0x780f_fc00, name_gtag).unwrap();
        assert!(
            name_result.is_some(),
            "NAME(id=1) not found (off={}, etag=0x{:08x})",
            dir.off,
            dir.etag
        );

        let struct_gtag = (tag::TYPE_STRUCT << 20) | (1 << 10) | 8;
        let struct_result = get_tag_backwards(&dir, 0x700f_fc00, struct_gtag).unwrap();
        assert!(struct_result.is_some(), "STRUCT(id=1) not found");

        let info = get_entry_info(&dir, 1, 255).unwrap();
        assert_eq!(info.name().unwrap(), "d0");
    }

    /// Directly test get_tag_backwards for NAME 2 in rename scenario.
    #[test]
    fn get_tag_backwards_name2_after_rename() {
        let (bd, config) = formatted_bd();
        let rcache = RefCell::new(bdcache::new_read_cache(&config).unwrap());
        let pcache = RefCell::new(bdcache::new_prog_cache(&config).unwrap());
        let ctx = BdContext::new(&bd, &config, &rcache, &pcache);
        let mut root = fetch_metadata_pair(&ctx, [0, 1]).unwrap();
        let d0_pair = [2u32, 3];
        let mkdir_attrs = [
            commit::CommitAttr::create(1),
            commit::CommitAttr::name_dir(1, b"d0"),
            commit::CommitAttr::dir_struct(1, d0_pair),
            commit::CommitAttr::soft_tail(d0_pair),
        ];
        commit::dir_commit_append(&ctx, &mut root, &mkdir_attrs, &mut None).unwrap();
        root = fetch_metadata_pair(&ctx, [0, 1]).unwrap();

        let rename_attrs = [
            commit::CommitAttr::create(2),
            commit::CommitAttr::name_dir(2, b"x0"),
            commit::CommitAttr::dir_struct(2, d0_pair),
            commit::CommitAttr::delete(1),
        ];
        commit::dir_commit_append(&ctx, &mut root, &rename_attrs, &mut None).unwrap();
        bdcache::bd_sync(&bd, &config, &rcache, &pcache).unwrap();

        let dir = fetch_metadata_pair(&ctx, [0, 1]).unwrap();
        let gmask = 0x780ffc00;
        let name_gtag = (tag::TYPE_NAME << 20) | (2 << 10) | 256;
        let r = get_tag_backwards(&dir, gmask, name_gtag).unwrap();
        assert!(
            r.is_some(),
            "NAME 2 must be found; dir.off={} etag=0x{:08x} count={}",
            dir.off,
            dir.etag,
            dir.count
        );
    }

    /// Find STRUCT 2 after rename (same setup as get_tag_backwards_name2_after_rename).
    #[test]
    fn get_tag_backwards_struct2_after_rename() {
        let (bd, config) = formatted_bd();
        let rcache = RefCell::new(bdcache::new_read_cache(&config).unwrap());
        let pcache = RefCell::new(bdcache::new_prog_cache(&config).unwrap());
        let ctx = BdContext::new(&bd, &config, &rcache, &pcache);
        let mut root = fetch_metadata_pair(&ctx, [0, 1]).unwrap();
        let d0_pair = [2u32, 3];
        let mkdir_attrs = [
            commit::CommitAttr::create(1),
            commit::CommitAttr::name_dir(1, b"d0"),
            commit::CommitAttr::dir_struct(1, d0_pair),
            commit::CommitAttr::soft_tail(d0_pair),
        ];
        commit::dir_commit_append(&ctx, &mut root, &mkdir_attrs, &mut None).unwrap();
        root = fetch_metadata_pair(&ctx, [0, 1]).unwrap();

        let rename_attrs = [
            commit::CommitAttr::create(2),
            commit::CommitAttr::name_dir(2, b"x0"),
            commit::CommitAttr::dir_struct(2, d0_pair),
            commit::CommitAttr::delete(1),
        ];
        commit::dir_commit_append(&ctx, &mut root, &rename_attrs, &mut None).unwrap();
        bdcache::bd_sync(&bd, &config, &rcache, &pcache).unwrap();

        let dir = fetch_metadata_pair(&ctx, [0, 1]).unwrap();
        let gmask = 0x700ffc00;
        let struct_gtag = (tag::TYPE_STRUCT << 20) | (2 << 10) | 8;
        let r = get_tag_backwards(&dir, gmask, struct_gtag).unwrap();
        assert!(
            r.is_some(),
            "STRUCT 2 must be found; dir.off={} etag=0x{:08x}",
            dir.off,
            dir.etag
        );
    }

    /// Find DELETE 1 after rename (same setup).
    #[test]
    fn get_tag_backwards_delete1_after_rename() {
        let (bd, config) = formatted_bd();
        let rcache = RefCell::new(bdcache::new_read_cache(&config).unwrap());
        let pcache = RefCell::new(bdcache::new_prog_cache(&config).unwrap());
        let ctx = BdContext::new(&bd, &config, &rcache, &pcache);
        let mut root = fetch_metadata_pair(&ctx, [0, 1]).unwrap();
        let d0_pair = [2u32, 3];
        let mkdir_attrs = [
            commit::CommitAttr::create(1),
            commit::CommitAttr::name_dir(1, b"d0"),
            commit::CommitAttr::dir_struct(1, d0_pair),
            commit::CommitAttr::soft_tail(d0_pair),
        ];
        commit::dir_commit_append(&ctx, &mut root, &mkdir_attrs, &mut None).unwrap();
        root = fetch_metadata_pair(&ctx, [0, 1]).unwrap();

        let rename_attrs = [
            commit::CommitAttr::create(2),
            commit::CommitAttr::name_dir(2, b"x0"),
            commit::CommitAttr::dir_struct(2, d0_pair),
            commit::CommitAttr::delete(1),
        ];
        commit::dir_commit_append(&ctx, &mut root, &rename_attrs, &mut None).unwrap();
        bdcache::bd_sync(&bd, &config, &rcache, &pcache).unwrap();

        let dir = fetch_metadata_pair(&ctx, [0, 1]).unwrap();
        let gmask = 0x700ffc00;
        let delete_gtag = (tag::TYPE_DELETE << 20) | (1 << 10);
        let r = get_tag_backwards(&dir, gmask, delete_gtag).unwrap();
        assert!(
            r.is_some(),
            "DELETE 1 must be found; dir.off={} etag=0x{:08x}",
            dir.off,
            dir.etag
        );
    }

    /// Search for CREATE 2 should return None (found where we were created). Per lfs_dir_getslice.
    #[test]
    fn get_tag_backwards_create2_noent() {
        let (bd, config) = formatted_bd();
        let rcache = RefCell::new(bdcache::new_read_cache(&config).unwrap());
        let pcache = RefCell::new(bdcache::new_prog_cache(&config).unwrap());
        let ctx = BdContext::new(&bd, &config, &rcache, &pcache);
        let mut root = fetch_metadata_pair(&ctx, [0, 1]).unwrap();
        let d0_pair = [2u32, 3];
        let mkdir_attrs = [
            commit::CommitAttr::create(1),
            commit::CommitAttr::name_dir(1, b"d0"),
            commit::CommitAttr::dir_struct(1, d0_pair),
            commit::CommitAttr::soft_tail(d0_pair),
        ];
        commit::dir_commit_append(&ctx, &mut root, &mkdir_attrs, &mut None).unwrap();
        root = fetch_metadata_pair(&ctx, [0, 1]).unwrap();

        let rename_attrs = [
            commit::CommitAttr::create(2),
            commit::CommitAttr::name_dir(2, b"x0"),
            commit::CommitAttr::dir_struct(2, d0_pair),
            commit::CommitAttr::delete(1),
        ];
        commit::dir_commit_append(&ctx, &mut root, &rename_attrs, &mut None).unwrap();
        bdcache::bd_sync(&bd, &config, &rcache, &pcache).unwrap();

        let dir = fetch_metadata_pair(&ctx, [0, 1]).unwrap();
        let gmask = 0x7ff0fc00;
        let create_gtag = (tag::TYPE_CREATE << 20) | (2 << 10) | 0;
        let r = get_tag_backwards(&dir, gmask, create_gtag).unwrap();
        assert!(
            r.is_none(),
            "CREATE 2 search must return None (found where we were created); got {:?}",
            r
        );
    }

    /// Append rename commit (CREATE 2, NAME 2, DIRSTRUCT 2, DELETE 1) and verify.
    #[test]
    fn fetch_after_rename_commit() {
        let (bd, config) = formatted_bd();
        let rcache = RefCell::new(bdcache::new_read_cache(&config).unwrap());
        let pcache = RefCell::new(bdcache::new_prog_cache(&config).unwrap());
        let ctx = BdContext::new(&bd, &config, &rcache, &pcache);
        let mut root = fetch_metadata_pair(&ctx, [0, 1]).unwrap();
        let d0_pair = [2u32, 3];
        let mkdir_attrs = [
            commit::CommitAttr::create(1),
            commit::CommitAttr::name_dir(1, b"d0"),
            commit::CommitAttr::dir_struct(1, d0_pair),
            commit::CommitAttr::soft_tail(d0_pair),
        ];
        commit::dir_commit_append(&ctx, &mut root, &mkdir_attrs, &mut None).unwrap();
        root = fetch_metadata_pair(&ctx, [0, 1]).unwrap();

        let rename_attrs = [
            commit::CommitAttr::create(2),
            commit::CommitAttr::name_dir(2, b"x0"),
            commit::CommitAttr::dir_struct(2, d0_pair),
            commit::CommitAttr::delete(1),
        ];
        commit::dir_commit_append(&ctx, &mut root, &rename_attrs, &mut None).unwrap();
        bdcache::bd_sync(&bd, &config, &rcache, &pcache).unwrap();

        let dir = fetch_metadata_pair(&ctx, [0, 1]).unwrap();
        assert!(
            dir.count >= 3,
            "rename creates id 2; count must be >= 3 for find/stat, got {}",
            dir.count
        );
        let info = get_entry_info(&dir, 2, 255).unwrap();
        assert_eq!(info.name().unwrap(), "x0");
        assert!(get_entry_info(&dir, 1, 255).is_err()); // id 1 deleted -> Noent
    }

    #[test]
    fn dir_traverse_size_formatted() {
        let (bd, config) = formatted_bd();
        let rcache = RefCell::new(bdcache::new_read_cache(&config).unwrap());
        let pcache = RefCell::new(bdcache::new_prog_cache(&config).unwrap());
        let ctx = BdContext::new(&bd, &config, &rcache, &pcache);
        let dir = fetch_metadata_pair(&ctx, [0, 1]).unwrap();
        let size = dir_traverse_size(&dir, 0, 1).unwrap();
        assert!(size > 0, "formatted root has superblock etc");
    }

    #[test]
    fn dir_traverse_size_empty_range() {
        let (bd, config) = formatted_bd();
        let rcache = RefCell::new(bdcache::new_read_cache(&config).unwrap());
        let pcache = RefCell::new(bdcache::new_prog_cache(&config).unwrap());
        let ctx = BdContext::new(&bd, &config, &rcache, &pcache);
        let dir = fetch_metadata_pair(&ctx, [0, 1]).unwrap();
        let size = dir_traverse_size(&dir, 1, 1).unwrap();
        assert_eq!(size, 0);
    }

    #[test]
    fn get_inline_slice_reads_file_data() {
        let (bd, config) = formatted_bd();
        let rcache = RefCell::new(bdcache::new_read_cache(&config).unwrap());
        let pcache = RefCell::new(bdcache::new_prog_cache(&config).unwrap());
        let ctx = BdContext::new(&bd, &config, &rcache, &pcache);
        let mut root = fetch_metadata_pair(&ctx, [0, 1]).unwrap();
        let content = b"Hello World!\0";
        let attrs = [
            commit::CommitAttr::create(1),
            commit::CommitAttr::name_reg(1, b"hello"),
            commit::CommitAttr::inline_struct(1, content),
        ];
        commit::dir_commit_append(&ctx, &mut root, &attrs, &mut None).unwrap();
        bdcache::bd_sync(&bd, &config, &rcache, &pcache).unwrap();

        let dir = fetch_metadata_pair(&ctx, [0, 1]).unwrap();
        let mut buf = [0u8; 32];
        let n = get_inline_slice(&dir, 1, 0, &mut buf).unwrap();
        assert_eq!(n, content.len());
        assert_eq!(&buf[..n], content);

        let n2 = get_inline_slice(&dir, 1, 6, &mut buf).unwrap();
        assert_eq!(n2, content.len() - 6);
        assert_eq!(&buf[..n2], b"World!\0");
    }
}
