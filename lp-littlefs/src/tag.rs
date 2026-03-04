//! Tag operations. Per lfs.c LFS_MKTAG, lfs_tag_*, lfs_mattr, lfs_diskoff.

use crate::types::{lfs_block_t, lfs_off_t, lfs_size_t, lfs_tag_t};

/// Per lfs.c LFS_MKTAG (lines 342-343)
///
/// C:
/// ```c
/// #define LFS_MKTAG(type, id, size) \
///     (((lfs_tag_t)(type) << 20) | ((lfs_tag_t)(id) << 10) | (lfs_tag_t)(size))
/// ```
#[inline(always)]
pub fn lfs_mktag(type_: u32, id: u32, size: u32) -> lfs_tag_t {
    ((type_ as lfs_tag_t) << 20) | ((id as lfs_tag_t) << 10) | (size as lfs_tag_t)
}

/// Per lfs.c LFS_MKTAG_IF (lines 345-346)
///
/// C:
/// ```c
/// #define LFS_MKTAG_IF(cond, type, id, size) \
///     ((cond) ? LFS_MKTAG(type, id, size) : LFS_MKTAG(LFS_FROM_NOOP, 0, 0))
/// ```
#[inline(always)]
pub fn lfs_mktag_if(cond: bool, type_: u32, id: u32, size: u32) -> lfs_tag_t {
    if cond {
        lfs_mktag(type_, id, size)
    } else {
        lfs_mktag(crate::lfs_type::lfs_type::LFS_FROM_NOOP, 0, 0)
    }
}

/// Per lfs.c lfs_tag_isvalid (lines 351-353)
///
/// C:
/// ```c
/// static inline bool lfs_tag_isvalid(lfs_tag_t tag) {
///     return !(tag & 0x80000000);
/// }
/// ```
#[inline(always)]
pub fn lfs_tag_isvalid(tag: lfs_tag_t) -> bool {
    (tag & 0x8000_0000) == 0
}

/// Per lfs.c lfs_tag_isdelete (lines 355-357)
///
/// C:
/// ```c
/// static inline bool lfs_tag_isdelete(lfs_tag_t tag) {
///     return ((int32_t)(tag << 22) >> 22) == -1;
/// }
/// ```
#[inline(always)]
pub fn lfs_tag_isdelete(tag: lfs_tag_t) -> bool {
    ((tag as i32) << 22) >> 22 == -1
}

/// Per lfs.c lfs_tag_type1 (lines 359-361)
///
/// C:
/// ```c
/// static inline uint16_t lfs_tag_type1(lfs_tag_t tag) {
///     return (tag & 0x70000000) >> 20;
/// }
/// ```
#[inline(always)]
pub fn lfs_tag_type1(tag: lfs_tag_t) -> u16 {
    ((tag & 0x7000_0000) >> 20) as u16
}

/// Per lfs.c lfs_tag_type2 (lines 363-365)
///
/// C:
/// ```c
/// static inline uint16_t lfs_tag_type2(lfs_tag_t tag) {
///     return (tag & 0x78000000) >> 20;
/// }
/// ```
#[inline(always)]
pub fn lfs_tag_type2(tag: lfs_tag_t) -> u16 {
    ((tag & 0x7800_0000) >> 20) as u16
}

/// Per lfs.c lfs_tag_type3 (lines 367-369)
///
/// C:
/// ```c
/// static inline uint16_t lfs_tag_type3(lfs_tag_t tag) {
///     return (tag & 0x7ff00000) >> 20;
/// }
/// ```
#[inline(always)]
pub fn lfs_tag_type3(tag: lfs_tag_t) -> u16 {
    ((tag & 0x7ff0_0000) >> 20) as u16
}

/// Per lfs.c lfs_tag_chunk (lines 371-373)
///
/// C:
/// ```c
/// static inline uint8_t lfs_tag_chunk(lfs_tag_t tag) {
///     return (tag & 0x0ff00000) >> 20;
/// }
/// ```
#[inline(always)]
pub fn lfs_tag_chunk(tag: lfs_tag_t) -> u8 {
    ((tag & 0x0ff0_0000) >> 20) as u8
}

/// Per lfs.c lfs_tag_splice (lines 375-377)
///
/// C:
/// ```c
/// static inline int8_t lfs_tag_splice(lfs_tag_t tag) {
///     return (int8_t)lfs_tag_chunk(tag);
/// }
/// ```
#[inline(always)]
pub fn lfs_tag_splice(tag: lfs_tag_t) -> i8 {
    lfs_tag_chunk(tag) as i8
}

/// Per lfs.c lfs_tag_id (lines 379-381)
///
/// C:
/// ```c
/// static inline uint16_t lfs_tag_id(lfs_tag_t tag) {
///     return (tag & 0x000ffc00) >> 10;
/// }
/// ```
#[inline(always)]
pub fn lfs_tag_id(tag: lfs_tag_t) -> u16 {
    ((tag & 0x000f_fc00) >> 10) as u16
}

/// Per lfs.c lfs_tag_size (lines 383-385)
///
/// C:
/// ```c
/// static inline lfs_size_t lfs_tag_size(lfs_tag_t tag) {
///     return tag & 0x000003ff;
/// }
/// ```
#[inline(always)]
pub fn lfs_tag_size(tag: lfs_tag_t) -> lfs_size_t {
    tag & 0x0000_03ff
}

/// Per lfs.c lfs_tag_dsize (lines 387-389) - sizeof(tag) + lfs_tag_size(tag + lfs_tag_isdelete(tag))
///
/// C:
/// ```c
/// static inline lfs_size_t lfs_tag_dsize(lfs_tag_t tag) {
///     return sizeof(tag) + lfs_tag_size(tag + lfs_tag_isdelete(tag));
/// }
/// ```
#[inline(always)]
pub fn lfs_tag_dsize(tag: lfs_tag_t) -> lfs_size_t {
    let size = if lfs_tag_isdelete(tag) {
        lfs_tag_size(tag.wrapping_add(1))
    } else {
        lfs_tag_size(tag)
    };
    4 + size // sizeof(tag)
}

/// Per lfs.c struct lfs_mattr (lines 392-395)
///
/// C:
/// ```c
/// struct lfs_mattr {
///     lfs_tag_t tag;
///     const void *buffer;
/// };
/// ```
#[repr(C)]
pub struct lfs_mattr {
    pub tag: lfs_tag_t,
    pub buffer: *const core::ffi::c_void,
}

/// Per lfs.c struct lfs_diskoff (lines 397-400)
///
/// C:
/// ```c
/// struct lfs_diskoff {
///     lfs_block_t block;
///     lfs_off_t off;
/// };
/// ```
#[repr(C)]
pub struct lfs_diskoff {
    pub block: lfs_block_t,
    pub off: lfs_off_t,
}
