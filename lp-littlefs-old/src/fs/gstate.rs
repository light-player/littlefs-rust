//! Global state (gstate) for power-loss resilience.
//!
//! Per lfs.h lfs_gstate_t, lfs.c lfs_gstate_xor, lfs_dir_getgstate.
//! XOR-sum of MOVESTATE deltas across the metadata tail chain.
//! Tracks orphans (incomplete deletes) and pending moves.

use crate::error::Error;
use crate::superblock::tag;

/// Global state: tag and metadata pair. Per lfs_gstate_t.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct GState {
    pub tag: u32,
    pub pair: [u32; 2],
}

impl GState {
    pub const fn zero() -> Self {
        Self {
            tag: 0,
            pair: [0, 0],
        }
    }

    /// XOR other into self. Per lfs_gstate_xor.
    pub fn xor(&mut self, other: &GState) {
        self.tag ^= other.tag;
        self.pair[0] ^= other.pair[0];
        self.pair[1] ^= other.pair[1];
    }

    /// True if all fields are zero. Per lfs_gstate_iszero.
    pub fn iszero(&self) -> bool {
        self.tag == 0 && self.pair[0] == 0 && self.pair[1] == 0
    }

    /// True if tag has orphan count (tag_size non-zero). Per lfs_gstate_hasorphans.
    pub fn hasorphans(&self) -> bool {
        tag_size(self.tag) != 0
    }

    /// Orphan count from tag size (0x1ff mask). Per lfs_gstate_getorphans.
    pub fn getorphans(&self) -> u8 {
        (tag_size(self.tag) & 0x1ff) as u8
    }

    /// True if tag has move (type1 non-zero). Per lfs_gstate_hasmove.
    pub fn hasmove(&self) -> bool {
        tag_type1(self.tag) != 0
    }

    /// True if superblock needs rewrite (tag size high bits). Per lfs_gstate_needssuperblock.
    pub fn needssuperblock(&self) -> bool {
        (tag_size(self.tag) >> 9) != 0
    }

    /// True if move targets this pair. Per lfs_gstate_hasmovehere.
    /// Uses pair_issync: [a,b] matches [b,a].
    pub fn hasmovehere(&self, pair: [u32; 2]) -> bool {
        self.hasmove() && pair_issync(self.pair, pair)
    }

    /// Decode from little-endian. Per lfs_gstate_fromle32.
    pub fn from_le_bytes(data: &[u8]) -> Result<Self, Error> {
        if data.len() < 12 {
            crate::trace!(
                "GState::from_le_bytes Corrupt: data len {} < 12",
                data.len()
            );
            return Err(Error::Corrupt);
        }
        Ok(Self {
            tag: u32::from_le_bytes(data[0..4].try_into().unwrap()),
            pair: [
                u32::from_le_bytes(data[4..8].try_into().unwrap()),
                u32::from_le_bytes(data[8..12].try_into().unwrap()),
            ],
        })
    }

    /// Encode to little-endian. Per lfs_gstate_tole32.
    pub fn as_le_bytes(&self) -> [u8; 12] {
        let mut out = [0u8; 12];
        out[0..4].copy_from_slice(&self.tag.to_le_bytes());
        out[4..8].copy_from_slice(&self.pair[0].to_le_bytes());
        out[8..12].copy_from_slice(&self.pair[1].to_le_bytes());
        out
    }
}

/// True if pairs refer to the same metadata block. Per lfs_pair_issync.
/// [a,b] matches [a,b] and [b,a].
pub fn pair_issync(a: [u32; 2], b: [u32; 2]) -> bool {
    (a[0] == b[0] && a[1] == b[1]) || (a[0] == b[1] && a[1] == b[0])
}

fn tag_type1(t: u32) -> u32 {
    (t & 0x7000_0000) >> 20
}
fn tag_size(t: u32) -> u32 {
    t & 0x3ff
}

/// Fix invalid bit. Per lfs.c mount: lfs->gstate.tag += !lfs_tag_isvalid(lfs->gstate.tag)
fn tag_isvalid(t: u32) -> bool {
    (t & 0x8000_0000) == 0
}

/// Ensure gstate tag has valid bit set so it can be distinguished from empty.
pub fn ensure_valid(gstate: &mut GState) {
    if !tag_isvalid(gstate.tag) {
        gstate.tag = gstate.tag.wrapping_add(1);
    }
}

/// Set move in gstate. Per lfs_fs_prepmove. id=0x3ff clears the move.
pub fn prepmove(gstate: &mut GState, id: u16, pair: [u32; 2]) {
    gstate.tag = (gstate.tag & !(0x7ff << 20 | 0x3ff << 10))
        | ((id != 0x3ff) as u32 * (tag::TYPE_DELETE << 20 | (id as u32) << 10));
    gstate.pair[0] = if id != 0x3ff { pair[0] } else { 0 };
    gstate.pair[1] = if id != 0x3ff { pair[1] } else { 0 };
}

/// Set needssuperblock bit. Per lfs_fs_prepsuperblock.
pub fn prepsuperblock(gstate: &mut GState, needssuperblock: bool) {
    gstate.tag = (gstate.tag & !(0x200 << 9)) | ((needssuperblock as u32) << 9);
}

/// Adjust orphan count. Per lfs_fs_preporphans. Use negative to clear.
pub fn preporphans(gstate: &mut GState, delta: i8) -> Result<(), Error> {
    let size = tag_size(gstate.tag) as i32;
    let size_signed = (size << 22) >> 22;
    if size_signed + (delta as i32) < -0x1ff || size_signed + (delta as i32) > 0x1ff {
        return Err(Error::Corrupt);
    }
    gstate.tag = gstate.tag.wrapping_add(delta as u32);
    let has_orphans = tag_size(gstate.tag) != 0;
    gstate.tag = (gstate.tag & !(1 << 31)) | ((has_orphans as u32) << 31);
    Ok(())
}

/// Compute delta to write for MOVESTATE. Per lfs_dir_commit inline/compact.
/// Returns Some(delta) if non-zero, None otherwise.
/// Upstream only calls dir_getgstate when initial delta is non-zero (lfs.c:2305).
/// When skip_dir_adjust, omit dir_getgstate (for explicit persist e.g. mkconsistent).
pub fn compute_movestate_delta(
    dir: &super::metadata::MdDir,
    gstate: &GState,
    gdisk: &GState,
    gdelta: &GState,
    relocated: bool,
    skip_dir_adjust: bool,
) -> Result<Option<GState>, Error> {
    let mut delta = GState::zero();
    if !relocated {
        delta.xor(gdisk);
        delta.xor(gstate);
    }
    delta.xor(gdelta);
    delta.tag &= !0x3ff; // LFS_MKTAG(0, 0, 0x3ff) - clear lower 10 bits
    if delta.iszero() {
        return Ok(None);
    }
    if !skip_dir_adjust {
        dir_getgstate(dir, &mut delta)?;
    }
    Ok(if delta.iszero() { None } else { Some(delta) })
}

/// Accumulate MOVESTATE deltas from a metadata dir into gstate.
/// Per lfs_dir_getgstate (lfs.c:1395). XORs all MOVESTATE tags in the dir.
pub fn dir_getgstate(dir: &super::metadata::MdDir, gstate: &mut GState) -> Result<(), Error> {
    super::metadata::dir_traverse_tags(dir, 0, 0x400, 0, false, |tag, data| {
        let type3 = (tag >> 20) & 0x7ff;
        if type3 != tag::TYPE_MOVESTATE {
            return Ok(super::metadata::TraverseAction::Continue);
        }
        if data.len() >= 12 {
            let temp = GState::from_le_bytes(data)?;
            gstate.xor(&temp);
        }
        Ok(super::metadata::TraverseAction::Continue)
    })?;
    Ok(())
}
