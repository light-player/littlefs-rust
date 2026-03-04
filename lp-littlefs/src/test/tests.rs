//! Unit tests using TestContext.

use super::*;
use crate::dir::traverse::TraverseTestOut;
use crate::fs::format::{test_format_minimal_superblock, test_traverse_format_attrs};

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
    lfs_alloc_ckpoint(lfs);

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
    let err = lfs_dir_alloc(lfs as *mut _, &mut root);
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

/// Full format. Crashes (SIGSEGV) when run as unit test; integration tests cover this.
#[test]
#[ignore = "SIGSEGV in lfs_dir_commit when TestContext runs as lib unit test; use integration tests"]
fn test_context_format() {
    let mut ctx = TestContext::default_blocks();
    let mut lfs = core::mem::MaybeUninit::<crate::Lfs>::zeroed();
    let err = crate::lfs_format(lfs.as_mut_ptr() as *mut crate::Lfs, ctx.config());
    assert_eq!(err, 0);
    assert_blocks_0_and_1_have_magic(ctx.config());
}

#[test]
#[ignore = "same as test_context_format"]
fn test_context_bypass() {
    let mut ctx = TestContext::default_blocks();
    let mut lfs = core::mem::MaybeUninit::<crate::Lfs>::zeroed();
    let err = test_format_minimal_superblock(lfs.as_mut_ptr() as *mut _, ctx.config());
    assert_eq!(err, 0);
    let mut has_magic = false;
    let mut buf = [0u8; 24];
    unsafe {
        let read = (*ctx.config()).read.expect("read");
        if read(ctx.config(), 0, 0, buf.as_mut_ptr(), 24) == 0
            && buf[MAGIC_OFFSET as usize..][..8] == *MAGIC
        {
            has_magic = true;
        }
        if read(ctx.config(), 1, 0, buf.as_mut_ptr(), 24) == 0
            && buf[MAGIC_OFFSET as usize..][..8] == *MAGIC
        {
            has_magic = true;
        }
    }
    assert!(has_magic);
}

#[test]
#[ignore = "same as test_context_format"]
fn test_context_traverse() {
    let mut ctx = TestContext::default_blocks();
    let mut lfs = core::mem::MaybeUninit::<crate::Lfs>::zeroed();
    let mut out = TraverseTestOut::default();
    let err =
        test_traverse_format_attrs(lfs.as_mut_ptr() as *mut _, ctx.config(), &mut out as *mut _);
    assert_eq!(err, 0);
    assert_eq!(out.call_count, 3);
    assert_eq!(out.tags[1], 0x0ff);
    assert_eq!(out.first_bytes[1], b'l');
}
