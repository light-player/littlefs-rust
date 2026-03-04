//! Utility functions. Per lfs_util.h static inline and lfs.c small type-level utils.

use crate::types::{lfs_block_t, lfs_size_t};

/// Per lfs_util.h lfs_max
#[inline(always)]
pub fn lfs_max(a: u32, b: u32) -> u32 {
    if a > b {
        a
    } else {
        b
    }
}

/// Per lfs_util.h lfs_min
#[inline(always)]
pub fn lfs_min(a: u32, b: u32) -> u32 {
    if a < b {
        a
    } else {
        b
    }
}

/// Per lfs_util.h lfs_aligndown
#[inline(always)]
pub fn lfs_aligndown(a: u32, alignment: u32) -> u32 {
    a - (a % alignment)
}

/// Per lfs_util.h lfs_alignup
#[inline(always)]
pub fn lfs_alignup(a: u32, alignment: u32) -> u32 {
    lfs_aligndown(a + alignment - 1, alignment)
}

/// Per lfs_util.h lfs_npw2 - smallest power of 2 >= a
#[inline(always)]
pub fn lfs_npw2(a: u32) -> u32 {
    let a = a.wrapping_sub(1);
    let s4 = if a > 0xffff { 1 } else { 0 };
    let a = a >> (s4 << 4);
    let s3 = if a > 0xff { 1 } else { 0 };
    let a = a >> (s3 << 3);
    let s2 = if a > 0xf { 1 } else { 0 };
    let a = a >> (s2 << 2);
    let s1 = if a > 0x3 { 1 } else { 0 };
    let a = a >> (s1 << 1);
    (s4 << 4 | s3 << 3 | s2 << 2 | s1 << 1 | (a >> 1)) + 1
}

/// Per lfs_util.h lfs_ctz - trailing zeros
#[inline(always)]
pub fn lfs_ctz(a: u32) -> u32 {
    lfs_npw2((a & a.wrapping_neg()).wrapping_add(1)) - 1
}

/// Per lfs_util.h lfs_popc - population count
#[inline(always)]
pub fn lfs_popc(a: u32) -> u32 {
    let a = a - ((a >> 1) & 0x5555_5555);
    let a = (a & 0x3333_3333) + ((a >> 2) & 0x3333_3333);
    (((a.wrapping_add(a >> 4)) & 0x0f0f_0f0f).wrapping_mul(0x0101_0101)) >> 24
}

/// Per lfs_util.h lfs_scmp - sequence comparison
#[inline(always)]
pub fn lfs_scmp(a: u32, b: u32) -> i32 {
    (a.wrapping_sub(b)) as i32
}

/// Per lfs_util.h lfs_fromle32 - little-endian to native
#[inline(always)]
pub fn lfs_fromle32(a: u32) -> u32 {
    u32::from_le(a)
}

/// Per lfs_util.h lfs_tole32
#[inline(always)]
pub fn lfs_tole32(a: u32) -> u32 {
    a.to_le()
}

/// Per lfs_util.h lfs_frombe32 - big-endian to native
#[inline(always)]
pub fn lfs_frombe32(a: u32) -> u32 {
    u32::from_be(a)
}

/// Per lfs_util.h lfs_tobe32
#[inline(always)]
pub fn lfs_tobe32(a: u32) -> u32 {
    a.to_be()
}

// --- lfs.c path operations ---

/// Per lfs.c lfs_path_namelen (lines 289-291)
///
/// C:
/// ```c
/// static inline lfs_size_t lfs_path_namelen(const char *path) {
///     return strcspn(path, "/");
/// }
/// ```
#[inline(always)]
pub fn lfs_path_namelen(path: &[u8]) -> u32 {
    path.iter().position(|&b| b == b'/').unwrap_or(path.len()) as lfs_size_t
}

/// Per lfs.c lfs_path_islast (lines 293-296)
///
/// C:
/// ```c
/// static inline bool lfs_path_islast(const char *path) {
///     lfs_size_t namelen = lfs_path_namelen(path);
///     return path[namelen + strspn(path + namelen, "/")] == '\0';
/// }
/// ```
#[inline(always)]
pub fn lfs_path_islast(path: &[u8]) -> bool {
    let namelen = lfs_path_namelen(path) as usize;
    let rest = path.get(namelen..).unwrap_or(&[]);
    let skip = rest.iter().take_while(|&&b| b == b'/').count();
    path.get(namelen + skip).map_or(true, |&b| b == 0)
}

/// Per lfs.c lfs_path_isdir (lines 298-300)
///
/// C:
/// ```c
/// static inline bool lfs_path_isdir(const char *path) {
///     return path[lfs_path_namelen(path)] != '\0';
/// }
/// ```
#[inline(always)]
pub fn lfs_path_isdir(path: &[u8]) -> bool {
    let namelen = lfs_path_namelen(path) as usize;
    path.get(namelen).map_or(false, |&b| b != 0)
}

/// Per lfs.c lfs_pair_swap
#[inline(always)]
pub fn lfs_pair_swap(pair: &mut [lfs_block_t; 2]) {
    pair.swap(0, 1);
}

/// Per lfs.c lfs_pair_isnull
#[inline(always)]
pub fn lfs_pair_isnull(pair: &[lfs_block_t; 2]) -> bool {
    use crate::types::LFS_BLOCK_NULL;
    pair[0] == LFS_BLOCK_NULL || pair[1] == LFS_BLOCK_NULL
}

/// Per lfs.c lfs_pair_cmp - returns 0 if equal
#[inline(always)]
pub fn lfs_pair_cmp(paira: &[lfs_block_t; 2], pairb: &[lfs_block_t; 2]) -> i32 {
    let eq = paira[0] == pairb[0]
        || paira[1] == pairb[1]
        || paira[0] == pairb[1]
        || paira[1] == pairb[0];
    if eq {
        0
    } else {
        1
    }
}

/// Per lfs.c lfs_pair_issync
#[inline(always)]
pub fn lfs_pair_issync(paira: &[lfs_block_t; 2], pairb: &[lfs_block_t; 2]) -> bool {
    (paira[0] == pairb[0] && paira[1] == pairb[1]) || (paira[0] == pairb[1] && paira[1] == pairb[0])
}
