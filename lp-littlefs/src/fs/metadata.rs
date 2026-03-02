//! Metadata block parsing.
//!
//! Per SPEC.md and lfs.c lfs_dir_fetchmatch, lfs_dir_getslice.

use crate::block::BlockDevice;
use crate::config::Config;
use crate::crc;
use crate::error::Error;
use crate::info::{FileType, Info};
use crate::superblock::{tag, Superblock};

/// Parsed metadata pair state. Holds block data for backward iteration.
#[derive(Clone)]
pub struct MdDir {
    /// The block we're reading from (pair[0] or pair[1], chosen by revision).
    pub pair: [u32; 2],
    /// Block content (from the chosen block).
    block: alloc::vec::Vec<u8>,
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
pub fn fetch_metadata_pair<B: BlockDevice>(
    bd: &B,
    config: &Config,
    pair: [u32; 2],
) -> Result<MdDir, Error> {
    let block_size = config.block_size as usize;
    if config.block_count != 0 && (pair[0] >= config.block_count || pair[1] >= config.block_count) {
        return Err(Error::Corrupt);
    }

    let mut revs = [0u32; 2];
    let mut blocks = [alloc::vec![0u8; block_size], alloc::vec![0u8; block_size]];
    bd.read(pair[0], 0, &mut blocks[0])?;
    bd.read(pair[1], 0, &mut blocks[1])?;
    revs[0] = u32::from_le_bytes(blocks[0][0..4].try_into().unwrap());
    revs[1] = u32::from_le_bytes(blocks[1][0..4].try_into().unwrap());

    // Pick block with higher revision (sequence compare).
    let r = if (revs[0] as i32).wrapping_sub(revs[1] as i32) >= 0 {
        0
    } else {
        1
    };
    let block = &blocks[r];
    let block_idx = pair[r];

    let mut tempcount: i32 = 0;
    let mut temptail: [u32; 2] = [0xffff_ffff, 0xffff_ffff];
    let mut tempsplit = false;
    let mut last_off = 0usize;
    let mut last_etag = 0u32;

    let mut ptag: u32 = 0xffff_ffff;
    let mut off = 4usize;
    let dir_rev = revs[r];
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
            let dcrc = u32::from_le_bytes(dcrc_bytes);
            // CRC covers revision + all tag headers (including this one); stored value is expected crc
            if crc != dcrc {
                break;
            }
            ptag ^= ((tag_chunk(tag) & 1) as u32) << 31;

            last_off = off + dsize;
            last_etag = ptag;
            tempcount = tempcount.max(0);
            temptail = [0xffff_ffff, 0xffff_ffff];
            tempsplit = false;

            crc = 0xffff_ffff;
            off += dsize;
            continue;
        }

        crc = crc::crc32(crc, &block[off + 4..off + dsize]);

        if tag_type1(tag) == tag::TYPE_NAME {
            if tag_id(tag) as i32 >= tempcount {
                tempcount = tag_id(tag) as i32 + 1;
            }
        } else if tag_type1(tag) == tag::TYPE_SPLICE {
            tempcount += tag_splice(tag);
        } else if tag_type1(tag) == tag::TYPE_TAIL {
            tempsplit = (tag_chunk(tag) & 1) != 0;
            temptail[0] = u32::from_le_bytes(block[off + 4..off + 8].try_into().unwrap());
            temptail[1] = u32::from_le_bytes(block[off + 8..off + 12].try_into().unwrap());
        }

        off += dsize;
    }

    if last_off == 0 {
        return Err(Error::Corrupt);
    }

    let count = if tempcount < 0 { 0 } else { tempcount as u16 };

    Ok(MdDir {
        pair: [block_idx, pair[1 - r]],
        block: block.to_vec(),
        off: last_off,
        etag: last_etag,
        count,
        tail: temptail,
        split: tempsplit,
    })
}

/// Iterate backward to find tag matching (gmask, gtag). Returns (tag, data_offset, size) or None.
/// Simplified: no synthetic move / splice gdiff for Phase 01.
pub fn get_tag_backwards(
    dir: &MdDir,
    gmask: u32,
    gtag: u32,
) -> Result<Option<(u32, usize, u32)>, Error> {
    let block = &dir.block;
    let mut off = dir.off;
    let mut ntag = dir.etag;

    while off >= 4 + tag_dsize(ntag) {
        off -= tag_dsize(ntag);
        let tag = ntag;
        if off < 4 {
            break;
        }
        let stored =
            u32::from_be_bytes(block[off..off + 4].try_into().map_err(|_| Error::Corrupt)?);
        ntag = (stored ^ tag) & 0x7fff_ffff;

        if (gmask & tag) == (gmask & gtag) {
            if tag_isdelete(tag) {
                return Ok(None);
            }
            let data_off = off + 4;
            let size = tag_size(tag);
            return Ok(Some((tag, data_off, size)));
        }
    }
    Ok(None)
}

/// Get entry info for id. Returns NAME (name+type) and STRUCT (size).
pub fn get_entry_info(dir: &MdDir, id: u16, name_max: u32) -> Result<Info, Error> {
    if id == 0x3ff {
        let mut info = Info::new(FileType::Dir, 0);
        info.set_name(b"/");
        return Ok(info);
    }

    // Per lfs_dir_get: NAME (REG/DIR) or SUPERBLOCK for root id 0.
    let gmask = 0x780f_fc00;
    let name_gtag = (tag::TYPE_REG << 20) | ((id as u32) << 10) | (name_max + 1);
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

/// Read superblock from root metadata pair.
#[allow(dead_code)]
pub fn read_superblock<B: BlockDevice>(
    bd: &B,
    config: &Config,
    root: [u32; 2],
) -> Result<Superblock, Error> {
    let dir = fetch_metadata_pair(bd, config, root)?;
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
    use crate::fs::format;

    fn formatted_bd() -> (RamBlockDevice, Config) {
        let config = Config::default_for_tests(128);
        let bd = RamBlockDevice::new(config.block_size, config.block_count);
        format::format(&bd, &config).unwrap();
        (bd, config)
    }

    #[test]
    fn fetch_metadata_pair_after_format() {
        let (bd, config) = formatted_bd();
        let dir = fetch_metadata_pair(&bd, &config, [0, 1]).unwrap();
        assert_eq!(dir.pair[0], 0);
        assert_eq!(dir.count, 1);
        assert!(!dir.split);
    }

    #[test]
    fn get_tag_backwards_inlinestruct_id0() {
        let (bd, config) = formatted_bd();
        let dir = fetch_metadata_pair(&bd, &config, [0, 1]).unwrap();
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
        let dir = fetch_metadata_pair(&bd, &config, [0, 1]).unwrap();
        let info = get_entry_info(&dir, 0, 255).unwrap();
        assert_eq!(info.name().unwrap(), "littlefs");
    }

    #[test]
    fn read_superblock_after_format() {
        let (bd, config) = formatted_bd();
        let sb = read_superblock(&bd, &config, [0, 1]).unwrap();
        assert_eq!(sb.block_size, config.block_size);
        assert_eq!(sb.block_count, config.block_count);
        assert_eq!(sb.name_max, 255);
    }
}
