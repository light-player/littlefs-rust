//! Upstream: tests/test_shrink.toml
//!
//! Shrink/grow block count tests via lfs_fs_grow (shrink path).

mod common;

use common::{
    assert_err, assert_ok, clone_config_with_block_count, default_config, init_context,
    LFS_O_CREAT, LFS_O_EXCL, LFS_O_RDONLY, LFS_O_WRONLY,
};
use lp_littlefs::{
    lfs_file_close, lfs_file_open, lfs_file_read, lfs_file_write, lfs_format, lfs_fs_grow,
    lfs_mount, lfs_unmount, Lfs, LfsConfig, LfsFile, LFS_ERR_INVAL, LFS_ERR_NOTEMPTY,
};

const BLOCK_SIZE: u32 = 512;

/// Upstream: [cases.test_shrink_simple]
///
/// defines.BLOCK_COUNT = [10, 15, 20]
/// defines.AFTER_BLOCK_COUNT = [5, 10, 15, 19]
/// if = 'AFTER_BLOCK_COUNT <= BLOCK_COUNT'
///
/// Format on BLOCK_COUNT blocks, shrink via lfs_fs_grow(AFTER_BLOCK_COUNT).
/// If sizes differ, mount with original config fails (LFS_ERR_INVAL),
/// mount with reduced config succeeds.
#[test]
fn test_shrink_simple() {
    for &block_count in &[10u32, 15, 20] {
        for &after in &[5u32, 10, 15, 19] {
            if after <= block_count {
                unsafe { shrink_simple(block_count, after) };
            }
        }
    }
}

unsafe fn shrink_simple(block_count: u32, after_block_count: u32) {
    let mut env = default_config(block_count);
    init_context(&mut env);
    let cfg = &env.config as *const LfsConfig;

    let mut lfs = core::mem::MaybeUninit::<Lfs>::zeroed();
    assert_ok(lfs_format(lfs.as_mut_ptr(), cfg));
    assert_ok(lfs_mount(lfs.as_mut_ptr(), cfg));
    assert_ok(lfs_fs_grow(lfs.as_mut_ptr(), after_block_count));
    let _ = lfs_unmount(lfs.as_mut_ptr());

    if block_count != after_block_count {
        assert_err(LFS_ERR_INVAL, lfs_mount(lfs.as_mut_ptr(), cfg));
    }

    // Mount with reduced config
    let cfg2 = clone_config_with_block_count(&env, after_block_count);
    let mut lfs2 = core::mem::MaybeUninit::<Lfs>::zeroed();
    assert_ok(lfs_mount(
        lfs2.as_mut_ptr(),
        &cfg2.config as *const LfsConfig,
    ));
    assert_ok(lfs_unmount(lfs2.as_mut_ptr()));
}

/// Upstream: [cases.test_shrink_full]
///
/// defines.BLOCK_COUNT = [10, 15, 20]
/// defines.AFTER_BLOCK_COUNT = [5, 7, 10, 12, 15, 17, 20]
/// defines.FILES_COUNT = [7, 8, 9, 10]
/// if = 'AFTER_BLOCK_COUNT <= BLOCK_COUNT && FILES_COUNT + 2 < BLOCK_COUNT'
///
/// Create FILES_COUNT+1 files of BLOCK_SIZE-0x40 bytes. Shrink via
/// lfs_fs_grow(AFTER_BLOCK_COUNT). On success: verify all files and
/// remount with reduced config. On LFS_ERR_NOTEMPTY: shrink expected
/// to fail (too many files for smaller device).
#[test]
fn test_shrink_full() {
    for &block_count in &[10u32, 15, 20] {
        for &after in &[5u32, 7, 10, 12, 15, 17, 20] {
            for &files_count in &[7u32, 8, 9, 10] {
                if after <= block_count && files_count + 2 < block_count {
                    unsafe { shrink_full(block_count, after, files_count) };
                }
            }
        }
    }
}

unsafe fn shrink_full(block_count: u32, after_block_count: u32, files_count: u32) {
    let mut env = default_config(block_count);
    init_context(&mut env);
    let cfg = &env.config as *const LfsConfig;
    let size = BLOCK_SIZE - 0x40;

    let mut lfs = core::mem::MaybeUninit::<Lfs>::zeroed();
    assert_ok(lfs_format(lfs.as_mut_ptr(), cfg));
    assert_ok(lfs_mount(lfs.as_mut_ptr(), cfg));

    // Create FILES_COUNT+1 files of BLOCK_SIZE - 0x40 bytes
    for i in 0..files_count + 1 {
        let path = format!("file_{:03}\0", i);
        let mut file = core::mem::MaybeUninit::<LfsFile>::zeroed();
        assert_ok(lfs_file_open(
            lfs.as_mut_ptr(),
            file.as_mut_ptr(),
            path.as_ptr(),
            LFS_O_WRONLY | LFS_O_CREAT | LFS_O_EXCL,
        ));

        let mut wbuffer = vec![b'b'; BLOCK_SIZE as usize];
        let header = format!("Hi {:03}", i);
        wbuffer[..header.len()].copy_from_slice(header.as_bytes());

        let n = lfs_file_write(
            lfs.as_mut_ptr(),
            file.as_mut_ptr(),
            wbuffer.as_ptr() as *const core::ffi::c_void,
            size,
        );
        assert_eq!(n, size as i32);
        assert_ok(lfs_file_close(lfs.as_mut_ptr(), file.as_mut_ptr()));
    }

    let err = lfs_fs_grow(lfs.as_mut_ptr(), after_block_count);
    if err == 0 {
        // Verify all files while still mounted
        for i in 0..files_count + 1 {
            let path = format!("file_{:03}\0", i);
            let mut file = core::mem::MaybeUninit::<LfsFile>::zeroed();
            assert_ok(lfs_file_open(
                lfs.as_mut_ptr(),
                file.as_mut_ptr(),
                path.as_ptr(),
                LFS_O_RDONLY,
            ));

            let mut rbuffer = vec![0u8; size as usize];
            let n = lfs_file_read(
                lfs.as_mut_ptr(),
                file.as_mut_ptr(),
                rbuffer.as_mut_ptr() as *mut core::ffi::c_void,
                BLOCK_SIZE,
            );
            assert_eq!(n, size as i32);
            assert_ok(lfs_file_close(lfs.as_mut_ptr(), file.as_mut_ptr()));

            // Build reference buffer
            let mut wbuffer_ref = vec![b'b'; size as usize];
            let header = format!("Hi {:03}", i);
            wbuffer_ref[..header.len()].copy_from_slice(header.as_bytes());
            assert_eq!(rbuffer, wbuffer_ref);
        }
    } else {
        assert_eq!(err, LFS_ERR_NOTEMPTY);
    }

    assert_ok(lfs_unmount(lfs.as_mut_ptr()));

    if err == 0 {
        if after_block_count != block_count {
            assert_err(LFS_ERR_INVAL, lfs_mount(lfs.as_mut_ptr(), cfg));
        }

        // Remount with reduced config and verify files again
        let cfg2 = clone_config_with_block_count(&env, after_block_count);
        let mut lfs2 = core::mem::MaybeUninit::<Lfs>::zeroed();
        assert_ok(lfs_mount(
            lfs2.as_mut_ptr(),
            &cfg2.config as *const LfsConfig,
        ));

        for i in 0..files_count + 1 {
            let path = format!("file_{:03}\0", i);
            let mut file = core::mem::MaybeUninit::<LfsFile>::zeroed();
            assert_ok(lfs_file_open(
                lfs2.as_mut_ptr(),
                file.as_mut_ptr(),
                path.as_ptr(),
                LFS_O_RDONLY,
            ));

            let mut rbuffer = vec![0u8; size as usize];
            let n = lfs_file_read(
                lfs2.as_mut_ptr(),
                file.as_mut_ptr(),
                rbuffer.as_mut_ptr() as *mut core::ffi::c_void,
                BLOCK_SIZE,
            );
            assert_eq!(n, size as i32);
            assert_ok(lfs_file_close(lfs2.as_mut_ptr(), file.as_mut_ptr()));

            let mut wbuffer_ref = vec![b'b'; size as usize];
            let header = format!("Hi {:03}", i);
            wbuffer_ref[..header.len()].copy_from_slice(header.as_bytes());
            assert_eq!(rbuffer, wbuffer_ref);
        }

        let _ = lfs_unmount(lfs2.as_mut_ptr());
    }
}
