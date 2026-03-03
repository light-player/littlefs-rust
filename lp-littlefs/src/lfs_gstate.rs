//! Global state. Per lfs.h lfs_gstate_t and lfs.c lfs_gstate_*.

use crate::tag::{lfs_tag_size, lfs_tag_type1};
use crate::types::lfs_block_t;
use crate::util::lfs_pair_cmp;
use crate::util::{lfs_fromle32, lfs_tole32};

/// Per lfs.h typedef struct lfs_gstate
#[repr(C)]
pub struct LfsGstate {
    pub tag: u32,
    pub pair: [lfs_block_t; 2],
}

/// Per lfs.c lfs_gstate_xor
#[inline(always)]
pub fn lfs_gstate_xor(a: &mut LfsGstate, b: &LfsGstate) {
    a.tag ^= b.tag;
    a.pair[0] ^= b.pair[0];
    a.pair[1] ^= b.pair[1];
}

/// Per lfs.c lfs_gstate_iszero
#[inline(always)]
pub fn lfs_gstate_iszero(a: &LfsGstate) -> bool {
    a.tag == 0 && a.pair[0] == 0 && a.pair[1] == 0
}

/// Per lfs.c lfs_gstate_hasorphans
#[inline(always)]
pub fn lfs_gstate_hasorphans(a: &LfsGstate) -> bool {
    lfs_tag_size(a.tag) != 0
}

/// Per lfs.c lfs_gstate_getorphans
#[inline(always)]
pub fn lfs_gstate_getorphans(a: &LfsGstate) -> u8 {
    (lfs_tag_size(a.tag) & 0x1ff) as u8
}

/// Per lfs.c lfs_gstate_hasmove
#[inline(always)]
pub fn lfs_gstate_hasmove(a: &LfsGstate) -> bool {
    lfs_tag_type1(a.tag) != 0
}

/// Per lfs.c lfs_gstate_needssuperblock
#[inline(always)]
pub fn lfs_gstate_needssuperblock(a: &LfsGstate) -> bool {
    (lfs_tag_size(a.tag) >> 9) != 0
}

/// Per lfs.c lfs_gstate_hasmovehere
#[inline(always)]
pub fn lfs_gstate_hasmovehere(a: &LfsGstate, pair: &[lfs_block_t; 2]) -> bool {
    lfs_tag_type1(a.tag) != 0 && lfs_pair_cmp(&a.pair, pair) == 0
}

/// Per lfs.c lfs_gstate_fromle32
#[inline(always)]
pub fn lfs_gstate_fromle32(a: &mut LfsGstate) {
    a.tag = lfs_fromle32(a.tag);
    a.pair[0] = lfs_fromle32(a.pair[0]);
    a.pair[1] = lfs_fromle32(a.pair[1]);
}

/// Per lfs.c lfs_gstate_tole32
#[inline(always)]
pub fn lfs_gstate_tole32(a: &mut LfsGstate) {
    a.tag = lfs_tole32(a.tag);
    a.pair[0] = lfs_tole32(a.pair[0]);
    a.pair[1] = lfs_tole32(a.pair[1]);
}
