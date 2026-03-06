//! CTZ struct (file block list). Per lfs.h lfs_file_t.ctz.

use crate::types::{lfs_block_t, lfs_size_t};
use crate::util::{lfs_fromle32, lfs_tole32};

/// Per lfs.h struct lfs_ctz (in lfs_file_t)
#[repr(C)]
#[derive(Clone, Copy)]
pub struct LfsCtz {
    pub head: lfs_block_t,
    pub size: lfs_size_t,
}

/// Per lfs.c lfs_ctz_fromle32
#[inline(always)]
pub fn lfs_ctz_fromle32(ctz: &mut LfsCtz) {
    ctz.head = lfs_fromle32(ctz.head);
    ctz.size = lfs_fromle32(ctz.size);
}

/// Per lfs.c lfs_ctz_tole32
#[inline(always)]
pub fn lfs_ctz_tole32(ctz: &mut LfsCtz) {
    ctz.head = lfs_tole32(ctz.head);
    ctz.size = lfs_tole32(ctz.size);
}
