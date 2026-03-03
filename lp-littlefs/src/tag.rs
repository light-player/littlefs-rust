//! Tag operations. Per lfs.c LFS_MKTAG, lfs_tag_*, lfs_mattr, lfs_diskoff.

use crate::types::{lfs_block_t, lfs_off_t, lfs_size_t, lfs_tag_t};

/// Per lfs.c LFS_MKTAG
#[inline(always)]
pub fn lfs_mktag(type_: u32, id: u32, size: u32) -> lfs_tag_t {
    ((type_ as lfs_tag_t) << 20) | ((id as lfs_tag_t) << 10) | (size as lfs_tag_t)
}

/// Per lfs.c LFS_MKTAG_IF
#[inline(always)]
pub fn lfs_mktag_if(cond: bool, type_: u32, id: u32, size: u32) -> lfs_tag_t {
    if cond {
        lfs_mktag(type_, id, size)
    } else {
        lfs_mktag(crate::lfs_type::lfs_type::LFS_FROM_NOOP, 0, 0)
    }
}

/// Per lfs.c lfs_tag_isvalid
#[inline(always)]
pub fn lfs_tag_isvalid(tag: lfs_tag_t) -> bool {
    (tag & 0x8000_0000) == 0
}

/// Per lfs.c lfs_tag_isdelete
#[inline(always)]
pub fn lfs_tag_isdelete(tag: lfs_tag_t) -> bool {
    ((tag as i32) << 22) >> 22 == -1
}

/// Per lfs.c lfs_tag_type1
#[inline(always)]
pub fn lfs_tag_type1(tag: lfs_tag_t) -> u16 {
    ((tag & 0x7000_0000) >> 20) as u16
}

/// Per lfs.c lfs_tag_type2
#[inline(always)]
pub fn lfs_tag_type2(tag: lfs_tag_t) -> u16 {
    ((tag & 0x7800_0000) >> 20) as u16
}

/// Per lfs.c lfs_tag_type3
#[inline(always)]
pub fn lfs_tag_type3(tag: lfs_tag_t) -> u16 {
    ((tag & 0x7ff0_0000) >> 20) as u16
}

/// Per lfs.c lfs_tag_chunk
#[inline(always)]
pub fn lfs_tag_chunk(tag: lfs_tag_t) -> u8 {
    ((tag & 0x0ff0_0000) >> 20) as u8
}

/// Per lfs.c lfs_tag_splice
#[inline(always)]
pub fn lfs_tag_splice(tag: lfs_tag_t) -> i8 {
    lfs_tag_chunk(tag) as i8
}

/// Per lfs.c lfs_tag_id
#[inline(always)]
pub fn lfs_tag_id(tag: lfs_tag_t) -> u16 {
    ((tag & 0x000f_fc00) >> 10) as u16
}

/// Per lfs.c lfs_tag_size
#[inline(always)]
pub fn lfs_tag_size(tag: lfs_tag_t) -> lfs_size_t {
    tag & 0x0000_03ff
}

/// Per lfs.c lfs_tag_dsize - sizeof(tag) + lfs_tag_size(tag + lfs_tag_isdelete(tag))
#[inline(always)]
pub fn lfs_tag_dsize(tag: lfs_tag_t) -> lfs_size_t {
    let size = if lfs_tag_isdelete(tag) {
        lfs_tag_size(tag.wrapping_add(1))
    } else {
        lfs_tag_size(tag)
    };
    4 + size // sizeof(tag)
}

/// Per lfs.c struct lfs_mattr
#[repr(C)]
pub struct lfs_mattr {
    pub tag: lfs_tag_t,
    pub buffer: *const core::ffi::c_void,
}

/// Per lfs.c struct lfs_diskoff
#[repr(C)]
pub struct lfs_diskoff {
    pub block: lfs_block_t,
    pub off: lfs_off_t,
}
