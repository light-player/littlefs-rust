//! Forward-CRC. Per lfs.c struct lfs_fcrc.

use crate::types::lfs_size_t;
use crate::util::{lfs_fromle32, lfs_tole32};

/// Per lfs.c struct lfs_fcrc
#[repr(C)]
pub struct LfsFcrc {
    pub size: lfs_size_t,
    pub crc: u32,
}

/// Per lfs.c lfs_fcrc_fromle32
#[inline(always)]
pub fn lfs_fcrc_fromle32(fcrc: &mut LfsFcrc) {
    fcrc.size = lfs_fromle32(fcrc.size);
    fcrc.crc = lfs_fromle32(fcrc.crc);
}

/// Per lfs.c lfs_fcrc_tole32
#[inline(always)]
pub fn lfs_fcrc_tole32(fcrc: &mut LfsFcrc) {
    fcrc.size = lfs_tole32(fcrc.size);
    fcrc.crc = lfs_tole32(fcrc.crc);
}
