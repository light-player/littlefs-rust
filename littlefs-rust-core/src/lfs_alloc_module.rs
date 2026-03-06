//! Allocator helpers. Per lfs_util.h lfs_malloc, lfs_free.
//!
//! Used when file config does not provide a buffer. Gated on alloc feature.

#[cfg(feature = "alloc")]
/// Translation docs: Allocate memory for littlefs buffers. Returns null on failure.
///
/// C: lfs_util.h:243-252
pub fn lfs_malloc(size: u32) -> *mut u8 {
    use alloc::alloc::{alloc, Layout};
    let layout = Layout::from_size_align(size as usize, 4).expect("invalid layout");

    unsafe { alloc(layout) }
}

#[cfg(feature = "alloc")]
/// Translation docs: Deallocate memory. C takes only (ptr); we need size for alloc::dealloc.
///
/// C: lfs_util.h:255-264
/// Divergence: C lfs_free(p) has no size; we pass size for Layout in dealloc.
pub fn lfs_free(ptr: *mut u8, size: u32) {
    if ptr.is_null() {
        return;
    }
    use alloc::alloc::{dealloc, Layout};
    let layout = Layout::from_size_align(size as usize, 4).expect("invalid layout");
    unsafe {
        dealloc(ptr, layout);
    }
}
