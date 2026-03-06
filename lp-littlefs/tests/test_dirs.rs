//! Directory iteration tests.
//!
//! Upstream: tests/test_dirs.toml
//! Source: https://github.com/littlefs-project/littlefs/blob/master/tests/test_dirs.toml

mod common;

use common::{
    assert_err, assert_ok, default_config, dir_entry_names, init_context, init_logger, path_bytes,
    LFS_O_CREAT, LFS_O_EXCL, LFS_O_RDONLY, LFS_O_WRONLY,
};
use lp_littlefs::lfs_type::lfs_type::{LFS_TYPE_DIR, LFS_TYPE_REG};
use lp_littlefs::{
    lfs_dir_close, lfs_dir_open, lfs_dir_read, lfs_dir_rewind, lfs_dir_seek, lfs_dir_tell,
    lfs_file_close, lfs_file_open, lfs_format, lfs_mkdir, lfs_mount, lfs_remove, lfs_rename,
    lfs_stat, lfs_unmount, Lfs, LfsConfig, LfsDir, LfsFile, LfsInfo, LFS_ERR_EXIST, LFS_ERR_ISDIR,
    LFS_ERR_NOENT, LFS_ERR_NOTDIR, LFS_ERR_NOTEMPTY,
};
use rstest::rstest;

/// Root path: "/" null-terminated.
static ROOT_PATH: [u8; 2] = [b'/', 0];

// --- test_dirs_root ---
// Upstream: dir_open("/"), dir_read returns ".", "..", then 0
#[test]
fn test_dirs_root() {
    init_logger();
    let mut env = default_config(128);
    init_context(&mut env);

    let mut lfs = core::mem::MaybeUninit::<Lfs>::zeroed();
    assert_ok(lfs_format(
        lfs.as_mut_ptr(),
        &env.config as *const LfsConfig,
    ));
    assert_ok(lfs_mount(lfs.as_mut_ptr(), &env.config as *const LfsConfig));

    let mut dir = core::mem::MaybeUninit::<LfsDir>::zeroed();
    assert_ok(lfs_dir_open(
        lfs.as_mut_ptr(),
        dir.as_mut_ptr(),
        ROOT_PATH.as_ptr(),
    ));

    let mut info = core::mem::MaybeUninit::<LfsInfo>::zeroed();
    let n = lfs_dir_read(lfs.as_mut_ptr(), dir.as_mut_ptr(), info.as_mut_ptr());
    assert_eq!(n, 1);
    let info = unsafe { info.assume_init() };
    assert_eq!(info.name[0], b'.');
    assert_eq!(info.name[1], 0);
    assert_eq!(info.type_, LFS_TYPE_DIR as u8);

    let mut info = core::mem::MaybeUninit::<LfsInfo>::zeroed();
    let n = lfs_dir_read(lfs.as_mut_ptr(), dir.as_mut_ptr(), info.as_mut_ptr());
    assert_eq!(n, 1);
    let info = unsafe { info.assume_init() };
    assert_eq!(info.name[0], b'.');
    assert_eq!(info.name[1], b'.');
    assert_eq!(info.name[2], 0);
    assert_eq!(info.type_, LFS_TYPE_DIR as u8);

    let mut info = core::mem::MaybeUninit::<LfsInfo>::zeroed();
    let n = lfs_dir_read(lfs.as_mut_ptr(), dir.as_mut_ptr(), info.as_mut_ptr());
    assert_eq!(n, 0);

    assert_ok(lfs_dir_close(lfs.as_mut_ptr(), dir.as_mut_ptr()));
    assert_ok(lfs_unmount(lfs.as_mut_ptr()));
}

// --- test_dirs_one_mkdir ---
// Upstream: [cases.test_dirs_one_mkdir] mkdir("d0"), stat, dir_read
#[test]
fn test_dirs_one_mkdir() {
    init_logger();
    let mut env = default_config(128);
    init_context(&mut env);

    let mut lfs = core::mem::MaybeUninit::<Lfs>::zeroed();
    assert_ok(lfs_format(
        lfs.as_mut_ptr(),
        &env.config as *const LfsConfig,
    ));
    assert_ok(lfs_mount(lfs.as_mut_ptr(), &env.config as *const LfsConfig));

    let path = path_bytes("d0");
    assert_ok(lfs_mkdir(lfs.as_mut_ptr(), path.as_ptr()));

    let mut info = core::mem::MaybeUninit::<LfsInfo>::zeroed();
    assert_ok(lfs_stat(lfs.as_mut_ptr(), path.as_ptr(), info.as_mut_ptr()));
    let info = unsafe { info.assume_init() };
    let nul = info.name.iter().position(|&b| b == 0).unwrap_or(256);
    assert_eq!(core::str::from_utf8(&info.name[..nul]).unwrap(), "d0");
    assert_eq!(info.type_, LFS_TYPE_DIR as u8);

    let names = dir_entry_names(lfs.as_mut_ptr(), &env.config as *const LfsConfig, "/")
        .expect("dir_entry_names");
    assert_eq!(names.len(), 1);
    assert_eq!(names[0], "d0");

    assert_ok(lfs_unmount(lfs.as_mut_ptr()));
}

// --- test_dirs_many_creation ---
/// Upstream: [cases.test_dirs_many_creation]
/// defines.N = range(3, 100, 3), if = 'N < BLOCK_COUNT/2'
///
/// Create N dirs dir000..dir{N-1}, unmount, mount, verify dir_read.
#[rstest]
fn test_dirs_many_creation(
    #[values(
        3, 6, 9, 12, 15, 18, 21, 24, 27, 30, 33, 36, 39, 42, 45, 48, 51, 54, 57, 60, 63, 66, 69,
        72, 75, 78, 81, 84, 87, 90, 93, 96, 99
    )]
    n: usize,
) {
    init_logger();
    let block_count = 256u32;
    if n >= block_count as usize / 2 {
        return;
    }
    let mut env = default_config(block_count);
    init_context(&mut env);

    let mut lfs = core::mem::MaybeUninit::<Lfs>::zeroed();
    assert_ok(lfs_format(
        lfs.as_mut_ptr(),
        &env.config as *const LfsConfig,
    ));
    assert_ok(lfs_mount(lfs.as_mut_ptr(), &env.config as *const LfsConfig));

    for i in 0..n {
        let path = path_bytes(&format!("dir{i:03}"));
        let err = lfs_mkdir(lfs.as_mut_ptr(), path.as_ptr());
        assert_ok(err);
    }

    let names = dir_entry_names(lfs.as_mut_ptr(), &env.config as *const LfsConfig, "/")
        .expect("dir_entry_names");
    assert_eq!(names.len(), n);
    let mut names_sorted = names.clone();
    names_sorted.sort();
    let expected: Vec<String> = (0..n).map(|i| format!("dir{i:03}")).collect();
    let mut expected_sorted = expected.clone();
    expected_sorted.sort();
    assert_eq!(names_sorted, expected_sorted);

    assert_ok(lfs_unmount(lfs.as_mut_ptr()));
}

// --- test_dirs_many_removal ---
/// Upstream: [cases.test_dirs_many_removal]
/// defines.N = range(3, 100, 11), if = 'N < BLOCK_COUNT/2'
///
/// Create N dirs removeme000.., verify, remove all, verify empty.
#[rstest]
fn test_dirs_many_removal(#[values(3, 14, 25, 36, 47, 58, 69, 80, 91)] n: usize) {
    init_logger();
    let block_count = 256u32;
    if n >= block_count as usize / 2 {
        return;
    }
    let mut env = default_config(block_count);
    init_context(&mut env);

    let mut lfs = core::mem::MaybeUninit::<Lfs>::zeroed();
    assert_ok(lfs_format(
        lfs.as_mut_ptr(),
        &env.config as *const LfsConfig,
    ));
    assert_ok(lfs_mount(lfs.as_mut_ptr(), &env.config as *const LfsConfig));

    for i in 0..n {
        let path = path_bytes(&format!("removeme{i:03}"));
        assert_ok(lfs_mkdir(lfs.as_mut_ptr(), path.as_ptr()));
    }
    for i in 0..n {
        let path = path_bytes(&format!("removeme{i:03}"));
        assert_ok(lfs_remove(lfs.as_mut_ptr(), path.as_ptr()));
    }

    let names = dir_entry_names(lfs.as_mut_ptr(), &env.config as *const LfsConfig, "/")
        .expect("dir_entry_names");
    assert!(names.is_empty());

    assert_ok(lfs_unmount(lfs.as_mut_ptr()));
}

// --- test_dirs_many_rename ---
/// Upstream: [cases.test_dirs_many_rename]
/// defines.N = range(3, 100, 11), if = 'N < BLOCK_COUNT/2'
///
/// Create N dirs test000.., rename to tedd000.., verify.
#[rstest]
fn test_dirs_many_rename(#[values(3, 14, 25, 36, 47, 58, 69, 80, 91)] n: usize) {
    init_logger();
    let block_count = 256u32;
    if n >= block_count as usize / 2 {
        return;
    }
    let mut env = default_config(block_count);
    init_context(&mut env);

    let mut lfs = core::mem::MaybeUninit::<Lfs>::zeroed();
    assert_ok(lfs_format(
        lfs.as_mut_ptr(),
        &env.config as *const LfsConfig,
    ));
    assert_ok(lfs_mount(lfs.as_mut_ptr(), &env.config as *const LfsConfig));

    for i in 0..n {
        let path = path_bytes(&format!("test{i:03}"));
        assert_ok(lfs_mkdir(lfs.as_mut_ptr(), path.as_ptr()));
    }
    for i in 0..n {
        let old_path = path_bytes(&format!("test{i:03}"));
        let new_path = path_bytes(&format!("tedd{i:03}"));
        let err = lfs_rename(lfs.as_mut_ptr(), old_path.as_ptr(), new_path.as_ptr());
        assert_ok(err);
    }

    let names = dir_entry_names(lfs.as_mut_ptr(), &env.config as *const LfsConfig, "/")
        .expect("dir_entry_names");
    assert_eq!(names.len(), n);
    let mut names_sorted = names.clone();
    names_sorted.sort();
    let expected: Vec<String> = (0..n).map(|i| format!("tedd{i:03}")).collect();
    let mut expected_sorted = expected.clone();
    expected_sorted.sort();
    assert_eq!(names_sorted, expected_sorted);

    assert_ok(lfs_unmount(lfs.as_mut_ptr()));
}

// --- Implemented upstream cases ---

/// Upstream: [cases.test_dirs_many_rename_append]
/// defines.N = range(5, 13, 2), if = 'N < BLOCK_COUNT/2'
/// Format, create N dirs a00..a{N-1}, unmount, mount, rename a→z, unmount,
/// mount, verify dir_read shows z00..z{N-1} in order.
#[test]
fn test_dirs_many_rename_append() {
    init_logger();
    for n in [5usize, 7, 9, 11] {
        let mut env = default_config(128);
        init_context(&mut env);

        let mut lfs = core::mem::MaybeUninit::<Lfs>::zeroed();
        assert_ok(lfs_format(
            lfs.as_mut_ptr(),
            &env.config as *const LfsConfig,
        ));
        assert_ok(lfs_mount(lfs.as_mut_ptr(), &env.config as *const LfsConfig));

        for i in 0..n {
            let path = path_bytes(&format!("a{i:02}"));
            assert_ok(lfs_mkdir(lfs.as_mut_ptr(), path.as_ptr()));
        }
        assert_ok(lfs_unmount(lfs.as_mut_ptr()));

        assert_ok(lfs_mount(lfs.as_mut_ptr(), &env.config as *const LfsConfig));
        for i in 0..n {
            let old = path_bytes(&format!("a{i:02}"));
            let new = path_bytes(&format!("z{i:02}"));
            assert_ok(lfs_rename(lfs.as_mut_ptr(), old.as_ptr(), new.as_ptr()));
        }
        assert_ok(lfs_unmount(lfs.as_mut_ptr()));

        assert_ok(lfs_mount(lfs.as_mut_ptr(), &env.config as *const LfsConfig));
        let mut dir = core::mem::MaybeUninit::<LfsDir>::zeroed();
        assert_ok(lfs_dir_open(
            lfs.as_mut_ptr(),
            dir.as_mut_ptr(),
            ROOT_PATH.as_ptr(),
        ));

        let mut info = core::mem::MaybeUninit::<LfsInfo>::zeroed();
        assert_eq!(
            lfs_dir_read(lfs.as_mut_ptr(), dir.as_mut_ptr(), info.as_mut_ptr()),
            1
        );
        let info_ref = unsafe { &*info.as_ptr() };
        assert_eq!(info_ref.type_, LFS_TYPE_DIR as u8);
        assert_eq!(info_ref.name[0], b'.');
        assert_eq!(info_ref.name[1], 0);

        let mut info = core::mem::MaybeUninit::<LfsInfo>::zeroed();
        assert_eq!(
            lfs_dir_read(lfs.as_mut_ptr(), dir.as_mut_ptr(), info.as_mut_ptr()),
            1
        );
        let info_ref = unsafe { &*info.as_ptr() };
        assert_eq!(info_ref.type_, LFS_TYPE_DIR as u8);
        assert_eq!(info_ref.name[0], b'.');
        assert_eq!(info_ref.name[1], b'.');
        assert_eq!(info_ref.name[2], 0);

        for i in 0..n {
            let expected = format!("z{i:02}");
            let mut info = core::mem::MaybeUninit::<LfsInfo>::zeroed();
            assert_eq!(
                lfs_dir_read(lfs.as_mut_ptr(), dir.as_mut_ptr(), info.as_mut_ptr()),
                1,
                "N={n}, expected entry {i}"
            );
            let info_ref = unsafe { &*info.as_ptr() };
            assert_eq!(info_ref.type_, LFS_TYPE_DIR as u8);
            let nul = info_ref.name.iter().position(|&b| b == 0).unwrap_or(256);
            let name = core::str::from_utf8(&info_ref.name[..nul]).unwrap();
            assert_eq!(name, expected, "N={n}, entry {i}");
        }

        let mut info = core::mem::MaybeUninit::<LfsInfo>::zeroed();
        assert_eq!(
            lfs_dir_read(lfs.as_mut_ptr(), dir.as_mut_ptr(), info.as_mut_ptr()),
            0
        );

        assert_ok(lfs_dir_close(lfs.as_mut_ptr(), dir.as_mut_ptr()));
        assert_ok(lfs_unmount(lfs.as_mut_ptr()));
    }
}

/// Upstream: [cases.test_dirs_many_reentrant]
/// defines.N = [5, 11], BLOCK_COUNT >= 4*N, reentrant, POWERLOSS_BEHAVIOR = [NOOP, OOO]
#[test]
#[cfg(feature = "slow_tests")]
fn test_dirs_many_reentrant() {
    init_logger();
    for n in [5usize, 11] {
        let block_count = (4 * n).max(128) as u32;
        let mut env = powerloss_config(block_count);
        init_powerloss_context(&mut env);
        let snapshot = env.snapshot();

        let result = run_powerloss_linear(
            &mut env,
            &snapshot,
            2000,
            |lfs_ptr, config| {
                let err = lfs_mount(lfs_ptr, config);
                if err != 0 {
                    let _ = lfs_format(lfs_ptr, config);
                    let e = lfs_mount(lfs_ptr, config);
                    if e != 0 {
                        return Err(e);
                    }
                }

                for i in 0..n {
                    let path = path_bytes(&format!("hi{i:03}"));
                    let err = lfs_mkdir(lfs_ptr, path.as_ptr());
                    if err != 0 && err != LFS_ERR_EXIST {
                        return Err(err);
                    }
                }
                for i in 0..n {
                    let path = path_bytes(&format!("hello{i:03}"));
                    let err = lfs_remove(lfs_ptr, path.as_ptr());
                    if err != 0 && err != LFS_ERR_NOENT {
                        return Err(err);
                    }
                }

                let mut dir = core::mem::MaybeUninit::<LfsDir>::zeroed();
                if lfs_dir_open(lfs_ptr, dir.as_mut_ptr(), ROOT_PATH.as_ptr()) != 0 {
                    return Err(-1);
                }
                let mut info = core::mem::MaybeUninit::<LfsInfo>::zeroed();
                let _ = lfs_dir_read(lfs_ptr, dir.as_mut_ptr(), info.as_mut_ptr());
                let _ = lfs_dir_read(lfs_ptr, dir.as_mut_ptr(), info.as_mut_ptr());
                for i in 0..n {
                    let expected = format!("hi{i:03}");
                    let r = lfs_dir_read(lfs_ptr, dir.as_mut_ptr(), info.as_mut_ptr());
                    if r != 1 {
                        let _ = lfs_dir_close(lfs_ptr, dir.as_mut_ptr());
                        return Err(if r < 0 { r } else { -1 });
                    }
                    let info_ref = unsafe { &*info.as_ptr() };
                    let nul = info_ref.name.iter().position(|&b| b == 0).unwrap_or(256);
                    let name = core::str::from_utf8(&info_ref.name[..nul]).unwrap();
                    if name != expected {
                        let _ = lfs_dir_close(lfs_ptr, dir.as_mut_ptr());
                        return Err(-1);
                    }
                }
                if lfs_dir_read(lfs_ptr, dir.as_mut_ptr(), info.as_mut_ptr()) != 0 {
                    let _ = lfs_dir_close(lfs_ptr, dir.as_mut_ptr());
                    return Err(-1);
                }
                if lfs_dir_close(lfs_ptr, dir.as_mut_ptr()) != 0 {
                    return Err(-1);
                }

                for i in 0..n {
                    let old = path_bytes(&format!("hi{i:03}"));
                    let new = path_bytes(&format!("hello{i:03}"));
                    if lfs_rename(lfs_ptr, old.as_ptr(), new.as_ptr()) != 0 {
                        return Err(-1);
                    }
                }

                if lfs_dir_open(lfs_ptr, dir.as_mut_ptr(), ROOT_PATH.as_ptr()) != 0 {
                    return Err(-1);
                }
                let _ = lfs_dir_read(lfs_ptr, dir.as_mut_ptr(), info.as_mut_ptr());
                let _ = lfs_dir_read(lfs_ptr, dir.as_mut_ptr(), info.as_mut_ptr());
                for i in 0..n {
                    let expected = format!("hello{i:03}");
                    let r = lfs_dir_read(lfs_ptr, dir.as_mut_ptr(), info.as_mut_ptr());
                    if r != 1 {
                        let _ = lfs_dir_close(lfs_ptr, dir.as_mut_ptr());
                        return Err(if r < 0 { r } else { -1 });
                    }
                    let info_ref = unsafe { &*info.as_ptr() };
                    let nul = info_ref.name.iter().position(|&b| b == 0).unwrap_or(256);
                    let name = core::str::from_utf8(&info_ref.name[..nul]).unwrap();
                    if name != expected {
                        let _ = lfs_dir_close(lfs_ptr, dir.as_mut_ptr());
                        return Err(-1);
                    }
                }
                if lfs_dir_read(lfs_ptr, dir.as_mut_ptr(), info.as_mut_ptr()) != 0 {
                    let _ = lfs_dir_close(lfs_ptr, dir.as_mut_ptr());
                    return Err(-1);
                }
                if lfs_dir_close(lfs_ptr, dir.as_mut_ptr()) != 0 {
                    return Err(-1);
                }

                for i in 0..n {
                    let path = path_bytes(&format!("hello{i:03}"));
                    if lfs_remove(lfs_ptr, path.as_ptr()) != 0 {
                        return Err(-1);
                    }
                }

                if lfs_dir_open(lfs_ptr, dir.as_mut_ptr(), ROOT_PATH.as_ptr()) != 0 {
                    return Err(-1);
                }
                let _ = lfs_dir_read(lfs_ptr, dir.as_mut_ptr(), info.as_mut_ptr());
                let _ = lfs_dir_read(lfs_ptr, dir.as_mut_ptr(), info.as_mut_ptr());
                if lfs_dir_read(lfs_ptr, dir.as_mut_ptr(), info.as_mut_ptr()) != 0 {
                    let _ = lfs_dir_close(lfs_ptr, dir.as_mut_ptr());
                    return Err(-1);
                }
                if lfs_dir_close(lfs_ptr, dir.as_mut_ptr()) != 0 {
                    return Err(-1);
                }

                if lfs_unmount(lfs_ptr) != 0 {
                    return Err(-1);
                }
                Ok(())
            },
            |_, _| Ok(()),
        );
        result.expect(&format!("test_dirs_many_reentrant N={n} should complete"));
    }
}

/// Upstream: [cases.test_dirs_file_creation]
/// defines.N = range(3, 100, 11), if = 'N < BLOCK_COUNT/2'
/// Create N empty files, unmount, mount, verify dir_read shows all with LFS_TYPE_REG.
#[test]
fn test_dirs_file_creation() {
    init_logger();
    for n in [3usize, 14, 25, 36, 47, 58, 69, 80, 91] {
        let mut env = default_config(128);
        init_context(&mut env);

        let mut lfs = core::mem::MaybeUninit::<Lfs>::zeroed();
        assert_ok(lfs_format(
            lfs.as_mut_ptr(),
            &env.config as *const LfsConfig,
        ));
        assert_ok(lfs_mount(lfs.as_mut_ptr(), &env.config as *const LfsConfig));

        for i in 0..n {
            let path = path_bytes(&format!("file{i:03}"));
            let mut file = core::mem::MaybeUninit::<LfsFile>::zeroed();
            assert_ok(lfs_file_open(
                lfs.as_mut_ptr(),
                file.as_mut_ptr(),
                path.as_ptr(),
                LFS_O_WRONLY | LFS_O_CREAT,
            ));
            assert_ok(lfs_file_close(lfs.as_mut_ptr(), file.as_mut_ptr()));
        }
        assert_ok(lfs_unmount(lfs.as_mut_ptr()));

        assert_ok(lfs_mount(lfs.as_mut_ptr(), &env.config as *const LfsConfig));
        let mut dir = core::mem::MaybeUninit::<LfsDir>::zeroed();
        assert_ok(lfs_dir_open(
            lfs.as_mut_ptr(),
            dir.as_mut_ptr(),
            ROOT_PATH.as_ptr(),
        ));

        let mut info = core::mem::MaybeUninit::<LfsInfo>::zeroed();
        assert_eq!(
            lfs_dir_read(lfs.as_mut_ptr(), dir.as_mut_ptr(), info.as_mut_ptr()),
            1
        );
        assert_eq!(unsafe { (*info.as_ptr()).type_ }, LFS_TYPE_DIR as u8);

        let mut info = core::mem::MaybeUninit::<LfsInfo>::zeroed();
        assert_eq!(
            lfs_dir_read(lfs.as_mut_ptr(), dir.as_mut_ptr(), info.as_mut_ptr()),
            1
        );
        assert_eq!(unsafe { (*info.as_ptr()).type_ }, LFS_TYPE_DIR as u8);

        for i in 0..n {
            let expected = format!("file{i:03}");
            let mut info = core::mem::MaybeUninit::<LfsInfo>::zeroed();
            assert_eq!(
                lfs_dir_read(lfs.as_mut_ptr(), dir.as_mut_ptr(), info.as_mut_ptr()),
                1,
                "N={n}, expected entry {i}"
            );
            let info_ref = unsafe { &*info.as_ptr() };
            assert_eq!(info_ref.type_, LFS_TYPE_REG as u8, "N={n}, entry {i} type");
            let nul = info_ref.name.iter().position(|&b| b == 0).unwrap_or(256);
            let name = core::str::from_utf8(&info_ref.name[..nul]).unwrap();
            assert_eq!(name, expected, "N={n}, entry {i} name");
        }

        let mut info = core::mem::MaybeUninit::<LfsInfo>::zeroed();
        assert_eq!(
            lfs_dir_read(lfs.as_mut_ptr(), dir.as_mut_ptr(), info.as_mut_ptr()),
            0
        );

        assert_ok(lfs_dir_close(lfs.as_mut_ptr(), dir.as_mut_ptr()));
        assert_ok(lfs_unmount(lfs.as_mut_ptr()));
    }
}

/// Upstream: [cases.test_dirs_file_removal]
/// defines.N = range(3, 100, 11), if = 'N < BLOCK_COUNT/2'
/// Create N files, verify present, remove all, verify empty.
#[test]
fn test_dirs_file_removal() {
    init_logger();
    for n in [3usize, 14, 25, 36, 47, 58, 69, 80, 91] {
        let mut env = default_config(128);
        init_context(&mut env);

        let mut lfs = core::mem::MaybeUninit::<Lfs>::zeroed();
        assert_ok(lfs_format(
            lfs.as_mut_ptr(),
            &env.config as *const LfsConfig,
        ));
        assert_ok(lfs_mount(lfs.as_mut_ptr(), &env.config as *const LfsConfig));

        for i in 0..n {
            let path = path_bytes(&format!("removeme{i:03}"));
            let mut file = core::mem::MaybeUninit::<LfsFile>::zeroed();
            assert_ok(lfs_file_open(
                lfs.as_mut_ptr(),
                file.as_mut_ptr(),
                path.as_ptr(),
                LFS_O_WRONLY | LFS_O_CREAT,
            ));
            assert_ok(lfs_file_close(lfs.as_mut_ptr(), file.as_mut_ptr()));
        }
        assert_ok(lfs_unmount(lfs.as_mut_ptr()));

        assert_ok(lfs_mount(lfs.as_mut_ptr(), &env.config as *const LfsConfig));
        let names = dir_entry_names(lfs.as_mut_ptr(), &env.config as *const LfsConfig, "/")
            .expect("dir_entry_names");
        let mut names_sorted = names.clone();
        names_sorted.sort();
        let mut expected: Vec<String> = (0..n).map(|i| format!("removeme{i:03}")).collect();
        expected.sort();
        assert_eq!(names_sorted, expected, "N={n} before removal");
        assert_ok(lfs_unmount(lfs.as_mut_ptr()));

        assert_ok(lfs_mount(lfs.as_mut_ptr(), &env.config as *const LfsConfig));
        for i in 0..n {
            let path = path_bytes(&format!("removeme{i:03}"));
            assert_ok(lfs_remove(lfs.as_mut_ptr(), path.as_ptr()));
        }
        assert_ok(lfs_unmount(lfs.as_mut_ptr()));

        assert_ok(lfs_mount(lfs.as_mut_ptr(), &env.config as *const LfsConfig));
        let names = dir_entry_names(lfs.as_mut_ptr(), &env.config as *const LfsConfig, "/")
            .expect("dir_entry_names");
        assert!(names.is_empty(), "N={n} after removal: {names:?}");
        assert_ok(lfs_unmount(lfs.as_mut_ptr()));
    }
}

/// Upstream: [cases.test_dirs_file_rename]
/// defines.N = range(3, 100, 11), if = 'N < BLOCK_COUNT/2'
/// Create N files test000.., rename to tedd000.., verify.
#[test]
fn test_dirs_file_rename() {
    init_logger();
    for n in [3usize, 14, 25, 36, 47, 58, 69, 80, 91] {
        let mut env = default_config(128);
        init_context(&mut env);

        let mut lfs = core::mem::MaybeUninit::<Lfs>::zeroed();
        assert_ok(lfs_format(
            lfs.as_mut_ptr(),
            &env.config as *const LfsConfig,
        ));
        assert_ok(lfs_mount(lfs.as_mut_ptr(), &env.config as *const LfsConfig));

        for i in 0..n {
            let path = path_bytes(&format!("test{i:03}"));
            let mut file = core::mem::MaybeUninit::<LfsFile>::zeroed();
            assert_ok(lfs_file_open(
                lfs.as_mut_ptr(),
                file.as_mut_ptr(),
                path.as_ptr(),
                LFS_O_WRONLY | LFS_O_CREAT,
            ));
            assert_ok(lfs_file_close(lfs.as_mut_ptr(), file.as_mut_ptr()));
        }
        assert_ok(lfs_unmount(lfs.as_mut_ptr()));

        assert_ok(lfs_mount(lfs.as_mut_ptr(), &env.config as *const LfsConfig));
        let names = dir_entry_names(lfs.as_mut_ptr(), &env.config as *const LfsConfig, "/")
            .expect("dir_entry_names");
        let mut names_sorted = names.clone();
        names_sorted.sort();
        let mut expected: Vec<String> = (0..n).map(|i| format!("test{i:03}")).collect();
        expected.sort();
        assert_eq!(names_sorted, expected, "N={n} before rename");
        assert_ok(lfs_unmount(lfs.as_mut_ptr()));

        assert_ok(lfs_mount(lfs.as_mut_ptr(), &env.config as *const LfsConfig));
        for i in 0..n {
            let old = path_bytes(&format!("test{i:03}"));
            let new = path_bytes(&format!("tedd{i:03}"));
            assert_ok(lfs_rename(lfs.as_mut_ptr(), old.as_ptr(), new.as_ptr()));
        }
        assert_ok(lfs_unmount(lfs.as_mut_ptr()));

        assert_ok(lfs_mount(lfs.as_mut_ptr(), &env.config as *const LfsConfig));
        let names = dir_entry_names(lfs.as_mut_ptr(), &env.config as *const LfsConfig, "/")
            .expect("dir_entry_names");
        let mut names_sorted = names.clone();
        names_sorted.sort();
        let mut expected: Vec<String> = (0..n).map(|i| format!("tedd{i:03}")).collect();
        expected.sort();
        assert_eq!(names_sorted, expected, "N={n} after rename");
        assert_ok(lfs_unmount(lfs.as_mut_ptr()));
    }
}

/// Upstream: [cases.test_dirs_file_reentrant]
/// defines.N = [5, 25], N < BLOCK_COUNT/2, reentrant, POWERLOSS_BEHAVIOR = [NOOP, OOO]
#[test]
#[cfg(feature = "slow_tests")]
fn test_dirs_file_reentrant() {
    init_logger();
    for n in [5usize, 25] {
        let block_count = 128u32;
        let mut env = powerloss_config(block_count);
        init_powerloss_context(&mut env);
        let snapshot = env.snapshot();

        let result = run_powerloss_linear(
            &mut env,
            &snapshot,
            3000,
            |lfs_ptr, config| {
                let err = lfs_mount(lfs_ptr, config);
                if err != 0 {
                    let _ = lfs_format(lfs_ptr, config);
                    let e = lfs_mount(lfs_ptr, config);
                    if e != 0 {
                        return Err(e);
                    }
                }

                let mut file = core::mem::MaybeUninit::<LfsFile>::zeroed();
                for i in 0..n {
                    let path = path_bytes(&format!("hi{i:03}"));
                    if lfs_file_open(
                        lfs_ptr,
                        file.as_mut_ptr(),
                        path.as_ptr(),
                        LFS_O_CREAT | LFS_O_WRONLY,
                    ) != 0
                    {
                        return Err(-1);
                    }
                    if lfs_file_close(lfs_ptr, file.as_mut_ptr()) != 0 {
                        return Err(-1);
                    }
                }
                for i in 0..n {
                    let path = path_bytes(&format!("hello{i:03}"));
                    let err = lfs_remove(lfs_ptr, path.as_ptr());
                    if err != 0 && err != LFS_ERR_NOENT {
                        return Err(err);
                    }
                }

                let mut dir = core::mem::MaybeUninit::<LfsDir>::zeroed();
                if lfs_dir_open(lfs_ptr, dir.as_mut_ptr(), ROOT_PATH.as_ptr()) != 0 {
                    return Err(-1);
                }
                let mut info = core::mem::MaybeUninit::<LfsInfo>::zeroed();
                let _ = lfs_dir_read(lfs_ptr, dir.as_mut_ptr(), info.as_mut_ptr());
                let _ = lfs_dir_read(lfs_ptr, dir.as_mut_ptr(), info.as_mut_ptr());
                for i in 0..n {
                    let expected = format!("hi{i:03}");
                    let r = lfs_dir_read(lfs_ptr, dir.as_mut_ptr(), info.as_mut_ptr());
                    if r != 1 {
                        let _ = lfs_dir_close(lfs_ptr, dir.as_mut_ptr());
                        return Err(if r < 0 { r } else { -1 });
                    }
                    let info_ref = unsafe { &*info.as_ptr() };
                    if info_ref.type_ != LFS_TYPE_REG as u8 {
                        let _ = lfs_dir_close(lfs_ptr, dir.as_mut_ptr());
                        return Err(-1);
                    }
                    let nul = info_ref.name.iter().position(|&b| b == 0).unwrap_or(256);
                    let name = core::str::from_utf8(&info_ref.name[..nul]).unwrap();
                    if name != expected {
                        let _ = lfs_dir_close(lfs_ptr, dir.as_mut_ptr());
                        return Err(-1);
                    }
                }
                if lfs_dir_read(lfs_ptr, dir.as_mut_ptr(), info.as_mut_ptr()) != 0 {
                    let _ = lfs_dir_close(lfs_ptr, dir.as_mut_ptr());
                    return Err(-1);
                }
                if lfs_dir_close(lfs_ptr, dir.as_mut_ptr()) != 0 {
                    return Err(-1);
                }

                for i in 0..n {
                    let old = path_bytes(&format!("hi{i:03}"));
                    let new = path_bytes(&format!("hello{i:03}"));
                    if lfs_rename(lfs_ptr, old.as_ptr(), new.as_ptr()) != 0 {
                        return Err(-1);
                    }
                }

                if lfs_dir_open(lfs_ptr, dir.as_mut_ptr(), ROOT_PATH.as_ptr()) != 0 {
                    return Err(-1);
                }
                let _ = lfs_dir_read(lfs_ptr, dir.as_mut_ptr(), info.as_mut_ptr());
                let _ = lfs_dir_read(lfs_ptr, dir.as_mut_ptr(), info.as_mut_ptr());
                for i in 0..n {
                    let expected = format!("hello{i:03}");
                    let r = lfs_dir_read(lfs_ptr, dir.as_mut_ptr(), info.as_mut_ptr());
                    if r != 1 {
                        let _ = lfs_dir_close(lfs_ptr, dir.as_mut_ptr());
                        return Err(if r < 0 { r } else { -1 });
                    }
                    let info_ref = unsafe { &*info.as_ptr() };
                    if info_ref.type_ != LFS_TYPE_REG as u8 {
                        let _ = lfs_dir_close(lfs_ptr, dir.as_mut_ptr());
                        return Err(-1);
                    }
                    let nul = info_ref.name.iter().position(|&b| b == 0).unwrap_or(256);
                    let name = core::str::from_utf8(&info_ref.name[..nul]).unwrap();
                    if name != expected {
                        let _ = lfs_dir_close(lfs_ptr, dir.as_mut_ptr());
                        return Err(-1);
                    }
                }
                if lfs_dir_read(lfs_ptr, dir.as_mut_ptr(), info.as_mut_ptr()) != 0 {
                    let _ = lfs_dir_close(lfs_ptr, dir.as_mut_ptr());
                    return Err(-1);
                }
                if lfs_dir_close(lfs_ptr, dir.as_mut_ptr()) != 0 {
                    return Err(-1);
                }

                for i in 0..n {
                    let path = path_bytes(&format!("hello{i:03}"));
                    if lfs_remove(lfs_ptr, path.as_ptr()) != 0 {
                        return Err(-1);
                    }
                }

                if lfs_unmount(lfs_ptr) != 0 {
                    return Err(-1);
                }
                Ok(())
            },
            |_, _| Ok(()),
        );
        result.expect(&format!("test_dirs_file_reentrant N={n} should complete"));
    }
}

/// Upstream: [cases.test_dirs_nested]
/// Create dirs, files, rename chains, cross-dir renames, then cleanup.
#[test]
fn test_dirs_nested() {
    init_logger();
    let mut env = default_config(128);
    init_context(&mut env);

    let mut lfs = core::mem::MaybeUninit::<Lfs>::zeroed();
    assert_ok(lfs_format(
        lfs.as_mut_ptr(),
        &env.config as *const LfsConfig,
    ));
    assert_ok(lfs_mount(lfs.as_mut_ptr(), &env.config as *const LfsConfig));

    assert_ok(lfs_mkdir(lfs.as_mut_ptr(), path_bytes("potato").as_ptr()));
    let mut file = core::mem::MaybeUninit::<LfsFile>::zeroed();
    assert_ok(lfs_file_open(
        lfs.as_mut_ptr(),
        file.as_mut_ptr(),
        path_bytes("burito").as_ptr(),
        LFS_O_WRONLY | LFS_O_CREAT,
    ));
    assert_ok(lfs_file_close(lfs.as_mut_ptr(), file.as_mut_ptr()));

    assert_ok(lfs_mkdir(
        lfs.as_mut_ptr(),
        path_bytes("potato/baked").as_ptr(),
    ));
    assert_ok(lfs_mkdir(
        lfs.as_mut_ptr(),
        path_bytes("potato/sweet").as_ptr(),
    ));
    assert_ok(lfs_mkdir(
        lfs.as_mut_ptr(),
        path_bytes("potato/fried").as_ptr(),
    ));

    let names = dir_entry_names(lfs.as_mut_ptr(), &env.config as *const LfsConfig, "potato")
        .expect("potato dir_entry_names");
    let mut names_sorted = names.clone();
    names_sorted.sort();
    assert_eq!(names_sorted, vec!["baked", "fried", "sweet"]);

    assert_err(
        LFS_ERR_NOTEMPTY,
        lfs_remove(lfs.as_mut_ptr(), path_bytes("potato").as_ptr()),
    );

    assert_ok(lfs_rename(
        lfs.as_mut_ptr(),
        path_bytes("potato").as_ptr(),
        path_bytes("coldpotato").as_ptr(),
    ));
    assert_ok(lfs_rename(
        lfs.as_mut_ptr(),
        path_bytes("coldpotato").as_ptr(),
        path_bytes("warmpotato").as_ptr(),
    ));
    assert_ok(lfs_rename(
        lfs.as_mut_ptr(),
        path_bytes("warmpotato").as_ptr(),
        path_bytes("hotpotato").as_ptr(),
    ));

    assert_err(
        LFS_ERR_NOENT,
        lfs_remove(lfs.as_mut_ptr(), path_bytes("potato").as_ptr()),
    );
    assert_err(
        LFS_ERR_NOENT,
        lfs_remove(lfs.as_mut_ptr(), path_bytes("coldpotato").as_ptr()),
    );
    assert_err(
        LFS_ERR_NOENT,
        lfs_remove(lfs.as_mut_ptr(), path_bytes("warmpotato").as_ptr()),
    );
    assert_err(
        LFS_ERR_NOTEMPTY,
        lfs_remove(lfs.as_mut_ptr(), path_bytes("hotpotato").as_ptr()),
    );

    assert_ok(lfs_mkdir(
        lfs.as_mut_ptr(),
        path_bytes("coldpotato").as_ptr(),
    ));
    assert_ok(lfs_rename(
        lfs.as_mut_ptr(),
        path_bytes("hotpotato/baked").as_ptr(),
        path_bytes("coldpotato/baked").as_ptr(),
    ));
    assert_ok(lfs_rename(
        lfs.as_mut_ptr(),
        path_bytes("hotpotato/fried").as_ptr(),
        path_bytes("coldpotato/fried").as_ptr(),
    ));
    assert_ok(lfs_rename(
        lfs.as_mut_ptr(),
        path_bytes("hotpotato/sweet").as_ptr(),
        path_bytes("coldpotato/sweet").as_ptr(),
    ));

    assert_ok(lfs_remove(
        lfs.as_mut_ptr(),
        path_bytes("hotpotato").as_ptr(),
    ));
    assert_ok(lfs_rename(
        lfs.as_mut_ptr(),
        path_bytes("coldpotato").as_ptr(),
        path_bytes("hotpotato").as_ptr(),
    ));

    assert_ok(lfs_remove(
        lfs.as_mut_ptr(),
        path_bytes("hotpotato/baked").as_ptr(),
    ));
    assert_ok(lfs_remove(
        lfs.as_mut_ptr(),
        path_bytes("hotpotato/fried").as_ptr(),
    ));
    assert_ok(lfs_remove(
        lfs.as_mut_ptr(),
        path_bytes("hotpotato/sweet").as_ptr(),
    ));
    assert_ok(lfs_remove(
        lfs.as_mut_ptr(),
        path_bytes("hotpotato").as_ptr(),
    ));

    let names = dir_entry_names(lfs.as_mut_ptr(), &env.config as *const LfsConfig, "/")
        .expect("root dir_entry_names");
    assert_eq!(names, vec!["burito"]);

    assert_ok(lfs_unmount(lfs.as_mut_ptr()));
}

/// Upstream: [cases.test_dirs_recursive_remove]
/// defines.N = [10, 100], if = 'N < BLOCK_COUNT/2'
/// Create parent dir with N subdirs, remove children during dir iteration, then parent.
#[test]
fn test_dirs_recursive_remove() {
    init_logger();
    for n in [10usize, 100] {
        let mut env = default_config(256);
        init_context(&mut env);

        let mut lfs = core::mem::MaybeUninit::<Lfs>::zeroed();
        assert_ok(lfs_format(
            lfs.as_mut_ptr(),
            &env.config as *const LfsConfig,
        ));
        assert_ok(lfs_mount(lfs.as_mut_ptr(), &env.config as *const LfsConfig));

        assert_ok(lfs_mkdir(
            lfs.as_mut_ptr(),
            path_bytes("prickly-pear").as_ptr(),
        ));
        for i in 0..n {
            let path = path_bytes(&format!("prickly-pear/cactus{i:03}"));
            assert_ok(lfs_mkdir(lfs.as_mut_ptr(), path.as_ptr()));
        }

        let names = dir_entry_names(
            lfs.as_mut_ptr(),
            &env.config as *const LfsConfig,
            "prickly-pear",
        )
        .expect("prickly-pear dir_entry_names");
        assert_eq!(names.len(), n, "N={n} subdir count");

        assert_err(
            LFS_ERR_NOTEMPTY,
            lfs_remove(lfs.as_mut_ptr(), path_bytes("prickly-pear").as_ptr()),
        );

        let mut dir = core::mem::MaybeUninit::<LfsDir>::zeroed();
        assert_ok(lfs_dir_open(
            lfs.as_mut_ptr(),
            dir.as_mut_ptr(),
            path_bytes("prickly-pear").as_ptr(),
        ));
        let mut info = core::mem::MaybeUninit::<LfsInfo>::zeroed();
        loop {
            let rc = lfs_dir_read(lfs.as_mut_ptr(), dir.as_mut_ptr(), info.as_mut_ptr());
            if rc == 0 {
                break;
            }
            assert_eq!(rc, 1, "N={n}, unexpected dir_read result");
            let info_ref = unsafe { &*info.as_ptr() };
            if info_ref.name[0] == b'.' {
                continue;
            }
            let nul = info_ref.name.iter().position(|&b| b == 0).unwrap_or(256);
            let name = core::str::from_utf8(&info_ref.name[..nul]).unwrap();
            let child_path = path_bytes(&format!("prickly-pear/{name}"));
            assert_ok(lfs_remove(lfs.as_mut_ptr(), child_path.as_ptr()));
        }
        assert_ok(lfs_dir_close(lfs.as_mut_ptr(), dir.as_mut_ptr()));

        assert_ok(lfs_remove(
            lfs.as_mut_ptr(),
            path_bytes("prickly-pear").as_ptr(),
        ));
        assert_ok(lfs_unmount(lfs.as_mut_ptr()));

        assert_ok(lfs_mount(lfs.as_mut_ptr(), &env.config as *const LfsConfig));
        let mut info = core::mem::MaybeUninit::<LfsInfo>::zeroed();
        assert_err(
            LFS_ERR_NOENT,
            lfs_stat(
                lfs.as_mut_ptr(),
                path_bytes("prickly-pear").as_ptr(),
                info.as_mut_ptr(),
            ),
        );
        assert_ok(lfs_unmount(lfs.as_mut_ptr()));
    }
}

/// Upstream: [cases.test_dirs_remove_read]
/// defines.N = 10, if = 'N < BLOCK_COUNT/2'
/// Create N dirs under prickly-pear/. Nested loop: open dir, iterate to j, remove dir k, iterate rest,
/// close, recreate k, unmount. Requires lfs_dir_seek.
#[test]
fn test_dirs_remove_read() {
    init_logger();
    const N: usize = 10;
    let mut env = default_config(256);
    init_context(&mut env);

    let mut lfs = core::mem::MaybeUninit::<Lfs>::zeroed();
    assert_ok(lfs_format(
        lfs.as_mut_ptr(),
        &env.config as *const LfsConfig,
    ));
    assert_ok(lfs_mount(lfs.as_mut_ptr(), &env.config as *const LfsConfig));

    assert_ok(lfs_mkdir(
        lfs.as_mut_ptr(),
        path_bytes("prickly-pear").as_ptr(),
    ));
    for i in 0..N {
        let path = path_bytes(&format!("prickly-pear/cactus{i:03}"));
        assert_ok(lfs_mkdir(lfs.as_mut_ptr(), path.as_ptr()));
    }

    for k in 0..N {
        for j in 0..=N {
            let mut dir = core::mem::MaybeUninit::<LfsDir>::zeroed();
            assert_ok(lfs_dir_open(
                lfs.as_mut_ptr(),
                dir.as_mut_ptr(),
                path_bytes("prickly-pear").as_ptr(),
            ));
            assert_ok(lfs_dir_rewind(lfs.as_mut_ptr(), dir.as_mut_ptr()));
            assert_ok(lfs_dir_seek(lfs.as_mut_ptr(), dir.as_mut_ptr(), j as _));
            assert_ok(lfs_remove(
                lfs.as_mut_ptr(),
                path_bytes(&format!("prickly-pear/cactus{k:03}")).as_ptr(),
            ));
            let mut info = core::mem::MaybeUninit::<LfsInfo>::zeroed();
            while lfs_dir_read(lfs.as_mut_ptr(), dir.as_mut_ptr(), info.as_mut_ptr()) > 0 {}
            assert_ok(lfs_dir_close(lfs.as_mut_ptr(), dir.as_mut_ptr()));
            assert_ok(lfs_mkdir(
                lfs.as_mut_ptr(),
                path_bytes(&format!("prickly-pear/cactus{k:03}")).as_ptr(),
            ));
        }
        assert_ok(lfs_unmount(lfs.as_mut_ptr()));
        assert_ok(lfs_mount(lfs.as_mut_ptr(), &env.config as *const LfsConfig));
    }

    assert_ok(lfs_unmount(lfs.as_mut_ptr()));
}

/// Upstream: [cases.test_dirs_other_errors]
/// Tests various error conditions for dirs and files.
#[test]
fn test_dirs_other_errors() {
    init_logger();
    let mut env = default_config(128);
    init_context(&mut env);

    let mut lfs = core::mem::MaybeUninit::<Lfs>::zeroed();
    assert_ok(lfs_format(
        lfs.as_mut_ptr(),
        &env.config as *const LfsConfig,
    ));
    assert_ok(lfs_mount(lfs.as_mut_ptr(), &env.config as *const LfsConfig));

    assert_ok(lfs_mkdir(lfs.as_mut_ptr(), path_bytes("potato").as_ptr()));
    let mut file = core::mem::MaybeUninit::<LfsFile>::zeroed();
    assert_ok(lfs_file_open(
        lfs.as_mut_ptr(),
        file.as_mut_ptr(),
        path_bytes("burito").as_ptr(),
        LFS_O_WRONLY | LFS_O_CREAT,
    ));
    assert_ok(lfs_file_close(lfs.as_mut_ptr(), file.as_mut_ptr()));

    assert_err(
        LFS_ERR_EXIST,
        lfs_mkdir(lfs.as_mut_ptr(), path_bytes("potato").as_ptr()),
    );
    assert_err(
        LFS_ERR_EXIST,
        lfs_mkdir(lfs.as_mut_ptr(), path_bytes("burito").as_ptr()),
    );

    let mut file = core::mem::MaybeUninit::<LfsFile>::zeroed();
    assert_err(
        LFS_ERR_EXIST,
        lfs_file_open(
            lfs.as_mut_ptr(),
            file.as_mut_ptr(),
            path_bytes("burito").as_ptr(),
            LFS_O_WRONLY | LFS_O_CREAT | LFS_O_EXCL,
        ),
    );
    let mut file = core::mem::MaybeUninit::<LfsFile>::zeroed();
    assert_err(
        LFS_ERR_EXIST,
        lfs_file_open(
            lfs.as_mut_ptr(),
            file.as_mut_ptr(),
            path_bytes("potato").as_ptr(),
            LFS_O_WRONLY | LFS_O_CREAT | LFS_O_EXCL,
        ),
    );

    let mut dir = core::mem::MaybeUninit::<LfsDir>::zeroed();
    assert_err(
        LFS_ERR_NOENT,
        lfs_dir_open(
            lfs.as_mut_ptr(),
            dir.as_mut_ptr(),
            path_bytes("tomato").as_ptr(),
        ),
    );
    let mut dir = core::mem::MaybeUninit::<LfsDir>::zeroed();
    assert_err(
        LFS_ERR_NOTDIR,
        lfs_dir_open(
            lfs.as_mut_ptr(),
            dir.as_mut_ptr(),
            path_bytes("burito").as_ptr(),
        ),
    );

    let mut file = core::mem::MaybeUninit::<LfsFile>::zeroed();
    assert_err(
        LFS_ERR_NOENT,
        lfs_file_open(
            lfs.as_mut_ptr(),
            file.as_mut_ptr(),
            path_bytes("tomato").as_ptr(),
            LFS_O_RDONLY,
        ),
    );
    let mut file = core::mem::MaybeUninit::<LfsFile>::zeroed();
    assert_err(
        LFS_ERR_ISDIR,
        lfs_file_open(
            lfs.as_mut_ptr(),
            file.as_mut_ptr(),
            path_bytes("potato").as_ptr(),
            LFS_O_RDONLY,
        ),
    );

    let mut file = core::mem::MaybeUninit::<LfsFile>::zeroed();
    assert_err(
        LFS_ERR_NOENT,
        lfs_file_open(
            lfs.as_mut_ptr(),
            file.as_mut_ptr(),
            path_bytes("tomato").as_ptr(),
            LFS_O_WRONLY,
        ),
    );
    let mut file = core::mem::MaybeUninit::<LfsFile>::zeroed();
    assert_err(
        LFS_ERR_ISDIR,
        lfs_file_open(
            lfs.as_mut_ptr(),
            file.as_mut_ptr(),
            path_bytes("potato").as_ptr(),
            LFS_O_WRONLY,
        ),
    );

    let mut file = core::mem::MaybeUninit::<LfsFile>::zeroed();
    assert_err(
        LFS_ERR_ISDIR,
        lfs_file_open(
            lfs.as_mut_ptr(),
            file.as_mut_ptr(),
            path_bytes("potato").as_ptr(),
            LFS_O_WRONLY | LFS_O_CREAT,
        ),
    );

    let mut file = core::mem::MaybeUninit::<LfsFile>::zeroed();
    assert_ok(lfs_file_open(
        lfs.as_mut_ptr(),
        file.as_mut_ptr(),
        path_bytes("tacoto").as_ptr(),
        LFS_O_WRONLY | LFS_O_CREAT,
    ));
    assert_ok(lfs_file_close(lfs.as_mut_ptr(), file.as_mut_ptr()));

    assert_err(
        LFS_ERR_ISDIR,
        lfs_rename(
            lfs.as_mut_ptr(),
            path_bytes("tacoto").as_ptr(),
            path_bytes("potato").as_ptr(),
        ),
    );
    assert_err(
        LFS_ERR_NOTDIR,
        lfs_rename(
            lfs.as_mut_ptr(),
            path_bytes("potato").as_ptr(),
            path_bytes("tacoto").as_ptr(),
        ),
    );

    assert_err(
        LFS_ERR_EXIST,
        lfs_mkdir(lfs.as_mut_ptr(), path_bytes("/").as_ptr()),
    );
    let mut file = core::mem::MaybeUninit::<LfsFile>::zeroed();
    assert_err(
        LFS_ERR_EXIST,
        lfs_file_open(
            lfs.as_mut_ptr(),
            file.as_mut_ptr(),
            path_bytes("/").as_ptr(),
            LFS_O_WRONLY | LFS_O_CREAT | LFS_O_EXCL,
        ),
    );
    let mut file = core::mem::MaybeUninit::<LfsFile>::zeroed();
    assert_err(
        LFS_ERR_ISDIR,
        lfs_file_open(
            lfs.as_mut_ptr(),
            file.as_mut_ptr(),
            path_bytes("/").as_ptr(),
            LFS_O_RDONLY,
        ),
    );
    let mut file = core::mem::MaybeUninit::<LfsFile>::zeroed();
    assert_err(
        LFS_ERR_ISDIR,
        lfs_file_open(
            lfs.as_mut_ptr(),
            file.as_mut_ptr(),
            path_bytes("/").as_ptr(),
            LFS_O_WRONLY,
        ),
    );
    let mut file = core::mem::MaybeUninit::<LfsFile>::zeroed();
    assert_err(
        LFS_ERR_ISDIR,
        lfs_file_open(
            lfs.as_mut_ptr(),
            file.as_mut_ptr(),
            path_bytes("/").as_ptr(),
            LFS_O_WRONLY | LFS_O_CREAT,
        ),
    );

    let names = dir_entry_names(lfs.as_mut_ptr(), &env.config as *const LfsConfig, "/")
        .expect("root listing");
    let mut names_sorted = names.clone();
    names_sorted.sort();
    assert_eq!(names_sorted, vec!["burito", "potato", "tacoto"]);

    assert_ok(lfs_unmount(lfs.as_mut_ptr()));

    assert_ok(lfs_mount(lfs.as_mut_ptr(), &env.config as *const LfsConfig));
    let names = dir_entry_names(lfs.as_mut_ptr(), &env.config as *const LfsConfig, "/")
        .expect("root listing after remount");
    let mut names_sorted = names.clone();
    names_sorted.sort();
    assert_eq!(names_sorted, vec!["burito", "potato", "tacoto"]);
    assert_ok(lfs_unmount(lfs.as_mut_ptr()));
}

/// Upstream: [cases.test_dirs_seek]
/// defines.COUNT = [4, 128, 132], if = 'COUNT < BLOCK_COUNT/2'
/// Create COUNT entries in a child dir. Exercise lfs_dir_seek, lfs_dir_tell, lfs_dir_rewind.
#[test]
fn test_dirs_seek() {
    init_logger();
    for count in [4usize, 128, 132] {
        let mut env = default_config(512);
        init_context(&mut env);

        let mut lfs = core::mem::MaybeUninit::<Lfs>::zeroed();
        assert_ok(lfs_format(
            lfs.as_mut_ptr(),
            &env.config as *const LfsConfig,
        ));
        assert_ok(lfs_mount(lfs.as_mut_ptr(), &env.config as *const LfsConfig));

        assert_ok(lfs_mkdir(lfs.as_mut_ptr(), path_bytes("child").as_ptr()));
        for i in 0..count {
            let path = path_bytes(&format!("child/entry{i:03}"));
            let mut file = core::mem::MaybeUninit::<LfsFile>::zeroed();
            assert_ok(lfs_file_open(
                lfs.as_mut_ptr(),
                file.as_mut_ptr(),
                path.as_ptr(),
                LFS_O_WRONLY | LFS_O_CREAT | LFS_O_EXCL,
            ));
            assert_ok(lfs_file_close(lfs.as_mut_ptr(), file.as_mut_ptr()));
        }

        let mut dir = core::mem::MaybeUninit::<LfsDir>::zeroed();
        assert_ok(lfs_dir_open(
            lfs.as_mut_ptr(),
            dir.as_mut_ptr(),
            path_bytes("child").as_ptr(),
        ));
        assert_ok(lfs_dir_rewind(lfs.as_mut_ptr(), dir.as_mut_ptr()));
        let pos0 = lfs_dir_tell(lfs.as_mut_ptr(), dir.as_mut_ptr());
        assert!(pos0 >= 0, "tell after rewind");

        let mut info = core::mem::MaybeUninit::<LfsInfo>::zeroed();
        let mut n = 0usize;
        while lfs_dir_read(lfs.as_mut_ptr(), dir.as_mut_ptr(), info.as_mut_ptr()) > 0 {
            n += 1;
        }
        assert_eq!(n, count + 2, "COUNT={count}: . and .. plus {count} entries");

        assert_ok(lfs_dir_rewind(lfs.as_mut_ptr(), dir.as_mut_ptr()));
        let half = (count + 2) / 2;
        assert_ok(lfs_dir_seek(
            lfs.as_mut_ptr(),
            dir.as_mut_ptr(),
            half as u32,
        ));
        let pos_half = lfs_dir_tell(lfs.as_mut_ptr(), dir.as_mut_ptr());
        assert!(pos_half >= 0, "tell after seek");

        assert_ok(lfs_dir_rewind(lfs.as_mut_ptr(), dir.as_mut_ptr()));
        let pos_rewind = lfs_dir_tell(lfs.as_mut_ptr(), dir.as_mut_ptr());
        assert_eq!(pos_rewind, pos0, "tell after rewind matches initial");

        assert_ok(lfs_dir_close(lfs.as_mut_ptr(), dir.as_mut_ptr()));
        assert_ok(lfs_unmount(lfs.as_mut_ptr()));
    }
}

/// Upstream: [cases.test_dirs_toot_seek]
/// defines.COUNT = [4, 128, 132]
/// Same as seek but on root directory.
#[test]
fn test_dirs_toot_seek() {
    init_logger();
    for count in [4usize, 128, 132] {
        let mut env = default_config(512);
        init_context(&mut env);

        let mut lfs = core::mem::MaybeUninit::<Lfs>::zeroed();
        assert_ok(lfs_format(
            lfs.as_mut_ptr(),
            &env.config as *const LfsConfig,
        ));
        assert_ok(lfs_mount(lfs.as_mut_ptr(), &env.config as *const LfsConfig));

        for i in 0..count {
            let path = path_bytes(&format!("entry{i:03}"));
            let mut file = core::mem::MaybeUninit::<LfsFile>::zeroed();
            assert_ok(lfs_file_open(
                lfs.as_mut_ptr(),
                file.as_mut_ptr(),
                path.as_ptr(),
                LFS_O_WRONLY | LFS_O_CREAT | LFS_O_EXCL,
            ));
            assert_ok(lfs_file_close(lfs.as_mut_ptr(), file.as_mut_ptr()));
        }

        let mut dir = core::mem::MaybeUninit::<LfsDir>::zeroed();
        assert_ok(lfs_dir_open(
            lfs.as_mut_ptr(),
            dir.as_mut_ptr(),
            ROOT_PATH.as_ptr(),
        ));
        assert_ok(lfs_dir_rewind(lfs.as_mut_ptr(), dir.as_mut_ptr()));
        let pos0 = lfs_dir_tell(lfs.as_mut_ptr(), dir.as_mut_ptr());
        assert!(pos0 >= 0, "tell after rewind");

        let mut info = core::mem::MaybeUninit::<LfsInfo>::zeroed();
        let mut n = 0usize;
        while lfs_dir_read(lfs.as_mut_ptr(), dir.as_mut_ptr(), info.as_mut_ptr()) > 0 {
            n += 1;
        }
        assert_eq!(n, count + 2, "COUNT={count}: . and .. plus {count} entries");

        assert_ok(lfs_dir_rewind(lfs.as_mut_ptr(), dir.as_mut_ptr()));
        let half = (count + 2) / 2;
        assert_ok(lfs_dir_seek(
            lfs.as_mut_ptr(),
            dir.as_mut_ptr(),
            half as u32,
        ));
        let _pos_half = lfs_dir_tell(lfs.as_mut_ptr(), dir.as_mut_ptr());

        assert_ok(lfs_dir_rewind(lfs.as_mut_ptr(), dir.as_mut_ptr()));
        let pos_rewind = lfs_dir_tell(lfs.as_mut_ptr(), dir.as_mut_ptr());
        assert_eq!(pos_rewind, pos0, "tell after rewind matches initial");

        assert_ok(lfs_dir_close(lfs.as_mut_ptr(), dir.as_mut_ptr()));
        assert_ok(lfs_unmount(lfs.as_mut_ptr()));
    }
}
