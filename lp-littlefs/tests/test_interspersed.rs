//! Upstream: tests/test_interspersed.toml
//!
//! Multi-file I/O interspersed with directory operations.

#![allow(clippy::needless_range_loop)]

mod common;

use common::{
    assert_ok, default_config, init_context, path_bytes, LFS_O_APPEND, LFS_O_CREAT, LFS_O_EXCL,
    LFS_O_RDONLY, LFS_O_WRONLY,
};
use lp_littlefs::{
    lfs_dir_close, lfs_dir_open, lfs_dir_read, lfs_file_close, lfs_file_open, lfs_file_read,
    lfs_file_size, lfs_file_sync, lfs_file_write, lfs_format, lfs_mount, lfs_remove, lfs_unmount,
    Lfs, LfsConfig, LfsDir, LfsFile, LfsInfo,
};
use rstest::rstest;

const ALPHAS: &[u8] = b"abcdefghijklmnopqrstuvwxyz";
const LFS_TYPE_DIR: u8 = 0x02;
const LFS_TYPE_REG: u8 = 0x01;

/// Upstream: [cases.test_interspersed_files]
/// defines.SIZE = [10, 100]
/// defines.FILES = [4, 10, 26]
///
/// Open FILES files ("a","b",...), write SIZE bytes to each in round-robin
/// (1 byte per iteration), close all. Verify directory listing (FILES + 2
/// for . and ..). Check each file has SIZE bytes, read back first 10 bytes.
#[rstest]
fn test_interspersed_files(#[values(10, 100)] size: usize, #[values(4, 10, 26)] files: usize) {
    let mut env = default_config(128);
    init_context(&mut env);

    let mut lfs = core::mem::MaybeUninit::<Lfs>::zeroed();
    assert_ok(lfs_format(
        lfs.as_mut_ptr(),
        &env.config as *const LfsConfig,
    ));
    assert_ok(lfs_mount(lfs.as_mut_ptr(), &env.config as *const LfsConfig));

    let mut file_handles: Vec<core::mem::MaybeUninit<LfsFile>> = (0..files)
        .map(|_| core::mem::MaybeUninit::zeroed())
        .collect();

    for j in 0..files {
        let path = path_bytes(&String::from(ALPHAS[j] as char));
        assert_ok(lfs_file_open(
            lfs.as_mut_ptr(),
            file_handles[j].as_mut_ptr(),
            path.as_ptr(),
            LFS_O_WRONLY | LFS_O_CREAT | LFS_O_EXCL,
        ));
    }

    for _i in 0..size {
        for j in 0..files {
            let byte = [ALPHAS[j]];
            let n = lfs_file_write(
                lfs.as_mut_ptr(),
                file_handles[j].as_mut_ptr(),
                byte.as_ptr() as *const core::ffi::c_void,
                1,
            );
            assert_eq!(n, 1);
        }
    }

    for j in 0..files {
        assert_ok(lfs_file_close(
            lfs.as_mut_ptr(),
            file_handles[j].as_mut_ptr(),
        ));
    }

    // Verify directory listing
    let root = path_bytes("/");
    let mut dir = core::mem::MaybeUninit::<LfsDir>::zeroed();
    assert_ok(lfs_dir_open(
        lfs.as_mut_ptr(),
        dir.as_mut_ptr(),
        root.as_ptr(),
    ));

    let mut info = core::mem::MaybeUninit::<LfsInfo>::zeroed();

    assert_eq!(
        lfs_dir_read(lfs.as_mut_ptr(), dir.as_mut_ptr(), info.as_mut_ptr()),
        1
    );
    let info_ref = unsafe { &*info.as_ptr() };
    assert_eq!(&info_ref.name[..1], b".");
    assert_eq!(info_ref.type_, LFS_TYPE_DIR);

    assert_eq!(
        lfs_dir_read(lfs.as_mut_ptr(), dir.as_mut_ptr(), info.as_mut_ptr()),
        1
    );
    let info_ref = unsafe { &*info.as_ptr() };
    assert_eq!(&info_ref.name[..2], b"..");
    assert_eq!(info_ref.type_, LFS_TYPE_DIR);

    for j in 0..files {
        let expected_name = String::from(ALPHAS[j] as char);
        assert_eq!(
            lfs_dir_read(lfs.as_mut_ptr(), dir.as_mut_ptr(), info.as_mut_ptr()),
            1
        );
        let info_ref = unsafe { &*info.as_ptr() };
        let nul = info_ref.name.iter().position(|&b| b == 0).unwrap_or(256);
        let name = core::str::from_utf8(&info_ref.name[..nul]).unwrap();
        assert_eq!(name, expected_name);
        assert_eq!(info_ref.type_, LFS_TYPE_REG);
        assert_eq!(info_ref.size, size as u32);
    }

    assert_eq!(
        lfs_dir_read(lfs.as_mut_ptr(), dir.as_mut_ptr(), info.as_mut_ptr()),
        0
    );
    assert_ok(lfs_dir_close(lfs.as_mut_ptr(), dir.as_mut_ptr()));

    // Re-open for reading and verify first 10 bytes
    let mut file_handles: Vec<core::mem::MaybeUninit<LfsFile>> = (0..files)
        .map(|_| core::mem::MaybeUninit::zeroed())
        .collect();

    for j in 0..files {
        let path = path_bytes(&String::from(ALPHAS[j] as char));
        assert_ok(lfs_file_open(
            lfs.as_mut_ptr(),
            file_handles[j].as_mut_ptr(),
            path.as_ptr(),
            LFS_O_RDONLY,
        ));
    }

    for _i in 0..10 {
        for j in 0..files {
            let mut buffer = [0u8; 1];
            let n = lfs_file_read(
                lfs.as_mut_ptr(),
                file_handles[j].as_mut_ptr(),
                buffer.as_mut_ptr() as *mut core::ffi::c_void,
                1,
            );
            assert_eq!(n, 1);
            assert_eq!(buffer[0], ALPHAS[j]);
        }
    }

    for j in 0..files {
        assert_ok(lfs_file_close(
            lfs.as_mut_ptr(),
            file_handles[j].as_mut_ptr(),
        ));
    }

    assert_ok(lfs_unmount(lfs.as_mut_ptr()));
}

/// Upstream: [cases.test_interspersed_remove_files]
/// defines.SIZE = [10, 100]
/// defines.FILES = [4, 10, 26]
///
/// Create FILES files with SIZE bytes each. Open "zzz", write one byte
/// and sync, remove one of the FILES-lettered files, repeat. After removing
/// all, verify "zzz" has FILES bytes and directory listing is correct.
#[rstest]
fn test_interspersed_remove_files(
    #[values(10, 100)] size: usize,
    #[values(4, 10, 26)] files: usize,
) {
    let mut env = default_config(128);
    init_context(&mut env);

    let mut lfs = core::mem::MaybeUninit::<Lfs>::zeroed();
    assert_ok(lfs_format(
        lfs.as_mut_ptr(),
        &env.config as *const LfsConfig,
    ));
    assert_ok(lfs_mount(lfs.as_mut_ptr(), &env.config as *const LfsConfig));

    // Create FILES files with SIZE bytes each
    for j in 0..files {
        let path = path_bytes(&String::from(ALPHAS[j] as char));
        let mut file = core::mem::MaybeUninit::<LfsFile>::zeroed();
        assert_ok(lfs_file_open(
            lfs.as_mut_ptr(),
            file.as_mut_ptr(),
            path.as_ptr(),
            LFS_O_WRONLY | LFS_O_CREAT | LFS_O_EXCL,
        ));
        for _i in 0..size {
            let byte = [ALPHAS[j]];
            let n = lfs_file_write(
                lfs.as_mut_ptr(),
                file.as_mut_ptr(),
                byte.as_ptr() as *const core::ffi::c_void,
                1,
            );
            assert_eq!(n, 1);
        }
        assert_ok(lfs_file_close(lfs.as_mut_ptr(), file.as_mut_ptr()));
    }
    assert_ok(lfs_unmount(lfs.as_mut_ptr()));

    // Remount, open "zzz", interleave writes+syncs with removes
    assert_ok(lfs_mount(lfs.as_mut_ptr(), &env.config as *const LfsConfig));
    let zzz_path = path_bytes("zzz");
    let mut file = core::mem::MaybeUninit::<LfsFile>::zeroed();
    assert_ok(lfs_file_open(
        lfs.as_mut_ptr(),
        file.as_mut_ptr(),
        zzz_path.as_ptr(),
        LFS_O_WRONLY | LFS_O_CREAT,
    ));

    for j in 0..files {
        let tilde = b"~";
        let n = lfs_file_write(
            lfs.as_mut_ptr(),
            file.as_mut_ptr(),
            tilde.as_ptr() as *const core::ffi::c_void,
            1,
        );
        assert_eq!(n, 1);
        assert_ok(lfs_file_sync(lfs.as_mut_ptr(), file.as_mut_ptr()));

        let path = path_bytes(&String::from(ALPHAS[j] as char));
        assert_ok(lfs_remove(lfs.as_mut_ptr(), path.as_ptr()));
    }
    assert_ok(lfs_file_close(lfs.as_mut_ptr(), file.as_mut_ptr()));

    // Verify directory: only "zzz" left
    let root = path_bytes("/");
    let mut dir = core::mem::MaybeUninit::<LfsDir>::zeroed();
    assert_ok(lfs_dir_open(
        lfs.as_mut_ptr(),
        dir.as_mut_ptr(),
        root.as_ptr(),
    ));

    let mut info = core::mem::MaybeUninit::<LfsInfo>::zeroed();

    assert_eq!(
        lfs_dir_read(lfs.as_mut_ptr(), dir.as_mut_ptr(), info.as_mut_ptr()),
        1
    );
    let info_ref = unsafe { &*info.as_ptr() };
    assert_eq!(&info_ref.name[..1], b".");
    assert_eq!(info_ref.type_, LFS_TYPE_DIR);

    assert_eq!(
        lfs_dir_read(lfs.as_mut_ptr(), dir.as_mut_ptr(), info.as_mut_ptr()),
        1
    );
    let info_ref = unsafe { &*info.as_ptr() };
    assert_eq!(&info_ref.name[..2], b"..");
    assert_eq!(info_ref.type_, LFS_TYPE_DIR);

    assert_eq!(
        lfs_dir_read(lfs.as_mut_ptr(), dir.as_mut_ptr(), info.as_mut_ptr()),
        1
    );
    let info_ref = unsafe { &*info.as_ptr() };
    let nul = info_ref.name.iter().position(|&b| b == 0).unwrap_or(256);
    let name = core::str::from_utf8(&info_ref.name[..nul]).unwrap();
    assert_eq!(name, "zzz");
    assert_eq!(info_ref.type_, LFS_TYPE_REG);
    assert_eq!(info_ref.size, files as u32);

    assert_eq!(
        lfs_dir_read(lfs.as_mut_ptr(), dir.as_mut_ptr(), info.as_mut_ptr()),
        0
    );
    assert_ok(lfs_dir_close(lfs.as_mut_ptr(), dir.as_mut_ptr()));

    // Verify "zzz" content
    let mut file = core::mem::MaybeUninit::<LfsFile>::zeroed();
    assert_ok(lfs_file_open(
        lfs.as_mut_ptr(),
        file.as_mut_ptr(),
        zzz_path.as_ptr(),
        LFS_O_RDONLY,
    ));
    for _i in 0..files {
        let mut buffer = [0u8; 1];
        let n = lfs_file_read(
            lfs.as_mut_ptr(),
            file.as_mut_ptr(),
            buffer.as_mut_ptr() as *mut core::ffi::c_void,
            1,
        );
        assert_eq!(n, 1);
        assert_eq!(buffer[0], b'~');
    }
    assert_ok(lfs_file_close(lfs.as_mut_ptr(), file.as_mut_ptr()));

    assert_ok(lfs_unmount(lfs.as_mut_ptr()));
}

/// Upstream: [cases.test_interspersed_remove_inconveniently]
/// defines.SIZE = [10, 100]
///
/// Open three files "e","f","g". Write SIZE/2 bytes to each. Remove "f"
/// while all three are still open. Write another SIZE/2 bytes to all three
/// (including removed "f"). Close all. Verify directory: "e" and "g"
/// present, "f" absent. Read "e" and "g", verify SIZE bytes.
#[rstest]
fn test_interspersed_remove_inconveniently(#[values(10, 100)] size: usize) {
    let mut env = default_config(128);
    init_context(&mut env);

    let mut lfs = core::mem::MaybeUninit::<Lfs>::zeroed();
    assert_ok(lfs_format(
        lfs.as_mut_ptr(),
        &env.config as *const LfsConfig,
    ));
    assert_ok(lfs_mount(lfs.as_mut_ptr(), &env.config as *const LfsConfig));

    let mut files: [core::mem::MaybeUninit<LfsFile>; 3] = [
        core::mem::MaybeUninit::zeroed(),
        core::mem::MaybeUninit::zeroed(),
        core::mem::MaybeUninit::zeroed(),
    ];

    let path_e = path_bytes("e");
    let path_f = path_bytes("f");
    let path_g = path_bytes("g");

    assert_ok(lfs_file_open(
        lfs.as_mut_ptr(),
        files[0].as_mut_ptr(),
        path_e.as_ptr(),
        LFS_O_WRONLY | LFS_O_CREAT,
    ));
    assert_ok(lfs_file_open(
        lfs.as_mut_ptr(),
        files[1].as_mut_ptr(),
        path_f.as_ptr(),
        LFS_O_WRONLY | LFS_O_CREAT,
    ));
    assert_ok(lfs_file_open(
        lfs.as_mut_ptr(),
        files[2].as_mut_ptr(),
        path_g.as_ptr(),
        LFS_O_WRONLY | LFS_O_CREAT,
    ));

    // Write SIZE/2 bytes to each
    for _i in 0..(size / 2) {
        assert_eq!(
            lfs_file_write(
                lfs.as_mut_ptr(),
                files[0].as_mut_ptr(),
                b"e".as_ptr() as *const core::ffi::c_void,
                1
            ),
            1
        );
        assert_eq!(
            lfs_file_write(
                lfs.as_mut_ptr(),
                files[1].as_mut_ptr(),
                b"f".as_ptr() as *const core::ffi::c_void,
                1
            ),
            1
        );
        assert_eq!(
            lfs_file_write(
                lfs.as_mut_ptr(),
                files[2].as_mut_ptr(),
                b"g".as_ptr() as *const core::ffi::c_void,
                1
            ),
            1
        );
    }

    // Remove "f" while it's still open
    assert_ok(lfs_remove(lfs.as_mut_ptr(), path_f.as_ptr()));

    // Write another SIZE/2 bytes to all three
    for _i in 0..(size / 2) {
        assert_eq!(
            lfs_file_write(
                lfs.as_mut_ptr(),
                files[0].as_mut_ptr(),
                b"e".as_ptr() as *const core::ffi::c_void,
                1
            ),
            1
        );
        assert_eq!(
            lfs_file_write(
                lfs.as_mut_ptr(),
                files[1].as_mut_ptr(),
                b"f".as_ptr() as *const core::ffi::c_void,
                1
            ),
            1
        );
        assert_eq!(
            lfs_file_write(
                lfs.as_mut_ptr(),
                files[2].as_mut_ptr(),
                b"g".as_ptr() as *const core::ffi::c_void,
                1
            ),
            1
        );
    }

    assert_ok(lfs_file_close(lfs.as_mut_ptr(), files[0].as_mut_ptr()));
    assert_ok(lfs_file_close(lfs.as_mut_ptr(), files[1].as_mut_ptr()));
    assert_ok(lfs_file_close(lfs.as_mut_ptr(), files[2].as_mut_ptr()));

    // Verify directory: "e" and "g" present, "f" absent
    let root = path_bytes("/");
    let mut dir = core::mem::MaybeUninit::<LfsDir>::zeroed();
    assert_ok(lfs_dir_open(
        lfs.as_mut_ptr(),
        dir.as_mut_ptr(),
        root.as_ptr(),
    ));

    let mut info = core::mem::MaybeUninit::<LfsInfo>::zeroed();

    assert_eq!(
        lfs_dir_read(lfs.as_mut_ptr(), dir.as_mut_ptr(), info.as_mut_ptr()),
        1
    );
    let info_ref = unsafe { &*info.as_ptr() };
    assert_eq!(&info_ref.name[..1], b".");
    assert_eq!(info_ref.type_, LFS_TYPE_DIR);

    assert_eq!(
        lfs_dir_read(lfs.as_mut_ptr(), dir.as_mut_ptr(), info.as_mut_ptr()),
        1
    );
    let info_ref = unsafe { &*info.as_ptr() };
    assert_eq!(&info_ref.name[..2], b"..");
    assert_eq!(info_ref.type_, LFS_TYPE_DIR);

    assert_eq!(
        lfs_dir_read(lfs.as_mut_ptr(), dir.as_mut_ptr(), info.as_mut_ptr()),
        1
    );
    let info_ref = unsafe { &*info.as_ptr() };
    let nul = info_ref.name.iter().position(|&b| b == 0).unwrap_or(256);
    assert_eq!(core::str::from_utf8(&info_ref.name[..nul]).unwrap(), "e");
    assert_eq!(info_ref.type_, LFS_TYPE_REG);
    assert_eq!(info_ref.size, size as u32);

    assert_eq!(
        lfs_dir_read(lfs.as_mut_ptr(), dir.as_mut_ptr(), info.as_mut_ptr()),
        1
    );
    let info_ref = unsafe { &*info.as_ptr() };
    let nul = info_ref.name.iter().position(|&b| b == 0).unwrap_or(256);
    assert_eq!(core::str::from_utf8(&info_ref.name[..nul]).unwrap(), "g");
    assert_eq!(info_ref.type_, LFS_TYPE_REG);
    assert_eq!(info_ref.size, size as u32);

    assert_eq!(
        lfs_dir_read(lfs.as_mut_ptr(), dir.as_mut_ptr(), info.as_mut_ptr()),
        0
    );
    assert_ok(lfs_dir_close(lfs.as_mut_ptr(), dir.as_mut_ptr()));

    // Read "e" and "g", verify SIZE bytes
    let mut files_r: [core::mem::MaybeUninit<LfsFile>; 2] = [
        core::mem::MaybeUninit::zeroed(),
        core::mem::MaybeUninit::zeroed(),
    ];
    assert_ok(lfs_file_open(
        lfs.as_mut_ptr(),
        files_r[0].as_mut_ptr(),
        path_e.as_ptr(),
        LFS_O_RDONLY,
    ));
    assert_ok(lfs_file_open(
        lfs.as_mut_ptr(),
        files_r[1].as_mut_ptr(),
        path_g.as_ptr(),
        LFS_O_RDONLY,
    ));

    for _i in 0..size {
        let mut buffer = [0u8; 1];
        assert_eq!(
            lfs_file_read(
                lfs.as_mut_ptr(),
                files_r[0].as_mut_ptr(),
                buffer.as_mut_ptr() as *mut core::ffi::c_void,
                1
            ),
            1
        );
        assert_eq!(buffer[0], b'e');
        assert_eq!(
            lfs_file_read(
                lfs.as_mut_ptr(),
                files_r[1].as_mut_ptr(),
                buffer.as_mut_ptr() as *mut core::ffi::c_void,
                1
            ),
            1
        );
        assert_eq!(buffer[0], b'g');
    }
    assert_ok(lfs_file_close(lfs.as_mut_ptr(), files_r[0].as_mut_ptr()));
    assert_ok(lfs_file_close(lfs.as_mut_ptr(), files_r[1].as_mut_ptr()));

    assert_ok(lfs_unmount(lfs.as_mut_ptr()));
}

/// Upstream: [cases.test_interspersed_reentrant_files]
/// defines.SIZE = [10, 100]
/// defines.FILES = [4, 10, 26]
/// defines.POWERLOSS_BEHAVIOR = [NOOP, OOO]
/// reentrant = true
///
/// Power-loss test. Mount-or-format. Open FILES files for append. Write
/// SIZE bytes per file with sync after each byte when size <= i. Close.
/// Verify directory and read 10 bytes from each.
#[rstest]
#[ignore = "power-loss"]
fn test_interspersed_reentrant_files(
    #[values(10, 100)] size: usize,
    #[values(4, 10, 26)] files: usize,
) {
    let mut env = default_config(128);
    init_context(&mut env);

    let mut lfs = core::mem::MaybeUninit::<Lfs>::zeroed();

    // Mount-or-format
    let err = lfs_mount(lfs.as_mut_ptr(), &env.config as *const LfsConfig);
    if err != 0 {
        assert_ok(lfs_format(
            lfs.as_mut_ptr(),
            &env.config as *const LfsConfig,
        ));
        assert_ok(lfs_mount(lfs.as_mut_ptr(), &env.config as *const LfsConfig));
    }

    let mut file_handles: Vec<core::mem::MaybeUninit<LfsFile>> = (0..files)
        .map(|_| core::mem::MaybeUninit::zeroed())
        .collect();

    for j in 0..files {
        let path = path_bytes(&String::from(ALPHAS[j] as char));
        assert_ok(lfs_file_open(
            lfs.as_mut_ptr(),
            file_handles[j].as_mut_ptr(),
            path.as_ptr(),
            LFS_O_WRONLY | LFS_O_CREAT | LFS_O_APPEND,
        ));
    }

    for i in 0..size {
        for j in 0..files {
            let file_sz = lfs_file_size(lfs.as_mut_ptr(), file_handles[j].as_mut_ptr());
            assert!(file_sz >= 0);
            if (file_sz as usize) <= i {
                let byte = [ALPHAS[j]];
                let n = lfs_file_write(
                    lfs.as_mut_ptr(),
                    file_handles[j].as_mut_ptr(),
                    byte.as_ptr() as *const core::ffi::c_void,
                    1,
                );
                assert_eq!(n, 1);
                assert_ok(lfs_file_sync(
                    lfs.as_mut_ptr(),
                    file_handles[j].as_mut_ptr(),
                ));
            }
        }
    }

    for j in 0..files {
        assert_ok(lfs_file_close(
            lfs.as_mut_ptr(),
            file_handles[j].as_mut_ptr(),
        ));
    }

    // Verify directory
    let root = path_bytes("/");
    let mut dir = core::mem::MaybeUninit::<LfsDir>::zeroed();
    assert_ok(lfs_dir_open(
        lfs.as_mut_ptr(),
        dir.as_mut_ptr(),
        root.as_ptr(),
    ));

    let mut info = core::mem::MaybeUninit::<LfsInfo>::zeroed();

    assert_eq!(
        lfs_dir_read(lfs.as_mut_ptr(), dir.as_mut_ptr(), info.as_mut_ptr()),
        1
    );
    let info_ref = unsafe { &*info.as_ptr() };
    assert_eq!(&info_ref.name[..1], b".");
    assert_eq!(info_ref.type_, LFS_TYPE_DIR);

    assert_eq!(
        lfs_dir_read(lfs.as_mut_ptr(), dir.as_mut_ptr(), info.as_mut_ptr()),
        1
    );
    let info_ref = unsafe { &*info.as_ptr() };
    assert_eq!(&info_ref.name[..2], b"..");
    assert_eq!(info_ref.type_, LFS_TYPE_DIR);

    for j in 0..files {
        let expected_name = String::from(ALPHAS[j] as char);
        assert_eq!(
            lfs_dir_read(lfs.as_mut_ptr(), dir.as_mut_ptr(), info.as_mut_ptr()),
            1
        );
        let info_ref = unsafe { &*info.as_ptr() };
        let nul = info_ref.name.iter().position(|&b| b == 0).unwrap_or(256);
        let name = core::str::from_utf8(&info_ref.name[..nul]).unwrap();
        assert_eq!(name, expected_name);
        assert_eq!(info_ref.type_, LFS_TYPE_REG);
        assert_eq!(info_ref.size, size as u32);
    }
    assert_eq!(
        lfs_dir_read(lfs.as_mut_ptr(), dir.as_mut_ptr(), info.as_mut_ptr()),
        0
    );
    assert_ok(lfs_dir_close(lfs.as_mut_ptr(), dir.as_mut_ptr()));

    // Read first 10 bytes from each
    let mut file_handles: Vec<core::mem::MaybeUninit<LfsFile>> = (0..files)
        .map(|_| core::mem::MaybeUninit::zeroed())
        .collect();

    for j in 0..files {
        let path = path_bytes(&String::from(ALPHAS[j] as char));
        assert_ok(lfs_file_open(
            lfs.as_mut_ptr(),
            file_handles[j].as_mut_ptr(),
            path.as_ptr(),
            LFS_O_RDONLY,
        ));
    }

    for _i in 0..10 {
        for j in 0..files {
            let mut buffer = [0u8; 1];
            let n = lfs_file_read(
                lfs.as_mut_ptr(),
                file_handles[j].as_mut_ptr(),
                buffer.as_mut_ptr() as *mut core::ffi::c_void,
                1,
            );
            assert_eq!(n, 1);
            assert_eq!(buffer[0], ALPHAS[j]);
        }
    }

    for j in 0..files {
        assert_ok(lfs_file_close(
            lfs.as_mut_ptr(),
            file_handles[j].as_mut_ptr(),
        ));
    }

    assert_ok(lfs_unmount(lfs.as_mut_ptr()));
}
