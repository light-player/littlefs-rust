//! Unit tests using TestContext.

use super::*;

/// Minimal: construct TestContext and verify config/ram. No lfs calls.
#[test]
fn test_context_smoke() {
    let ctx = TestContext::default_blocks();
    assert!(!ctx.config().is_null());
    let cfg = unsafe { &*ctx.config() };
    assert!(!cfg.context.is_null(), "config.context should be set");
    assert!(cfg.read.is_some());
    assert_eq!(ctx.ram.data.len(), 512 * 128);
    // Direct read through callback
    let mut buf = [0u8; 8];
    let err = unsafe { cfg.read.expect("read")(ctx.config(), 0, 0, buf.as_mut_ptr(), 8) };
    assert_eq!(err, 0);
    assert_eq!(buf, [0u8; 8]);
}

/// Call lfs_init only. Isolates init from full format.
#[test]
fn test_context_lfs_init() {
    let mut ctx = TestContext::default_blocks();
    let mut lfs = core::mem::MaybeUninit::<crate::Lfs>::zeroed();
    let err = crate::fs::lfs_init(lfs.as_mut_ptr() as *mut _, ctx.config());
    assert_eq!(err, 0);
}

/// Init + lookahead setup + lfs_dir_alloc. Stops before commit.
#[test]
fn test_context_format_to_alloc() {
    use crate::block_alloc::alloc::lfs_alloc_ckpoint;
    use crate::dir::commit::lfs_dir_alloc;
    use crate::util::lfs_min;

    let mut ctx = TestContext::default_blocks();
    let mut lfs = core::mem::MaybeUninit::<crate::Lfs>::zeroed();
    let err = crate::fs::lfs_init(lfs.as_mut_ptr() as *mut _, ctx.config());
    assert_eq!(err, 0);

    let lfs = unsafe { &mut *lfs.as_mut_ptr() };
    let cfg = unsafe { &*lfs.cfg };
    if !lfs.lookahead.buffer.is_null() {
        unsafe {
            core::ptr::write_bytes(lfs.lookahead.buffer, 0, cfg.lookahead_size as usize);
        }
    }
    lfs.lookahead.start = 0;
    lfs.lookahead.size = lfs_min(8 * cfg.lookahead_size, lfs.block_count);
    lfs.lookahead.next = 0;
    unsafe { lfs_alloc_ckpoint(lfs as *mut _) };

    let mut root = crate::dir::LfsMdir {
        pair: [0, 0],
        rev: 0,
        off: 0,
        etag: 0,
        count: 0,
        erased: false,
        split: false,
        tail: [0, 0],
    };
    let err = unsafe { lfs_dir_alloc(lfs as *mut _, &mut root) };
    assert_eq!(err, 0);
}

/// Verify buffer pointers are writable (lfs_init writes to them).
#[test]
fn test_context_buffers_writable() {
    let ctx = TestContext::default_blocks();
    let cfg = unsafe { &*ctx.config() };
    // Manually write to each buffer - simulate what lfs_cache_zero and format do
    let block_size = ctx.ram.block_size as usize;
    if !cfg.read_buffer.is_null() {
        unsafe { core::ptr::write_bytes(cfg.read_buffer as *mut u8, 0xff, block_size) };
    }
    if !cfg.prog_buffer.is_null() {
        unsafe { core::ptr::write_bytes(cfg.prog_buffer as *mut u8, 0xff, block_size) };
    }
    if !cfg.lookahead_buffer.is_null() {
        unsafe { core::ptr::write_bytes(cfg.lookahead_buffer as *mut u8, 0, block_size) };
    }
}
