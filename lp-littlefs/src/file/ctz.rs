//! CTZ operations. Per lfs.c lfs_ctz_index, lfs_ctz_find, lfs_ctz_extend, lfs_ctz_traverse.

/// Per lfs.c lfs_ctz_index (lines 2873-2885)
///
/// C:
/// ```c
/// static int lfs_ctz_index(lfs_t *lfs, lfs_off_t *off) {
///     lfs_off_t size = *off;
///     lfs_off_t b = lfs->cfg->block_size - 2*4;
///     lfs_off_t i = size / b;
///     if (i == 0) return 0;
///     i = (size - 4*(lfs_popc(i-1)+2)) / b;
///     *off = size - b*i - 4*lfs_popc(i);
///     return i;
/// }
/// ```
pub fn lfs_ctz_index(_lfs: *const core::ffi::c_void) -> i32 {
    todo!("lfs_ctz_index")
}
