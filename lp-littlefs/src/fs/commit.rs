//! Directory commit machinery.
//!
//! Appends tags to metadata blocks. Per lfs_dir_commitattr, lfs_dir_commitcrc,
//! lfs_dir_alloc (lfs.c).

use ::alloc::vec::Vec;

use crate::block::BlockDevice;
use crate::config::Config;
use crate::crc;
use crate::error::Error;
use crate::superblock::tag;
use crate::trace;

use super::alloc::{self, Lookahead};
use super::metadata::MdDir;

fn mktag(type_: u32, id: u32, size: u32) -> u32 {
    (type_ << 20) | (id << 10) | size
}

/// Data for a commit attribute.
pub enum CommitData<'a> {
    None,
    Slice(&'a [u8]),
    Pair([u32; 2]),
}

/// A single attribute to commit.
pub struct CommitAttr<'a> {
    pub tag: u32,
    pub data: CommitData<'a>,
}

impl<'a> CommitAttr<'a> {
    pub fn create(id: u16) -> Self {
        Self {
            tag: mktag(tag::TYPE_CREATE, id as u32, 0),
            data: CommitData::None,
        }
    }

    pub fn delete(id: u16) -> Self {
        Self {
            tag: mktag(tag::TYPE_DELETE, id as u32, 0),
            data: CommitData::None,
        }
    }

    pub fn name_dir(id: u16, name: &'a [u8]) -> Self {
        let size = name.len().min(255);
        Self {
            tag: mktag(tag::TYPE_DIR, id as u32, size as u32),
            data: CommitData::Slice(&name[..size]),
        }
    }

    #[allow(dead_code)]
    pub fn name_reg(id: u16, name: &'a [u8]) -> Self {
        let size = name.len().min(255);
        Self {
            tag: mktag(tag::TYPE_REG, id as u32, size as u32),
            data: CommitData::Slice(&name[..size]),
        }
    }

    pub fn dir_struct(id: u16, pair: [u32; 2]) -> Self {
        Self {
            tag: mktag(tag::TYPE_DIRSTRUCT, id as u32, 8),
            data: CommitData::Pair(pair),
        }
    }

    pub fn soft_tail(pair: [u32; 2]) -> Self {
        Self {
            tag: mktag(tag::TYPE_SOFTTAIL, 0x3ff, 8),
            data: CommitData::Pair(pair),
        }
    }

    #[allow(dead_code)]
    pub fn hard_tail(pair: [u32; 2]) -> Self {
        Self {
            tag: mktag(tag::TYPE_HARDTAIL, 0x3ff, 8),
            data: CommitData::Pair(pair),
        }
    }
}

fn tag_dsize(tag: u32) -> usize {
    let size = tag & 0x3ff;
    let data_size = if size == 0x3ff { 0 } else { size as usize };
    4 + data_size
}

fn tag_chunk(tag: u32) -> u8 {
    ((tag & 0x0ff0_0000) >> 20) as u8
}

fn attr_data_bytes(attr: &CommitAttr<'_>) -> Vec<u8> {
    match &attr.data {
        CommitData::None => Vec::new(),
        CommitData::Slice(s) => s.to_vec(),
        CommitData::Pair(p) => [p[0].to_le_bytes(), p[1].to_le_bytes()]
            .into_iter()
            .flatten()
            .collect(),
    }
}

/// Apply attr to dir state (count, tail, split). Does not persist.
fn apply_attr_to_state(dir: &mut MdDir, attr: &CommitAttr<'_>) {
    let type3 = (attr.tag >> 20) & 0x7ff;
    if type3 == tag::TYPE_CREATE {
        dir.count = dir.count.saturating_add(1);
    } else if type3 == tag::TYPE_DELETE {
        dir.count = dir.count.saturating_sub(1);
    } else if type3 == tag::TYPE_SOFTTAIL || type3 == tag::TYPE_HARDTAIL {
        if let CommitData::Pair(p) = &attr.data {
            dir.tail = *p;
            dir.split = type3 == tag::TYPE_HARDTAIL;
        }
    }
}

/// Allocate a new empty metadata pair.
pub fn dir_alloc<B: BlockDevice>(
    bd: &B,
    config: &Config,
    root: [u32; 2],
    lookahead: &mut Lookahead,
) -> Result<MdDir, Error> {
    let block_size = config.block_size as usize;

    let b1 = alloc::alloc(bd, config, root, lookahead)?;
    let b0 = alloc::alloc(bd, config, root, lookahead)?;

    bd.erase(b0)?;
    bd.erase(b1)?;

    let pair = [b0, b1];
    Ok(MdDir::alloc_empty(pair, block_size))
}

/// Append attributes to a directory. Returns Err(Nospc) if block is full.
/// For a freshly allocated dir (off==4), writes revision 1 first.
pub fn dir_commit_append<B: BlockDevice>(
    bd: &B,
    config: &Config,
    dir: &mut MdDir,
    attrs: &[CommitAttr<'_>],
) -> Result<(), Error> {
    trace!(
        "dir_commit_append pair={:?} n_attrs={} off={}",
        dir.pair,
        attrs.len(),
        dir.off
    );
    let block_size = config.block_size as usize;
    let prog_size = config.prog_size as usize;
    let block_idx = dir.pair[0];

    let mut off = dir.off;
    let mut ptag = dir.etag;
    let mut crc = 0xffff_ffffu32;

    if off == 4 {
        let mut rev_buf = Vec::with_capacity(prog_size);
        rev_buf.resize(prog_size, 0xff);
        rev_buf[0..4].copy_from_slice(&1u32.to_le_bytes());
        bd.prog(block_idx, 0, &rev_buf)?;
        crc = crc::crc32(crc, &1u32.to_le_bytes());
    }

    for attr in attrs {
        let dsize = tag_dsize(attr.tag);
        if off + dsize > block_size - 8 {
            return Err(Error::Nospc);
        }

        let ntag = (attr.tag & 0x7fff_ffff) ^ ptag;
        let ntag_be = ntag.to_be_bytes();
        bd.prog(block_idx, off as u32, &ntag_be)?;
        crc = crc::crc32(crc, &ntag_be);
        off += 4;

        let data = attr_data_bytes(attr);
        if data.len() >= dsize - 4 {
            bd.prog(block_idx, off as u32, &data[..dsize - 4])?;
            crc = crc::crc32(crc, &data[..dsize - 4]);
        }
        off += dsize - 4;
        ptag = attr.tag & 0x7fff_ffff;

        apply_attr_to_state(dir, attr);
    }

    // CRC tag + value (8 bytes) must fit. Per lfs.c lfs_dir_commitcrc: write
    // immediately after last attr; no padding between tags (parser expects
    // next tag contiguously).
    if off + 8 > block_size {
        return Err(Error::Nospc);
    }

    let crc_tag = mktag(tag::TYPE_CCRC, 0x3ff, 4);
    let stored_crc_tag = (crc_tag & 0x7fff_ffff) ^ ptag;
    bd.prog(block_idx, off as u32, &stored_crc_tag.to_be_bytes())?;
    crc = crc::crc32(crc, &stored_crc_tag.to_be_bytes());
    off += 4;
    bd.prog(block_idx, off as u32, &crc.to_le_bytes())?;
    off += 4;

    dir.off = off;
    dir.etag = (crc_tag & 0x7fff_ffff) ^ ((tag_chunk(crc_tag) & 1) as u32) << 31;

    trace!("dir_commit_append done new_off={} count={}", off, dir.count);
    Ok(())
}
