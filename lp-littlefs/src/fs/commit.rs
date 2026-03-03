//! Directory commit machinery.
//!
//! Appends tags to metadata blocks. Per lfs_dir_commitattr (lfs.c:1621),
//! lfs_dir_commitcrc (lfs.c:1669), lfs_dir_alloc (lfs.c:1815).

use ::alloc::vec::Vec;

use crate::block::BlockDevice;
use crate::crc;
use crate::error::Error;
use crate::superblock::tag;
use crate::trace;

use super::alloc::{self, Lookahead};
use super::bdcache::BdContext;
use super::gstate::{self, GState};
use super::metadata::{dir_traverse_size, dir_traverse_tags, MdDir, TraverseAction};
use super::parent::{fs_parent, fs_pred};

/// GState context for commit: gstate (read), gdisk and gdelta (read-write).
/// When root is Some, only update gdisk when committing to that pair, so
/// MOVESTATE is persisted to the traversed chain (mount follows root tail).
pub struct GStateCtx<'a> {
    pub gstate: &'a GState,
    pub gdisk: &'a mut GState,
    pub gdelta: &'a mut GState,
    /// If Some, gdisk is updated only when committing to this pair.
    pub root: Option<[u32; 2]>,
    /// When true, skip dir_getgstate adjustment (for explicit persist e.g. mkconsistent).
    pub skip_dir_adjust: bool,
}

fn mktag(type_: u32, id: u32, size: u32) -> u32 {
    (type_ << 20) | (id << 10) | size
}

/// Data for a commit attribute.
pub enum CommitData<'a> {
    None,
    Slice(&'a [u8]),
    Pair([u32; 2]),
    CtzStruct { head: u32, size: u32 },
    GState(super::gstate::GState),
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

    pub fn inline_struct(id: u16, data: &'a [u8]) -> Self {
        let size = data.len().min(0x3fe);
        Self {
            tag: mktag(tag::TYPE_INLINESTRUCT, id as u32, size as u32),
            data: CommitData::Slice(&data[..size]),
        }
    }

    pub fn ctz_struct(id: u16, head: u32, size: u32) -> Self {
        Self {
            tag: mktag(tag::TYPE_CTZSTRUCT, id as u32, 8),
            data: CommitData::CtzStruct { head, size },
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

    /// MOVESTATE tag for gstate persistence. Per lfs.c LFS_TYPE_MOVESTATE.
    pub fn movestate(gstate: &super::gstate::GState) -> Self {
        Self {
            tag: mktag(tag::TYPE_MOVESTATE, 0x3ff, 12),
            data: CommitData::GState(*gstate),
        }
    }
}

fn tag_dsize(tag: u32) -> usize {
    let size = tag & 0x3ff;
    let data_size = if size == 0x3ff { 0 } else { size as usize };
    4 + data_size
}

fn attr_data_bytes(attr: &CommitAttr<'_>) -> Vec<u8> {
    match &attr.data {
        CommitData::None => Vec::new(),
        CommitData::Slice(s) => s.to_vec(),
        CommitData::Pair(p) => [p[0].to_le_bytes(), p[1].to_le_bytes()]
            .into_iter()
            .flatten()
            .collect(),
        CommitData::CtzStruct { head, size } => [head.to_le_bytes(), size.to_le_bytes()]
            .into_iter()
            .flatten()
            .collect(),
        CommitData::GState(g) => g.as_le_bytes().to_vec(),
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
/// Per lfs_dir_alloc (lfs.c:1815).
pub fn dir_alloc<B: BlockDevice>(
    ctx: &BdContext<'_, B>,
    root: [u32; 2],
    lookahead: &mut Lookahead,
) -> Result<MdDir, Error> {
    let block_size = ctx.config.block_size as usize;

    let b1 = alloc::alloc(ctx, root, lookahead)?;
    let b0 = alloc::alloc(ctx, root, lookahead)?;

    ctx.erase(b0)?;
    ctx.erase(b1)?;

    let pair = [b0, b1];
    Ok(MdDir::alloc_empty(pair, block_size))
}

/// Append attributes to a directory. Returns Err(Nospc) if block is full.
/// For a freshly allocated dir (off==4), writes revision 1 first.
/// Per lfs_dir_commitattr (lfs.c:1621), lfs_dir_commitcrc (lfs.c:1669).
pub fn dir_commit_append<B: BlockDevice>(
    ctx: &BdContext<'_, B>,
    dir: &mut MdDir,
    attrs: &[CommitAttr<'_>],
    gstate_ctx: &mut Option<&mut GStateCtx<'_>>,
    disk_version: u32,
) -> Result<(), Error> {
    trace!(
        "dir_commit_append pair={:?} writing to block={} at off={} n_attrs={}",
        dir.pair,
        dir.pair[0],
        dir.off,
        attrs.len()
    );
    let block_size = ctx.config.block_size as usize;
    let prog_size = ctx.config.prog_size as usize;
    let block_idx = dir.pair[0];

    let begin = if dir.off == 4 { 0 } else { dir.off };
    let mut off = dir.off;
    let mut ptag = dir.etag;
    let mut crc = 0xffff_ffffu32;

    if off == 4 {
        let new_rev = dir.rev + 1;
        let mut rev_buf = Vec::with_capacity(prog_size);
        rev_buf.resize(prog_size, 0xff);
        rev_buf[0..4].copy_from_slice(&new_rev.to_le_bytes());
        ctx.prog(block_idx, 0, &rev_buf)?;
        crc = crc::crc32(crc, &new_rev.to_le_bytes());
        dir.rev = new_rev;
    }

    const CRC_MIN: usize = 20;
    for attr in attrs {
        let dsize = tag_dsize(attr.tag);
        if off + dsize > block_size - CRC_MIN {
            return Err(Error::Nospc);
        }

        let ntag = (attr.tag & 0x7fff_ffff) ^ ptag;
        let ntag_be = ntag.to_be_bytes();
        ctx.prog(block_idx, off as u32, &ntag_be)?;
        crc = crc::crc32(crc, &ntag_be);
        off += 4;

        let data = attr_data_bytes(attr);
        if data.len() >= dsize - 4 {
            ctx.prog(block_idx, off as u32, &data[..dsize - 4])?;
            crc = crc::crc32(crc, &data[..dsize - 4]);
        }
        off += dsize - 4;
        ptag = attr.tag & 0x7fff_ffff;

        apply_attr_to_state(dir, attr);
    }

    if let Some(gctx) = gstate_ctx.as_mut() {
        if let Some(delta) = gstate::compute_movestate_delta(
            dir,
            gctx.gstate,
            gctx.gdisk,
            gctx.gdelta,
            false,
            gctx.skip_dir_adjust,
        )? {
            let movestate_attr = CommitAttr::movestate(&delta);
            let dsize = tag_dsize(movestate_attr.tag);
            if off + dsize > block_size - CRC_MIN {
                return Err(Error::Nospc);
            }
            let ntag = (movestate_attr.tag & 0x7fff_ffff) ^ ptag;
            ctx.prog(block_idx, off as u32, &ntag.to_be_bytes())?;
            crc = crc::crc32(crc, &ntag.to_be_bytes());
            off += 4;
            let data = attr_data_bytes(&movestate_attr);
            ctx.prog(block_idx, off as u32, &data)?;
            crc = crc::crc32(crc, &data);
            off += dsize - 4;
            ptag = movestate_attr.tag & 0x7fff_ffff;

            if gctx.root.is_none_or(|r| pair_eq(dir.pair, r)) {
                *gctx.gdisk = *gctx.gstate;
            }
            *gctx.gdelta = GState::zero();
        }
    }

    if off + CRC_MIN > block_size {
        return Err(Error::Nospc);
    }

    dir_commit_crc(
        ctx,
        block_idx,
        &mut off,
        &mut ptag,
        &mut crc,
        begin,
        block_size,
        prog_size,
        disk_version,
    )?;

    dir.off = off;
    dir.etag = ptag;

    trace!("dir_commit_append done new_off={} count={}", off, dir.count);
    Ok(())
}

/// Result of dir_compact: Ok(()) or Ok(Relocated) when block was relocated.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum CompactResult {
    Ok,
    Relocated,
}

/// Result of dir_relocatingcommit.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum RelocatingResult {
    Ok,
    Relocated,
    Dropped,
}

fn metadata_max(ctx: &BdContext<'_, impl BlockDevice>) -> usize {
    if ctx.config.metadata_max != 0 {
        ctx.config.metadata_max as usize
    } else {
        ctx.config.block_size as usize
    }
}

fn dir_needsrelocation(ctx: &BdContext<'_, impl BlockDevice>, dir: &MdDir) -> bool {
    if ctx.config.block_cycles <= 0 {
        return false;
    }
    let modulus = ((ctx.config.block_cycles + 1) | 1) as u32;
    (dir.rev + 1).is_multiple_of(modulus)
}

fn pair_eq(a: [u32; 2], b: [u32; 2]) -> bool {
    a[0] == b[0] && a[1] == b[1]
}

fn alignup_usize(a: usize, alignment: usize) -> usize {
    if alignment == 0 {
        return a;
    }
    (a + alignment - 1) & !(alignment - 1)
}

/// CRC phase of a commit. Per lfs_dir_commitcrc (lfs.c:1669).
/// Aligns end to prog_size, writes FCRC when space allows, CCRC blocks, verifies.
fn dir_commit_crc<B: BlockDevice>(
    ctx: &BdContext<'_, B>,
    block_idx: u32,
    off: &mut usize,
    ptag: &mut u32,
    crc: &mut u32,
    begin: usize,
    block_size: usize,
    prog_size: usize,
    disk_version: u32,
) -> Result<(), Error> {
    let end = alignup_usize((*off + 20).min(block_size), prog_size);

    let mut off1 = 0usize;
    let mut crc1 = 0u32;

    while *off < end {
        let off_after_tag = *off + 4;
        let mut noff = (end - off_after_tag).min(0x3fe) + off_after_tag;
        if noff < end {
            noff = noff.min(end - 20);
        }

        let mut eperturb: u8 = 0xff;
        if noff >= end && noff <= block_size - prog_size {
            let mut buf = [0u8; 1];
            if ctx.read(block_idx, noff as u32, &mut buf).is_ok() {
                eperturb = buf[0];
            }

            if disk_version > 0x0002_0000 {
                let fcrc_crc = ctx.crc(block_idx, noff as u32, prog_size, 0xffff_ffff)?;
                let fcrc_tag = mktag(tag::TYPE_FCRC, 0x3ff, 8);
                let fcrc_stored_tag = (fcrc_tag & 0x7fff_ffff) ^ *ptag;
                let mut fcrc_data = [0u8; 8];
                fcrc_data[0..4].copy_from_slice(&(prog_size as u32).to_le_bytes());
                fcrc_data[4..8].copy_from_slice(&fcrc_crc.to_le_bytes());

                ctx.prog(block_idx, *off as u32, &fcrc_stored_tag.to_be_bytes())?;
                *crc = crc::crc32(*crc, &fcrc_stored_tag.to_be_bytes());
                *off += 4;
                ctx.prog(block_idx, *off as u32, &fcrc_data)?;
                *crc = crc::crc32(*crc, &fcrc_data);
                *off += 8;
                *ptag = fcrc_tag & 0x7fff_ffff;
            }
        }

        let ccrc_size = noff - (*off + 4);
        let ccrc_type = tag::TYPE_CCRC + (((!eperturb) >> 7) as u32);
        let ccrc_tag = mktag(ccrc_type, 0x3ff, ccrc_size as u32);
        let ccrc_stored_tag = (ccrc_tag & 0x7fff_ffff) ^ *ptag;
        *crc = crc::crc32(*crc, &ccrc_stored_tag.to_be_bytes());
        let stored_crc = *crc;

        ctx.prog(block_idx, *off as u32, &ccrc_stored_tag.to_be_bytes())?;
        *off += 4;
        // off1 = start of stored CRC (for verify: end of CRC'd region; for read: position of CRC)
        if off1 == 0 {
            off1 = *off;
            crc1 = stored_crc;
        }
        ctx.prog(block_idx, *off as u32, &stored_crc.to_le_bytes())?;
        *off += 4;

        *ptag = (ccrc_tag & 0x7fff_ffff) ^ ((0x80u32 & !(eperturb as u32)) << 24);
        *crc = 0xffff_ffff;
        *off = noff;

        if noff >= end {
            ctx.sync()?;
        }
    }

    let verify_len = off1.saturating_sub(begin);
    if verify_len > 0 {
        let crc_verify = ctx.crc(block_idx, begin as u32, verify_len, 0xffff_ffff)?;
        if crc_verify != crc1 {
            return Err(Error::Corrupt);
        }
    }
    let mut stored_buf = [0u8; 4];
    ctx.read(block_idx, off1 as u32, &mut stored_buf)?;
    let stored_crc_val = u32::from_le_bytes(stored_buf);
    if stored_crc_val == 0 {
        return Err(Error::Corrupt);
    }

    Ok(())
}

/// Compact source tags [begin, end) and optional attrs into dir. Writes to dir.pair[1], then swaps.
/// Attrs are appended after source tags (for relocatingcommit merge).
/// On Nospc or Corrupt, relocates to a new block and retries.
/// Returns Relocated if a relocation occurred.
/// Per lfs_dir_compact (lfs.c:1952).
pub fn dir_compact<B: BlockDevice>(
    bdc: &BdContext<'_, B>,
    dir: &mut MdDir,
    source: &MdDir,
    begin: u16,
    end: u16,
    attrs: &[CommitAttr<'_>],
    root: [u32; 2],
    lookahead: &mut Lookahead,
    gstate_ctx: &mut Option<&mut GStateCtx<'_>>,
    disk_version: u32,
) -> Result<CompactResult, Error> {
    let meta_max = metadata_max(bdc);
    let prog_size = bdc.config.prog_size as usize;
    let mut relocated = false;
    const SUPERBLOCK: [u32; 2] = [0, 1];

    dir.rev += 1;

    loop {
        let block_idx = dir.pair[1];

        bdc.erase(block_idx)?;

        let new_rev = dir.rev;
        let rev_buf = new_rev.to_le_bytes();
        let mut rev_prog = ::alloc::vec![0xff; prog_size];
        rev_prog[0..4].copy_from_slice(&rev_buf);
        if let Err(e) = bdc.prog(block_idx, 0, &rev_prog) {
            if matches!(e, Error::Corrupt) {
                relocated = true;
            }
            if pair_eq(dir.pair, SUPERBLOCK) && relocated {
                return Err(Error::Nospc);
            }
            if let Error::Corrupt = e {
                continue;
            }
            return Err(e);
        }

        let mut off = 4usize;
        let mut ptag: u32 = 0xffff_ffff;
        let mut crc = crate::crc::crc32(0xffff_ffff, &rev_buf);

        const CRC_MIN: usize = 20;
        let traverse_res =
            dir_traverse_tags(source, begin, end, -(begin as i32), true, |tag, data| {
                let dsize = tag_dsize(tag);
                if off + dsize > meta_max - CRC_MIN {
                    return Err(Error::Nospc);
                }
                let ntag = (tag & 0x7fff_ffff) ^ ptag;
                let ntag_be = ntag.to_be_bytes();
                bdc.prog(block_idx, off as u32, &ntag_be).map_err(|e| {
                    if matches!(e, Error::Corrupt) {
                        Error::Corrupt
                    } else {
                        e
                    }
                })?;
                crc = crate::crc::crc32(crc, &ntag_be);
                off += 4;
                if !data.is_empty() {
                    bdc.prog(block_idx, off as u32, data)?;
                    crc = crate::crc::crc32(crc, data);
                    off += data.len();
                }
                ptag = tag & 0x7fff_ffff;
                Ok(TraverseAction::Continue)
            });

        if let Err(e) = traverse_res {
            if matches!(e, Error::Nospc | Error::Corrupt) {
                relocated = true;
                if pair_eq(dir.pair, SUPERBLOCK) {
                    return Err(Error::Nospc);
                }
                let new_block = alloc::alloc(bdc, root, lookahead)?;
                dir.pair[1] = new_block;
                continue;
            }
            return Err(e);
        }

        let mut need_relocate = false;
        for attr in attrs {
            let dsize = tag_dsize(attr.tag);
            if off + dsize > meta_max - CRC_MIN {
                need_relocate = true;
                break;
            }
            let data = attr_data_bytes(attr);
            let ntag = (attr.tag & 0x7fff_ffff) ^ ptag;
            match bdc.prog(block_idx, off as u32, &ntag.to_be_bytes()) {
                Err(Error::Corrupt) => {
                    need_relocate = true;
                    break;
                }
                Err(e) => return Err(e),
                Ok(()) => {}
            }
            crc = crate::crc::crc32(crc, &ntag.to_be_bytes());
            off += 4;
            if !data.is_empty() {
                bdc.prog(block_idx, off as u32, &data)?;
                crc = crate::crc::crc32(crc, &data);
                off += data.len();
            }
            ptag = attr.tag & 0x7fff_ffff;
        }
        if need_relocate {
            relocated = true;
            if pair_eq(dir.pair, SUPERBLOCK) {
                return Err(Error::Nospc);
            }
            let new_block = alloc::alloc(bdc, root, lookahead)?;
            dir.pair[1] = new_block;
            continue;
        }

        if !dir.tail_is_null() {
            let tail_tag = mktag(
                if dir.split {
                    tag::TYPE_HARDTAIL
                } else {
                    tag::TYPE_SOFTTAIL
                },
                0x3ff,
                8,
            );
            let tail_data = [dir.tail[0].to_le_bytes(), dir.tail[1].to_le_bytes()]
                .into_iter()
                .flatten()
                .collect::<Vec<_>>();
            let dsize = tag_dsize(tail_tag);
            if off + dsize > meta_max - CRC_MIN {
                relocated = true;
                if pair_eq(dir.pair, SUPERBLOCK) {
                    return Err(Error::Nospc);
                }
                let new_block = alloc::alloc(bdc, root, lookahead)?;
                dir.pair[1] = new_block;
                continue;
            }
            let ntag = (tail_tag & 0x7fff_ffff) ^ ptag;
            bdc.prog(block_idx, off as u32, &ntag.to_be_bytes())?;
            crc = crate::crc::crc32(crc, &ntag.to_be_bytes());
            off += 4;
            bdc.prog(block_idx, off as u32, &tail_data)?;
            crc = crate::crc::crc32(crc, &tail_data);
            off += 8;
            ptag = tail_tag & 0x7fff_ffff;
        }

        if let Some(ctx) = gstate_ctx.as_mut() {
            if let Some(delta) = gstate::compute_movestate_delta(
                dir,
                ctx.gstate,
                ctx.gdisk,
                ctx.gdelta,
                relocated,
                ctx.skip_dir_adjust,
            )? {
                let movestate_attr = CommitAttr::movestate(&delta);
                let dsize = tag_dsize(movestate_attr.tag);
                if off + dsize > meta_max - CRC_MIN {
                    relocated = true;
                    if pair_eq(dir.pair, SUPERBLOCK) {
                        return Err(Error::Nospc);
                    }
                    let new_block = alloc::alloc(bdc, root, lookahead)?;
                    dir.pair[1] = new_block;
                    continue;
                }
                let data = attr_data_bytes(&movestate_attr);
                let ntag = (movestate_attr.tag & 0x7fff_ffff) ^ ptag;
                bdc.prog(block_idx, off as u32, &ntag.to_be_bytes())?;
                crc = crate::crc::crc32(crc, &ntag.to_be_bytes());
                off += 4;
                bdc.prog(block_idx, off as u32, &data)?;
                crc = crate::crc::crc32(crc, &data);
                off += dsize - 4;
                ptag = movestate_attr.tag & 0x7fff_ffff;

                if !relocated && ctx.root.is_none_or(|r| pair_eq(dir.pair, r)) {
                    *ctx.gdisk = *ctx.gstate;
                }
                *ctx.gdelta = GState::zero();
            }
        }

        if off + CRC_MIN > meta_max {
            relocated = true;
            if pair_eq(dir.pair, SUPERBLOCK) {
                return Err(Error::Nospc);
            }
            let new_block = alloc::alloc(bdc, root, lookahead)?;
            dir.pair[1] = new_block;
            continue;
        }
        let block_size = bdc.config.block_size as usize;
        if let Err(e) = dir_commit_crc(
            bdc,
            block_idx,
            &mut off,
            &mut ptag,
            &mut crc,
            0,
            block_size,
            prog_size,
            disk_version,
        ) {
            if matches!(e, Error::Corrupt) {
                relocated = true;
                if pair_eq(dir.pair, SUPERBLOCK) {
                    return Err(Error::Nospc);
                }
                let new_block = alloc::alloc(bdc, root, lookahead)?;
                dir.pair[1] = new_block;
                continue;
            }
            return Err(e);
        }

        dir.pair.swap(0, 1);
        if attrs.is_empty() {
            dir.count = end.saturating_sub(begin);
        }
        dir.off = off;
        dir.etag = ptag;

        return Ok(if relocated {
            CompactResult::Relocated
        } else {
            CompactResult::Ok
        });
    }
}

/// Split dir: compact source [split, end) into new tail pair; set dir.tail and dir.split.
pub fn dir_split<B: BlockDevice>(
    ctx: &BdContext<'_, B>,
    dir: &mut MdDir,
    source: &MdDir,
    split: u16,
    end: u16,
    root: [u32; 2],
    lookahead: &mut Lookahead,
    gstate_ctx: &mut Option<&mut GStateCtx<'_>>,
    disk_version: u32,
) -> Result<(), Error> {
    let mut tail = dir_alloc(ctx, root, lookahead)?;
    tail.split = dir.split;
    tail.tail = dir.tail;

    dir_compact(
        ctx,
        &mut tail,
        source,
        split,
        end,
        &[],
        root,
        lookahead,
        gstate_ctx,
        disk_version,
    )?;

    dir.tail = tail.pair;
    dir.split = true;

    Ok(())
}

/// Splitting compact: binary-search split point until metadata fits, then compact.
/// Returns Relocated if dir_compact relocated.
pub fn dir_splittingcompact<B: BlockDevice>(
    ctx: &BdContext<'_, B>,
    dir: &mut MdDir,
    source: &MdDir,
    begin: u16,
    end: u16,
    attrs: &[CommitAttr<'_>],
    root: [u32; 2],
    lookahead: &mut Lookahead,
    gstate_ctx: &mut Option<&mut GStateCtx<'_>>,
    disk_version: u32,
) -> Result<CompactResult, Error> {
    let meta_max = metadata_max(ctx);
    let prog_size = ctx.config.prog_size as usize;

    let mut split = begin;

    while end.saturating_sub(split) > 1 {
        let size = dir_traverse_size(source, split, end)?;
        let cap = (meta_max - 40).min(((meta_max / 2) + prog_size - 1) & !(prog_size - 1));
        if end - split < 0xff && size <= cap {
            break;
        }
        split = split + (end - split) / 2;
    }

    if split != begin {
        let res = dir_split(
            ctx,
            dir,
            source,
            split,
            end,
            root,
            lookahead,
            gstate_ctx,
            disk_version,
        );
        if let Err(Error::Nospc) = res {
            trace!("dir_splittingcompact: split failed (Nospc), compact with degraded split");
        } else {
            res?;
            return dir_splittingcompact(
                ctx,
                dir,
                source,
                begin,
                split,
                attrs,
                root,
                lookahead,
                gstate_ctx,
                disk_version,
            );
        }
    }

    dir_compact(
        ctx,
        dir,
        source,
        begin,
        end,
        attrs,
        root,
        lookahead,
        gstate_ctx,
        disk_version,
    )
}

/// Relocating commit: try inline append, else splittingcompact.
/// Returns Relocated if block was relocated, Dropped if dir was dropped (empty with split pred).
/// Per lfs_dir_relocatingcommit (lfs.c:2234).
pub fn dir_relocatingcommit<B: BlockDevice>(
    ctx: &BdContext<'_, B>,
    dir: &mut MdDir,
    _pair: [u32; 2],
    attrs: &[CommitAttr<'_>],
    root: [u32; 2],
    lookahead: &mut Lookahead,
    pdir: &mut MdDir,
    gstate_ctx: &mut Option<&mut GStateCtx<'_>>,
    disk_version: u32,
) -> Result<RelocatingResult, Error> {
    let mut hasdelete = false;
    for attr in attrs {
        apply_attr_to_state(dir, attr);
        let type3 = (attr.tag >> 20) & 0x7ff;
        if type3 == tag::TYPE_DELETE {
            hasdelete = true;
        }
    }

    if hasdelete && dir.count == 0 {
        if let Some((pred, _)) = fs_parent(ctx, root, dir.pair, 255)? {
            *pdir = pred;
            if pdir.split {
                return Ok(RelocatingResult::Dropped);
            }
        }
    }

    let force_compact = dir_needsrelocation(ctx, dir);
    if !force_compact {
        match dir_commit_append(ctx, dir, attrs, gstate_ctx, disk_version) {
            Ok(()) => return Ok(RelocatingResult::Ok),
            Err(Error::Nospc) | Err(Error::Corrupt) => {}
            Err(e) => return Err(e),
        }
    }

    let source = dir.clone();
    let res = dir_splittingcompact(
        ctx,
        dir,
        &source,
        0,
        source.count,
        attrs,
        root,
        lookahead,
        gstate_ctx,
        disk_version,
    )?;

    Ok(if res == CompactResult::Relocated {
        RelocatingResult::Relocated
    } else {
        RelocatingResult::Ok
    })
}

/// Orphaning commit: relocatingcommit plus relocation chain fixup.
/// Per lfs_dir_orphaningcommit (lfs.c:2408).
pub fn dir_orphaningcommit<B: BlockDevice>(
    ctx: &BdContext<'_, B>,
    dir: &mut MdDir,
    attrs: &[CommitAttr<'_>],
    root: &mut [u32; 2],
    lookahead: &mut Lookahead,
    name_max: u32,
    gstate: &GState,
    gdisk: &mut GState,
    gdelta: &mut GState,
    skip_dir_adjust: bool,
    disk_version: u32,
) -> Result<bool, Error> {
    trace!(
        "dir_orphaningcommit pair={:?} n_attrs={}",
        dir.pair,
        attrs.len()
    );
    let lpair = dir.pair;
    let mut ldir = dir.clone();
    let mut pdir = MdDir::alloc_empty([0, 0], ctx.config.block_size as usize);
    let mut dummy_pdir = MdDir::alloc_empty([0, 0], ctx.config.block_size as usize);
    let mut gstate_ctx = GStateCtx {
        gstate,
        gdisk,
        gdelta,
        root: Some(*root),
        skip_dir_adjust,
    };
    let mut gstate_opt = Some(&mut gstate_ctx);

    let mut state = dir_relocatingcommit(
        ctx,
        &mut ldir,
        lpair,
        attrs,
        *root,
        lookahead,
        &mut pdir,
        &mut gstate_opt,
        disk_version,
    )?;

    if lpair[0] == dir.pair[0] && lpair[1] == dir.pair[1] {
        *dir = ldir.clone();
    }

    let mut orphans = false;
    if state == RelocatingResult::Dropped {
        let pdir_pair = pdir.pair;
        let steal_attrs = [if dir.split {
            CommitAttr::hard_tail(dir.tail)
        } else {
            CommitAttr::soft_tail(dir.tail)
        }];
        state = dir_relocatingcommit(
            ctx,
            &mut pdir,
            pdir_pair,
            &steal_attrs,
            *root,
            lookahead,
            &mut dummy_pdir,
            &mut gstate_opt,
            disk_version,
        )?;
        ldir = pdir;
    }

    while state == RelocatingResult::Relocated {
        if lpair[0] == root[0] && lpair[1] == root[1] {
            root[0] = ldir.pair[0];
            root[1] = ldir.pair[1];
        }

        let _parent_state =
            if let Some((mut pdir, tag_id)) = fs_parent(ctx, *root, lpair, name_max)? {
                let ppair = pdir.pair;
                let ldir_pair = ldir.pair;
                let update_attrs = [CommitAttr::dir_struct(tag_id, ldir_pair)];
                let s = dir_relocatingcommit(
                    ctx,
                    &mut pdir,
                    ppair,
                    &update_attrs,
                    *root,
                    lookahead,
                    &mut dummy_pdir,
                    &mut gstate_opt,
                    disk_version,
                )?;
                if s == RelocatingResult::Relocated {
                    ldir = pdir;
                    orphans = true;
                    continue;
                }
                s
            } else {
                RelocatingResult::Ok
            };

        if let Some(mut pred) = fs_pred(ctx, *root, lpair)? {
            let pred_pair = pred.pair;
            let update_attrs = [if pred.split {
                CommitAttr::hard_tail(ldir.pair)
            } else {
                CommitAttr::soft_tail(ldir.pair)
            }];
            state = dir_relocatingcommit(
                ctx,
                &mut pred,
                pred_pair,
                &update_attrs,
                *root,
                lookahead,
                &mut dummy_pdir,
                &mut gstate_opt,
                disk_version,
            )?;
            ldir = pred;
        } else {
            break;
        }
    }

    Ok(orphans)
}

/// Drop orphan from metadata chain. Steals tail and gstate from orphan into pred.
/// Per lfs_dir_drop (lfs.c:1859).
pub fn dir_drop<B: BlockDevice>(
    ctx: &BdContext<'_, B>,
    pred: &mut MdDir,
    orphan: &MdDir,
    root: &mut [u32; 2],
    lookahead: &mut Lookahead,
    _name_max: u32,
    gstate: &GState,
    gdisk: &mut GState,
    gdelta: &mut GState,
    disk_version: u32,
) -> Result<(), Error> {
    super::gstate::dir_getgstate(orphan, gdelta)?;
    let tail_attr = if orphan.split {
        CommitAttr::hard_tail(orphan.tail)
    } else {
        CommitAttr::soft_tail(orphan.tail)
    };
    let mut dummy = MdDir::alloc_empty([0, 0], ctx.config.block_size as usize);
    let mut gstate_ctx = GStateCtx {
        gstate,
        gdisk,
        gdelta,
        root: Some(*root),
        skip_dir_adjust: false,
    };
    let mut gstate_opt = Some(&mut gstate_ctx);
    dir_relocatingcommit(
        ctx,
        pred,
        pred.pair,
        &[tail_attr],
        *root,
        lookahead,
        &mut dummy,
        &mut gstate_opt,
        disk_version,
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::block::RamBlockDevice;
    use crate::config::Config;
    use crate::fs::alloc::Lookahead;
    use crate::fs::bdcache::{self, BdContext};
    use crate::fs::format;
    use crate::fs::metadata::fetch_metadata_pair;
    use core::cell::RefCell;

    #[test]
    fn dir_compact_preserves_content() {
        let config = Config::default_for_tests(128);
        let bd = RamBlockDevice::new(config.block_size, config.block_count);
        format::format(&bd, &config).unwrap();

        let rcache = RefCell::new(bdcache::new_read_cache(&config).unwrap());
        let pcache = RefCell::new(bdcache::new_prog_cache(&config).unwrap());
        let ctx = BdContext::new(&bd, &config, &rcache, &pcache);
        let mut dir = fetch_metadata_pair(&ctx, [0u32, 1]).unwrap();
        let source = dir.clone();
        let mut lookahead = Lookahead::new(&config);
        dir_compact(
            &ctx,
            &mut dir,
            &source,
            0,
            1,
            &[],
            [0u32, 1],
            &mut lookahead,
            &mut None,
            crate::superblock::DISK_VERSION,
        )
        .unwrap();
        bdcache::bd_sync(&bd, &config, &rcache, &pcache).unwrap();

        let refetched = fetch_metadata_pair(&ctx, [0u32, 1]).unwrap();
        assert_eq!(refetched.count, 1);
        assert!(!refetched.split);
    }
}
