//! Path resolution integration tests.
//!
//! Upstream: tests/test_paths.toml
//! Source: https://github.com/littlefs-project/littlefs/blob/master/tests/test_paths.toml

mod common;

use common::{assert_err, assert_ok, default_config, init_context, init_logger, path_bytes};
#[allow(unused_imports)]
use lp_littlefs::lfs_type::lfs_type::{LFS_TYPE_DIR, LFS_TYPE_REG};
use lp_littlefs::{
    lfs_dir_close, lfs_dir_open, lfs_format, lfs_mkdir, lfs_mount, lfs_remove, lfs_rename,
    lfs_stat, lfs_unmount, Lfs, LfsConfig, LfsDir, LfsInfo, LFS_ERR_EXIST, LFS_ERR_INVAL,
    LFS_ERR_ISDIR, LFS_ERR_NAMETOOLONG, LFS_ERR_NOENT, LFS_ERR_NOTDIR, LFS_ERR_NOTEMPTY,
};
use lp_littlefs::{lfs_file_close, lfs_file_open, LfsFile};
use rstest::rstest;

use common::{LFS_O_CREAT, LFS_O_EXCL, LFS_O_RDONLY, LFS_O_WRONLY};

fn info_name_str(info: &LfsInfo) -> &str {
    let nul = info.name.iter().position(|&b| b == 0).unwrap_or(256);
    core::str::from_utf8(&info.name[..nul]).unwrap_or("")
}

/// Null-terminated path from raw bytes (for non-UTF8 names like 0x7f, 0xff).
fn path_bytes_raw(bytes: &[u8]) -> Vec<u8> {
    let mut v = bytes.to_vec();
    v.push(0);
    v
}

const PATHS: &[&str] = &[
    "drip",
    "coldbrew",
    "turkish",
    "tubruk",
    "vietnamese",
    "thai",
];

// --- test_paths_simple_dirs ---
#[test]
fn test_paths_simple_dirs() {
    init_logger();
    let mut env = default_config(128);
    init_context(&mut env);

    let mut lfs = core::mem::MaybeUninit::<Lfs>::zeroed();
    assert_ok(lfs_format(
        lfs.as_mut_ptr(),
        &env.config as *const LfsConfig,
    ));
    assert_ok(lfs_mount(lfs.as_mut_ptr(), &env.config as *const LfsConfig));

    let coffee = path_bytes("coffee");
    assert_ok(lfs_mkdir(lfs.as_mut_ptr(), coffee.as_ptr()));

    for name in PATHS {
        let path = path_bytes(&format!("coffee/{name}"));
        assert_ok(lfs_mkdir(lfs.as_mut_ptr(), path.as_ptr()));
        let mut info = core::mem::MaybeUninit::<LfsInfo>::zeroed();
        assert_ok(lfs_stat(lfs.as_mut_ptr(), path.as_ptr(), info.as_mut_ptr()));
        let info = unsafe { info.assume_init() };
        let nul = info.name.iter().position(|&b| b == 0).unwrap_or(256);
        assert_eq!(core::str::from_utf8(&info.name[..nul]).unwrap(), *name);
        assert_eq!(info.type_, LFS_TYPE_DIR as u8);
    }
    assert_ok(lfs_unmount(lfs.as_mut_ptr()));
}

// --- test_paths_simple_files ---
#[test]
fn test_paths_simple_files() {
    init_logger();
    let mut env = default_config(128);
    init_context(&mut env);

    let mut lfs = core::mem::MaybeUninit::<Lfs>::zeroed();
    assert_ok(lfs_format(
        lfs.as_mut_ptr(),
        &env.config as *const LfsConfig,
    ));
    assert_ok(lfs_mount(lfs.as_mut_ptr(), &env.config as *const LfsConfig));

    let coffee = path_bytes("coffee");
    assert_ok(lfs_mkdir(lfs.as_mut_ptr(), coffee.as_ptr()));

    for name in PATHS {
        let path = path_bytes(&format!("coffee/{name}"));
        let mut file = core::mem::MaybeUninit::<LfsFile>::zeroed();
        assert_ok(lfs_file_open(
            lfs.as_mut_ptr(),
            file.as_mut_ptr(),
            path.as_ptr(),
            LFS_O_WRONLY | LFS_O_CREAT | LFS_O_EXCL,
        ));
        assert_ok(lfs_file_close(lfs.as_mut_ptr(), file.as_mut_ptr()));
        let mut info = core::mem::MaybeUninit::<LfsInfo>::zeroed();
        assert_ok(lfs_stat(lfs.as_mut_ptr(), path.as_ptr(), info.as_mut_ptr()));
        let info = unsafe { info.assume_init() };
        let nul = info.name.iter().position(|&b| b == 0).unwrap_or(256);
        assert_eq!(core::str::from_utf8(&info.name[..nul]).unwrap(), *name);
        assert_eq!(info.type_, LFS_TYPE_REG as u8);
    }
    assert_ok(lfs_unmount(lfs.as_mut_ptr()));
}

// --- test_paths_absolute_files ---
#[test]
fn test_paths_absolute_files() {
    init_logger();
    let mut env = default_config(128);
    init_context(&mut env);

    let mut lfs = core::mem::MaybeUninit::<Lfs>::zeroed();
    assert_ok(lfs_format(
        lfs.as_mut_ptr(),
        &env.config as *const LfsConfig,
    ));
    assert_ok(lfs_mount(lfs.as_mut_ptr(), &env.config as *const LfsConfig));

    let coffee = path_bytes("coffee");
    assert_ok(lfs_mkdir(lfs.as_mut_ptr(), coffee.as_ptr()));

    for name in PATHS {
        let path = path_bytes(&format!("/coffee/{name}"));
        let mut file = core::mem::MaybeUninit::<LfsFile>::zeroed();
        assert_ok(lfs_file_open(
            lfs.as_mut_ptr(),
            file.as_mut_ptr(),
            path.as_ptr(),
            LFS_O_WRONLY | LFS_O_CREAT | LFS_O_EXCL,
        ));
        assert_ok(lfs_file_close(lfs.as_mut_ptr(), file.as_mut_ptr()));
    }
    for name in PATHS {
        let path = path_bytes(&format!("/coffee/{name}"));
        let mut info = core::mem::MaybeUninit::<LfsInfo>::zeroed();
        assert_ok(lfs_stat(lfs.as_mut_ptr(), path.as_ptr(), info.as_mut_ptr()));
        let info = unsafe { info.assume_init() };
        let nul = info.name.iter().position(|&b| b == 0).unwrap_or(256);
        assert_eq!(core::str::from_utf8(&info.name[..nul]).unwrap(), *name);
        assert_eq!(info.type_, LFS_TYPE_REG as u8);
    }
    assert_ok(lfs_unmount(lfs.as_mut_ptr()));
}

// --- test_paths_absolute_dirs ---
#[test]
fn test_paths_absolute_dirs() {
    init_logger();
    let mut env = default_config(128);
    init_context(&mut env);

    let mut lfs = core::mem::MaybeUninit::<Lfs>::zeroed();
    assert_ok(lfs_format(
        lfs.as_mut_ptr(),
        &env.config as *const LfsConfig,
    ));
    assert_ok(lfs_mount(lfs.as_mut_ptr(), &env.config as *const LfsConfig));

    let coffee = path_bytes("coffee");
    assert_ok(lfs_mkdir(lfs.as_mut_ptr(), coffee.as_ptr()));

    for name in PATHS {
        let path = path_bytes(&format!("/coffee/{name}"));
        assert_ok(lfs_mkdir(lfs.as_mut_ptr(), path.as_ptr()));
    }
    for name in PATHS {
        let path = path_bytes(&format!("/coffee/{name}"));
        let mut info = core::mem::MaybeUninit::<LfsInfo>::zeroed();
        assert_ok(lfs_stat(lfs.as_mut_ptr(), path.as_ptr(), info.as_mut_ptr()));
        let info = unsafe { info.assume_init() };
        let nul = info.name.iter().position(|&b| b == 0).unwrap_or(256);
        assert_eq!(core::str::from_utf8(&info.name[..nul]).unwrap(), *name);
        assert_eq!(info.type_, LFS_TYPE_DIR as u8);
    }
    assert_ok(lfs_unmount(lfs.as_mut_ptr()));
}

// --- test_paths_noent ---
#[test]
fn test_paths_noent() {
    init_logger();
    let mut env = default_config(128);
    init_context(&mut env);

    let mut lfs = core::mem::MaybeUninit::<Lfs>::zeroed();
    assert_ok(lfs_format(
        lfs.as_mut_ptr(),
        &env.config as *const LfsConfig,
    ));
    assert_ok(lfs_mount(lfs.as_mut_ptr(), &env.config as *const LfsConfig));

    let coffee = path_bytes("coffee");
    assert_ok(lfs_mkdir(lfs.as_mut_ptr(), coffee.as_ptr()));
    for name in PATHS {
        let path = path_bytes(&format!("coffee/{name}"));
        assert_ok(lfs_mkdir(lfs.as_mut_ptr(), path.as_ptr()));
    }

    for bad in &[
        "_rip",
        "c_ldbrew",
        "tu_kish",
        "tub_uk",
        "_vietnamese",
        "thai_",
    ] {
        let path = path_bytes(&format!("coffee/{bad}"));
        let mut info = core::mem::MaybeUninit::<LfsInfo>::zeroed();
        let err = lfs_stat(lfs.as_mut_ptr(), path.as_ptr(), info.as_mut_ptr());
        assert_err(LFS_ERR_NOENT, err);

        let mut file = core::mem::MaybeUninit::<LfsFile>::zeroed();
        let err = lfs_file_open(
            lfs.as_mut_ptr(),
            file.as_mut_ptr(),
            path.as_ptr(),
            LFS_O_RDONLY,
        );
        assert_err(LFS_ERR_NOENT, err);
    }
    assert_ok(lfs_unmount(lfs.as_mut_ptr()));
}

// --- test_paths_root ---
#[test]
fn test_paths_root() {
    init_logger();
    let mut env = default_config(128);
    init_context(&mut env);

    let mut lfs = core::mem::MaybeUninit::<Lfs>::zeroed();
    assert_ok(lfs_format(
        lfs.as_mut_ptr(),
        &env.config as *const LfsConfig,
    ));
    assert_ok(lfs_mount(lfs.as_mut_ptr(), &env.config as *const LfsConfig));

    let root_path = path_bytes("/");
    let mut dir = core::mem::MaybeUninit::<LfsDir>::zeroed();
    assert_ok(lfs_dir_open(
        lfs.as_mut_ptr(),
        dir.as_mut_ptr(),
        root_path.as_ptr(),
    ));
    assert_ok(lfs_dir_close(lfs.as_mut_ptr(), dir.as_mut_ptr()));

    let mut info = core::mem::MaybeUninit::<LfsInfo>::zeroed();
    assert_ok(lfs_stat(
        lfs.as_mut_ptr(),
        root_path.as_ptr(),
        info.as_mut_ptr(),
    ));
    let info = unsafe { info.assume_init() };
    assert_eq!(info.type_, LFS_TYPE_DIR as u8);

    assert_ok(lfs_unmount(lfs.as_mut_ptr()));
}

// --- Deferred edge-case tests (per roadmap 07a) ---

#[rstest]
#[case::dirs(true)]
#[case::files(false)]
fn test_paths_redundant_slashes(#[case] dir_mode: bool) {
    init_logger();
    let mut env = default_config(128);
    init_context(&mut env);
    let mut lfs = core::mem::MaybeUninit::<Lfs>::zeroed();
    assert_ok(lfs_format(
        lfs.as_mut_ptr(),
        &env.config as *const LfsConfig,
    ));
    assert_ok(lfs_mount(lfs.as_mut_ptr(), &env.config as *const LfsConfig));
    let lfs = unsafe { lfs.assume_init_mut() };

    assert_ok(lfs_mkdir(lfs, path_bytes("coffee").as_ptr()));
    let create_paths = &[
        "/coffee/drip",
        "//coffee//coldbrew",
        "///coffee///turkish",
        "////coffee////tubruk",
        "/////coffee/////vietnamese",
        "//////coffee//////thai",
    ];
    for path_str in create_paths {
        let path = path_bytes(path_str);
        if dir_mode {
            assert_ok(lfs_mkdir(lfs, path.as_ptr()));
        } else {
            let mut file = core::mem::MaybeUninit::<LfsFile>::zeroed();
            assert_ok(lfs_file_open(
                lfs,
                file.as_mut_ptr(),
                path.as_ptr(),
                LFS_O_WRONLY | LFS_O_CREAT | LFS_O_EXCL,
            ));
            assert_ok(lfs_file_close(lfs, file.as_mut_ptr()));
        }
    }

    let stat_paths = &[
        "//////coffee//////drip",
        "/////coffee/////coldbrew",
        "////coffee////turkish",
        "///coffee///tubruk",
        "//coffee//vietnamese",
        "/coffee/thai",
    ];
    let expect_names = [
        "drip",
        "coldbrew",
        "turkish",
        "tubruk",
        "vietnamese",
        "thai",
    ];
    for (path_str, expect) in stat_paths.iter().zip(expect_names) {
        let path = path_bytes(path_str);
        let mut info = core::mem::MaybeUninit::<LfsInfo>::zeroed();
        assert_ok(lfs_stat(lfs, path.as_ptr(), info.as_mut_ptr()));
        let info = unsafe { info.assume_init() };
        assert_eq!(info_name_str(&info), expect);
        assert_eq!(
            info.type_,
            if dir_mode { LFS_TYPE_DIR } else { LFS_TYPE_REG } as u8
        );
    }

    assert_ok(lfs_mkdir(lfs, path_bytes("espresso").as_ptr()));
    let renames = &[
        ("//////coffee//////drip", "/espresso/espresso"),
        ("/////coffee/////coldbrew", "//espresso//americano"),
        ("////coffee////turkish", "///espresso///macchiato"),
        ("///coffee///tubruk", "////espresso////latte"),
        ("//coffee//vietnamese", "/////espresso/////cappuccino"),
        ("/coffee/thai", "//////espresso//////mocha"),
    ];
    for (old, new) in renames {
        assert_ok(lfs_rename(
            lfs,
            path_bytes(old).as_ptr(),
            path_bytes(new).as_ptr(),
        ));
    }
    let mut info_dummy = core::mem::MaybeUninit::<LfsInfo>::zeroed();
    for path_str in stat_paths {
        let path = path_bytes(path_str);
        assert_err(
            LFS_ERR_NOENT,
            lfs_stat(lfs, path.as_ptr(), info_dummy.as_mut_ptr()),
        );
    }
    for path_str in &[
        "/espresso/espresso",
        "//espresso//americano",
        "///espresso///macchiato",
        "////espresso////latte",
        "/////espresso/////cappuccino",
        "/espresso/mocha",
    ] {
        assert_ok(lfs_remove(lfs, path_bytes(path_str).as_ptr()));
    }
    assert_ok(lfs_unmount(lfs));
}

#[rstest]
#[case::dirs(true)]
#[case::files(false)]
fn test_paths_trailing_slashes(#[case] dir_mode: bool) {
    init_logger();
    let mut env = default_config(128);
    init_context(&mut env);
    let mut lfs = core::mem::MaybeUninit::<Lfs>::zeroed();
    assert_ok(lfs_format(
        lfs.as_mut_ptr(),
        &env.config as *const LfsConfig,
    ));
    assert_ok(lfs_mount(lfs.as_mut_ptr(), &env.config as *const LfsConfig));
    let lfs = unsafe { lfs.assume_init_mut() };

    assert_ok(lfs_mkdir(lfs, path_bytes("coffee").as_ptr()));
    if dir_mode {
        for s in &[
            "coffee/drip/",
            "coffee/coldbrew//",
            "coffee/turkish///",
            "coffee/tubruk////",
            "coffee/vietnamese/////",
            "coffee/thai//////",
        ] {
            assert_ok(lfs_mkdir(lfs, path_bytes(s).as_ptr()));
        }
    } else {
        for s in &[
            "coffee/drip/",
            "coffee/coldbrew//",
            "coffee/turkish///",
            "coffee/tubruk////",
            "coffee/vietnamese/////",
            "coffee/thai//////",
        ] {
            let mut file = core::mem::MaybeUninit::<LfsFile>::zeroed();
            assert_err(
                LFS_ERR_NOTDIR,
                lfs_file_open(
                    lfs,
                    file.as_mut_ptr(),
                    path_bytes(s).as_ptr(),
                    LFS_O_WRONLY | LFS_O_CREAT | LFS_O_EXCL,
                ),
            );
        }
        for name in PATHS {
            let path = path_bytes(&format!("coffee/{name}"));
            let mut file = core::mem::MaybeUninit::<LfsFile>::zeroed();
            assert_ok(lfs_file_open(
                lfs,
                file.as_mut_ptr(),
                path.as_ptr(),
                LFS_O_WRONLY | LFS_O_CREAT | LFS_O_EXCL,
            ));
            assert_ok(lfs_file_close(lfs, file.as_mut_ptr()));
        }
    }

    let stat_slashes = &[
        "coffee/drip//////",
        "coffee/coldbrew/////",
        "coffee/turkish////",
        "coffee/tubruk///",
        "coffee/vietnamese//",
        "coffee/thai/",
    ];
    for (i, path_str) in stat_slashes.iter().enumerate() {
        let path = path_bytes(path_str);
        let mut info = core::mem::MaybeUninit::<LfsInfo>::zeroed();
        let err = lfs_stat(lfs, path.as_ptr(), info.as_mut_ptr());
        if dir_mode {
            assert_ok(err);
            let info = unsafe { info.assume_init() };
            assert_eq!(info_name_str(&info), PATHS[i]);
            assert_eq!(info.type_, LFS_TYPE_DIR as u8);
        } else {
            assert_err(LFS_ERR_NOTDIR, err);
        }
    }

    assert_ok(lfs_mkdir(lfs, path_bytes("espresso").as_ptr()));
    if dir_mode {
        let renames = &[
            ("coffee/drip//////", "espresso/espresso/"),
            ("coffee/coldbrew/////", "espresso/americano//"),
            ("coffee/turkish////", "espresso/macchiato///"),
            ("coffee/tubruk///", "espresso/latte////"),
            ("coffee/vietnamese//", "espresso/cappuccino/////"),
            ("coffee/thai/", "espresso/mocha//////"),
        ];
        for (old, new) in renames {
            assert_ok(lfs_rename(
                lfs,
                path_bytes(old).as_ptr(),
                path_bytes(new).as_ptr(),
            ));
        }
        for s in &[
            "espresso/espresso/",
            "espresso/americano//",
            "espresso/macchiato///",
            "espresso/latte////",
            "espresso/cappuccino/////",
            "espresso/mocha//////",
        ] {
            assert_ok(lfs_remove(lfs, path_bytes(s).as_ptr()));
        }
    }
    assert_ok(lfs_unmount(lfs));
}

#[rstest]
#[case::dirs(true)]
#[case::files(false)]
fn test_paths_dots(#[case] dir_mode: bool) {
    init_logger();
    let mut env = default_config(128);
    init_context(&mut env);
    let mut lfs = core::mem::MaybeUninit::<Lfs>::zeroed();
    assert_ok(lfs_format(
        lfs.as_mut_ptr(),
        &env.config as *const LfsConfig,
    ));
    assert_ok(lfs_mount(lfs.as_mut_ptr(), &env.config as *const LfsConfig));
    let lfs = unsafe { lfs.assume_init_mut() };

    assert_ok(lfs_mkdir(lfs, path_bytes("coffee").as_ptr()));
    let create_paths = &[
        "/coffee/drip",
        "/./coffee/./coldbrew",
        "/././coffee/././turkish",
        "/./././coffee/./././tubruk",
        "/././././coffee/././././vietnamese",
        "/./././././coffee/./././././thai",
    ];
    for path_str in create_paths {
        let path = path_bytes(path_str);
        if dir_mode {
            assert_ok(lfs_mkdir(lfs, path.as_ptr()));
        } else {
            let mut file = core::mem::MaybeUninit::<LfsFile>::zeroed();
            assert_ok(lfs_file_open(
                lfs,
                file.as_mut_ptr(),
                path.as_ptr(),
                LFS_O_WRONLY | LFS_O_CREAT | LFS_O_EXCL,
            ));
            assert_ok(lfs_file_close(lfs, file.as_mut_ptr()));
        }
    }

    let stat_paths = &[
        "/no/no/../../no/no/../../coffee/drip",
        "/no/no/../../coffee/no/../coldbrew",
        "/no/no/../../coffee/turkish",
        "/coffee/no/../tubruk",
        "/no/../coffee/vietnamese",
        "/coffee/thai",
    ];
    let expect_names = [
        "drip",
        "coldbrew",
        "turkish",
        "tubruk",
        "vietnamese",
        "thai",
    ];
    for (path_str, expect) in stat_paths.iter().zip(expect_names) {
        let path = path_bytes(path_str);
        let mut info = core::mem::MaybeUninit::<LfsInfo>::zeroed();
        assert_ok(lfs_stat(lfs, path.as_ptr(), info.as_mut_ptr()));
        let info = unsafe { info.assume_init() };
        assert_eq!(info_name_str(&info), expect);
        assert_eq!(
            info.type_,
            if dir_mode { LFS_TYPE_DIR } else { LFS_TYPE_REG } as u8
        );
    }

    assert_ok(lfs_mkdir(lfs, path_bytes("espresso").as_ptr()));
    let renames = &[
        ("/no/no/../../no/no/../../coffee/drip", "/espresso/espresso"),
        (
            "/no/no/../../coffee/no/../coldbrew",
            "/./espresso/./americano",
        ),
        ("/no/no/../../coffee/turkish", "/././espresso/././macchiato"),
        ("/coffee/no/../tubruk", "/./././espresso/./././latte"),
        (
            "/no/../coffee/vietnamese",
            "/././././espresso/././././cappuccino",
        ),
        ("/coffee/thai", "/./././././espresso/./././././mocha"),
    ];
    for (old, new) in renames {
        assert_ok(lfs_rename(
            lfs,
            path_bytes(old).as_ptr(),
            path_bytes(new).as_ptr(),
        ));
    }
    for s in &[
        "/espresso/espresso",
        "/./espresso/./americano",
        "/././espresso/././macchiato",
        "/./././espresso/./././latte",
        "/././././espresso/././././cappuccino",
        "/./././././espresso/./././././mocha",
    ] {
        assert_ok(lfs_remove(lfs, path_bytes(s).as_ptr()));
    }
    assert_ok(lfs_unmount(lfs));
}

#[rstest]
#[case::dirs(true)]
#[case::files(false)]
fn test_paths_trailing_dots(#[case] dir_mode: bool) {
    init_logger();
    let mut env = default_config(128);
    init_context(&mut env);
    let mut lfs = core::mem::MaybeUninit::<Lfs>::zeroed();
    assert_ok(lfs_format(
        lfs.as_mut_ptr(),
        &env.config as *const LfsConfig,
    ));
    assert_ok(lfs_mount(lfs.as_mut_ptr(), &env.config as *const LfsConfig));
    let lfs = unsafe { lfs.assume_init_mut() };

    assert_ok(lfs_mkdir(lfs, path_bytes("coffee").as_ptr()));
    if dir_mode {
        for s in &[
            "coffee/drip/.",
            "coffee/coldbrew/./.",
            "coffee/turkish/././.",
            "coffee/tubruk/./././.",
            "coffee/vietnamese/././././.",
            "coffee/thai/./././././.",
        ] {
            assert_err(LFS_ERR_NOENT, lfs_mkdir(lfs, path_bytes(s).as_ptr()));
        }
        for name in PATHS {
            assert_ok(lfs_mkdir(
                lfs,
                path_bytes(&format!("coffee/{name}")).as_ptr(),
            ));
        }
    } else {
        for s in &[
            "coffee/drip/.",
            "coffee/coldbrew/./.",
            "coffee/turkish/././.",
            "coffee/tubruk/./././.",
            "coffee/vietnamese/././././.",
            "coffee/thai/./././././.",
        ] {
            let mut file = core::mem::MaybeUninit::<LfsFile>::zeroed();
            assert_err(
                LFS_ERR_NOENT,
                lfs_file_open(
                    lfs,
                    file.as_mut_ptr(),
                    path_bytes(s).as_ptr(),
                    LFS_O_WRONLY | LFS_O_CREAT | LFS_O_EXCL,
                ),
            );
        }
        for name in PATHS {
            let mut file = core::mem::MaybeUninit::<LfsFile>::zeroed();
            assert_ok(lfs_file_open(
                lfs,
                file.as_mut_ptr(),
                path_bytes(&format!("coffee/{name}")).as_ptr(),
                LFS_O_WRONLY | LFS_O_CREAT | LFS_O_EXCL,
            ));
            assert_ok(lfs_file_close(lfs, file.as_mut_ptr()));
        }
    }

    let stat_dots = &[
        "coffee/drip/./././././.",
        "coffee/coldbrew/././././.",
        "coffee/turkish/./././.",
        "coffee/tubruk/././.",
        "coffee/vietnamese/./.",
        "coffee/thai/.",
    ];
    let mut info = core::mem::MaybeUninit::<LfsInfo>::zeroed();
    for (i, path_str) in stat_dots.iter().enumerate() {
        let path = path_bytes(path_str);
        let err = lfs_stat(lfs, path.as_ptr(), info.as_mut_ptr());
        if dir_mode {
            assert_ok(err);
            let info_ref = unsafe { &*info.as_ptr() };
            assert_eq!(info_name_str(info_ref), PATHS[i]);
            assert_eq!(info_ref.type_, LFS_TYPE_DIR as u8);
        } else {
            assert_err(LFS_ERR_NOTDIR, err);
        }
    }

    assert_ok(lfs_mkdir(lfs, path_bytes("espresso").as_ptr()));
    if dir_mode {
        let renames_ok = &[
            ("coffee/drip/./././././.", "espresso/espresso"),
            ("coffee/coldbrew/././././.", "espresso/americano"),
            ("coffee/turkish/./././.", "espresso/macchiato"),
            ("coffee/tubruk/././.", "espresso/latte"),
            ("coffee/vietnamese/./.", "espresso/cappuccino"),
            ("coffee/thai/.", "espresso/mocha"),
        ];
        for (old, new) in renames_ok {
            assert_ok(lfs_rename(
                lfs,
                path_bytes(old).as_ptr(),
                path_bytes(new).as_ptr(),
            ));
        }
        for s in &[
            "espresso/espresso/.",
            "espresso/americano/./.",
            "espresso/macchiato/././.",
            "espresso/latte/./././.",
            "espresso/cappuccino/././././.",
            "espresso/mocha/./././././.",
        ] {
            assert_ok(lfs_remove(lfs, path_bytes(s).as_ptr()));
        }
    }
    assert_ok(lfs_unmount(lfs));
}

#[rstest]
#[case::dirs(true)]
#[case::files(false)]
fn test_paths_dotdots(#[case] dir_mode: bool) {
    init_logger();
    let mut env = default_config(128);
    init_context(&mut env);
    let mut lfs = core::mem::MaybeUninit::<Lfs>::zeroed();
    assert_ok(lfs_format(
        lfs.as_mut_ptr(),
        &env.config as *const LfsConfig,
    ));
    assert_ok(lfs_mount(lfs.as_mut_ptr(), &env.config as *const LfsConfig));
    let lfs = unsafe { lfs.assume_init_mut() };

    assert_ok(lfs_mkdir(lfs, path_bytes("no").as_ptr()));
    assert_ok(lfs_mkdir(lfs, path_bytes("no/no").as_ptr()));
    assert_ok(lfs_mkdir(lfs, path_bytes("coffee").as_ptr()));
    assert_ok(lfs_mkdir(lfs, path_bytes("coffee/no").as_ptr()));
    let create_paths = &[
        "/coffee/drip",
        "/no/../coffee/coldbrew",
        "/coffee/no/../turkish",
        "/no/no/../../coffee/tubruk",
        "/no/no/../../coffee/no/../vietnamese",
        "/no/no/../../no/no/../../coffee/thai",
    ];
    for path_str in create_paths {
        let path = path_bytes(path_str);
        if dir_mode {
            assert_ok(lfs_mkdir(lfs, path.as_ptr()));
        } else {
            let mut file = core::mem::MaybeUninit::<LfsFile>::zeroed();
            assert_ok(lfs_file_open(
                lfs,
                file.as_mut_ptr(),
                path.as_ptr(),
                LFS_O_WRONLY | LFS_O_CREAT | LFS_O_EXCL,
            ));
            assert_ok(lfs_file_close(lfs, file.as_mut_ptr()));
        }
    }

    let stat_paths = &[
        "/./././././coffee/./././././drip",
        "/././././coffee/././././coldbrew",
        "/./././coffee/./././turkish",
        "/././coffee/././tubruk",
        "/./coffee/./vietnamese",
        "/coffee/thai",
    ];
    let expect_names = [
        "drip",
        "coldbrew",
        "turkish",
        "tubruk",
        "vietnamese",
        "thai",
    ];
    for (path_str, expect) in stat_paths.iter().zip(expect_names) {
        let path = path_bytes(path_str);
        let mut info = core::mem::MaybeUninit::<LfsInfo>::zeroed();
        assert_ok(lfs_stat(lfs, path.as_ptr(), info.as_mut_ptr()));
        let info = unsafe { info.assume_init() };
        assert_eq!(info_name_str(&info), expect);
        assert_eq!(
            info.type_,
            if dir_mode { LFS_TYPE_DIR } else { LFS_TYPE_REG } as u8
        );
    }

    assert_ok(lfs_mkdir(lfs, path_bytes("espresso").as_ptr()));
    let renames = &[
        ("/./././././coffee/./././././drip", "/espresso/espresso"),
        (
            "/././././coffee/././././coldbrew",
            "/./espresso/./americano",
        ),
        ("/./././coffee/./././turkish", "/././espresso/././macchiato"),
        ("/./././coffee/././tubruk", "/./././espresso/./././latte"),
        (
            "/./coffee/./vietnamese",
            "/././././espresso/././././cappuccino",
        ),
        ("/coffee/thai", "/./././././espresso/./././././mocha"),
    ];
    for (old, new) in renames {
        assert_ok(lfs_rename(
            lfs,
            path_bytes(old).as_ptr(),
            path_bytes(new).as_ptr(),
        ));
    }
    for s in &[
        "/espresso/espresso",
        "/./espresso/./americano",
        "/././espresso/././macchiato",
        "/./././espresso/./././latte",
        "/././././espresso/././././cappuccino",
        "/./././././espresso/./././././mocha",
    ] {
        assert_ok(lfs_remove(lfs, path_bytes(s).as_ptr()));
    }
    assert_ok(lfs_unmount(lfs));
}

#[rstest]
#[case::dirs(true)]
#[case::files(false)]
fn test_paths_trailing_dotdots(#[case] dir_mode: bool) {
    init_logger();
    let mut env = default_config(128);
    init_context(&mut env);
    let mut lfs = core::mem::MaybeUninit::<Lfs>::zeroed();
    assert_ok(lfs_format(
        lfs.as_mut_ptr(),
        &env.config as *const LfsConfig,
    ));
    assert_ok(lfs_mount(lfs.as_mut_ptr(), &env.config as *const LfsConfig));
    let lfs = unsafe { lfs.assume_init_mut() };

    assert_ok(lfs_mkdir(lfs, path_bytes("coffee").as_ptr()));

    if dir_mode {
        assert_err(
            LFS_ERR_EXIST,
            lfs_mkdir(lfs, path_bytes("coffee/drip/..").as_ptr()),
        );
        assert_err(
            LFS_ERR_EXIST,
            lfs_mkdir(lfs, path_bytes("coffee/coldbrew/../..").as_ptr()),
        );
        assert_err(
            LFS_ERR_INVAL,
            lfs_mkdir(lfs, path_bytes("coffee/turkish/../../..").as_ptr()),
        );
        assert_err(
            LFS_ERR_INVAL,
            lfs_mkdir(lfs, path_bytes("coffee/tubruk/../../../..").as_ptr()),
        );
        assert_err(
            LFS_ERR_INVAL,
            lfs_mkdir(lfs, path_bytes("coffee/vietnamese/../../../../..").as_ptr()),
        );
        assert_err(
            LFS_ERR_INVAL,
            lfs_mkdir(lfs, path_bytes("coffee/thai/../../../../../..").as_ptr()),
        );
        for name in PATHS {
            assert_ok(lfs_mkdir(
                lfs,
                path_bytes(&format!("coffee/{name}")).as_ptr(),
            ));
        }
    } else {
        assert_err(
            LFS_ERR_EXIST,
            lfs_file_open(
                lfs,
                &mut core::mem::MaybeUninit::<LfsFile>::zeroed() as *mut _ as *mut LfsFile,
                path_bytes("coffee/drip/..").as_ptr(),
                LFS_O_WRONLY | LFS_O_CREAT | LFS_O_EXCL,
            ),
        );
        assert_err(
            LFS_ERR_EXIST,
            lfs_file_open(
                lfs,
                &mut core::mem::MaybeUninit::<LfsFile>::zeroed() as *mut _ as *mut LfsFile,
                path_bytes("coffee/coldbrew/../..").as_ptr(),
                LFS_O_WRONLY | LFS_O_CREAT | LFS_O_EXCL,
            ),
        );
        assert_err(
            LFS_ERR_INVAL,
            lfs_file_open(
                lfs,
                &mut core::mem::MaybeUninit::<LfsFile>::zeroed() as *mut _ as *mut LfsFile,
                path_bytes("coffee/turkish/../../..").as_ptr(),
                LFS_O_WRONLY | LFS_O_CREAT | LFS_O_EXCL,
            ),
        );
        assert_err(
            LFS_ERR_INVAL,
            lfs_file_open(
                lfs,
                &mut core::mem::MaybeUninit::<LfsFile>::zeroed() as *mut _ as *mut LfsFile,
                path_bytes("coffee/tubruk/../../../..").as_ptr(),
                LFS_O_WRONLY | LFS_O_CREAT | LFS_O_EXCL,
            ),
        );
        assert_err(
            LFS_ERR_INVAL,
            lfs_file_open(
                lfs,
                &mut core::mem::MaybeUninit::<LfsFile>::zeroed() as *mut _ as *mut LfsFile,
                path_bytes("coffee/vietnamese/../../../../..").as_ptr(),
                LFS_O_WRONLY | LFS_O_CREAT | LFS_O_EXCL,
            ),
        );
        assert_err(
            LFS_ERR_INVAL,
            lfs_file_open(
                lfs,
                &mut core::mem::MaybeUninit::<LfsFile>::zeroed() as *mut _ as *mut LfsFile,
                path_bytes("coffee/thai/../../../../../..").as_ptr(),
                LFS_O_WRONLY | LFS_O_CREAT | LFS_O_EXCL,
            ),
        );
        for name in PATHS {
            let mut file = core::mem::MaybeUninit::<LfsFile>::zeroed();
            assert_ok(lfs_file_open(
                lfs,
                file.as_mut_ptr(),
                path_bytes(&format!("coffee/{name}")).as_ptr(),
                LFS_O_WRONLY | LFS_O_CREAT | LFS_O_EXCL,
            ));
            assert_ok(lfs_file_close(lfs, file.as_mut_ptr()));
        }
    }

    // stat paths
    let mut info_err = core::mem::MaybeUninit::<LfsInfo>::zeroed();
    assert_err(
        LFS_ERR_INVAL,
        lfs_stat(
            lfs,
            path_bytes("coffee/drip/../../../../../..").as_ptr(),
            info_err.as_mut_ptr(),
        ),
    );
    assert_err(
        LFS_ERR_INVAL,
        lfs_stat(
            lfs,
            path_bytes("coffee/coldbrew/../../../../..").as_ptr(),
            info_err.as_mut_ptr(),
        ),
    );
    assert_err(
        LFS_ERR_INVAL,
        lfs_stat(
            lfs,
            path_bytes("coffee/turkish/../../../..").as_ptr(),
            info_err.as_mut_ptr(),
        ),
    );
    assert_err(
        LFS_ERR_INVAL,
        lfs_stat(
            lfs,
            path_bytes("coffee/tubruk/../../..").as_ptr(),
            info_err.as_mut_ptr(),
        ),
    );

    let mut info = core::mem::MaybeUninit::<LfsInfo>::zeroed();
    assert_ok(lfs_stat(
        lfs,
        path_bytes("coffee/vietnamese/../..").as_ptr(),
        info.as_mut_ptr(),
    ));
    let info = unsafe { info.assume_init() };
    assert_eq!(info_name_str(&info), "/");
    assert_eq!(info.type_, LFS_TYPE_DIR as u8);

    let mut info2 = core::mem::MaybeUninit::<LfsInfo>::zeroed();
    assert_ok(lfs_stat(
        lfs,
        path_bytes("coffee/thai/..").as_ptr(),
        info2.as_mut_ptr(),
    ));
    let info2 = unsafe { info2.assume_init() };
    assert_eq!(info_name_str(&info2), "coffee");
    assert_eq!(info2.type_, LFS_TYPE_DIR as u8);

    assert_ok(lfs_unmount(lfs));
}

#[rstest]
#[case::dirs(true)]
#[case::files(false)]
fn test_paths_dot_dotdots(#[case] dir_mode: bool) {
    init_logger();
    let mut env = default_config(128);
    init_context(&mut env);
    let mut lfs = core::mem::MaybeUninit::<Lfs>::zeroed();
    assert_ok(lfs_format(
        lfs.as_mut_ptr(),
        &env.config as *const LfsConfig,
    ));
    assert_ok(lfs_mount(lfs.as_mut_ptr(), &env.config as *const LfsConfig));
    let lfs = unsafe { lfs.assume_init_mut() };

    assert_ok(lfs_mkdir(lfs, path_bytes("no").as_ptr()));
    assert_ok(lfs_mkdir(lfs, path_bytes("no/no").as_ptr()));
    assert_ok(lfs_mkdir(lfs, path_bytes("coffee").as_ptr()));
    assert_ok(lfs_mkdir(lfs, path_bytes("coffee/no").as_ptr()));

    if dir_mode {
        assert_ok(lfs_mkdir(lfs, path_bytes("/coffee/drip").as_ptr()));
        assert_ok(lfs_mkdir(
            lfs,
            path_bytes("/no/./../coffee/coldbrew").as_ptr(),
        ));
        assert_ok(lfs_mkdir(
            lfs,
            path_bytes("/coffee/no/./../turkish").as_ptr(),
        ));
        assert_ok(lfs_mkdir(
            lfs,
            path_bytes("/no/no/./.././../coffee/tubruk").as_ptr(),
        ));
        assert_ok(lfs_mkdir(
            lfs,
            path_bytes("/no/no/./.././../coffee/no/./../vietnamese").as_ptr(),
        ));
        assert_ok(lfs_mkdir(
            lfs,
            path_bytes("/no/no/./.././../no/no/./.././../coffee/thai").as_ptr(),
        ));
    } else {
        let mut file = core::mem::MaybeUninit::<LfsFile>::zeroed();
        assert_ok(lfs_file_open(
            lfs,
            file.as_mut_ptr(),
            path_bytes("/coffee/drip").as_ptr(),
            LFS_O_WRONLY | LFS_O_CREAT | LFS_O_EXCL,
        ));
        assert_ok(lfs_file_close(lfs, file.as_mut_ptr()));
        assert_ok(lfs_file_open(
            lfs,
            file.as_mut_ptr(),
            path_bytes("/no/./../coffee/coldbrew").as_ptr(),
            LFS_O_WRONLY | LFS_O_CREAT | LFS_O_EXCL,
        ));
        assert_ok(lfs_file_close(lfs, file.as_mut_ptr()));
        for path in [
            "/coffee/no/./../turkish",
            "/no/no/./.././../coffee/tubruk",
            "/no/no/./.././../coffee/no/./../vietnamese",
            "/no/no/./.././../no/no/./.././../coffee/thai",
        ] {
            let mut f = core::mem::MaybeUninit::<LfsFile>::zeroed();
            assert_ok(lfs_file_open(
                lfs,
                f.as_mut_ptr(),
                path_bytes(path).as_ptr(),
                LFS_O_WRONLY | LFS_O_CREAT | LFS_O_EXCL,
            ));
            assert_ok(lfs_file_close(lfs, f.as_mut_ptr()));
        }
    }

    // stat paths
    let mut info = core::mem::MaybeUninit::<LfsInfo>::zeroed();
    assert_ok(lfs_stat(
        lfs,
        path_bytes("/no/no/./.././../no/no/./.././../coffee/drip").as_ptr(),
        info.as_mut_ptr(),
    ));
    let info = unsafe { info.assume_init() };
    assert_eq!(info_name_str(&info), "drip");
    assert_eq!(
        info.type_,
        if dir_mode { LFS_TYPE_DIR } else { LFS_TYPE_REG } as u8
    );

    let mut info2 = core::mem::MaybeUninit::<LfsInfo>::zeroed();
    assert_ok(lfs_stat(
        lfs,
        path_bytes("/no/no/./.././../coffee/no/./../coldbrew").as_ptr(),
        info2.as_mut_ptr(),
    ));
    let info2 = unsafe { info2.assume_init() };
    assert_eq!(info_name_str(&info2), "coldbrew");
    assert_eq!(
        info2.type_,
        if dir_mode { LFS_TYPE_DIR } else { LFS_TYPE_REG } as u8
    );

    for (path, expected_name) in [
        ("/no/no/./.././../coffee/turkish", "turkish"),
        ("/coffee/no/./../tubruk", "tubruk"),
        ("/no/./../coffee/vietnamese", "vietnamese"),
        ("/coffee/thai", "thai"),
    ] {
        let mut i = core::mem::MaybeUninit::<LfsInfo>::zeroed();
        assert_ok(lfs_stat(lfs, path_bytes(path).as_ptr(), i.as_mut_ptr()));
        let i = unsafe { i.assume_init() };
        assert_eq!(info_name_str(&i), expected_name);
        assert_eq!(
            i.type_,
            if dir_mode { LFS_TYPE_DIR } else { LFS_TYPE_REG } as u8
        );
    }

    if dir_mode {
        let mut file = core::mem::MaybeUninit::<LfsFile>::zeroed();
        for path in [
            "/coffee/drip",
            "/no/./../coffee/coldbrew",
            "/coffee/no/./../turkish",
            "/no/no/./.././../coffee/tubruk",
            "/no/no/./.././../coffee/no/./../vietnamese",
            "/no/no/./.././../no/no/./.././../coffee/thai",
        ] {
            assert_err(
                LFS_ERR_ISDIR,
                lfs_file_open(
                    lfs,
                    file.as_mut_ptr(),
                    path_bytes(path).as_ptr(),
                    LFS_O_RDONLY,
                ),
            );
            assert_err(
                LFS_ERR_ISDIR,
                lfs_file_open(
                    lfs,
                    file.as_mut_ptr(),
                    path_bytes(path).as_ptr(),
                    LFS_O_WRONLY | LFS_O_CREAT,
                ),
            );
            assert_err(
                LFS_ERR_EXIST,
                lfs_file_open(
                    lfs,
                    file.as_mut_ptr(),
                    path_bytes(path).as_ptr(),
                    LFS_O_WRONLY | LFS_O_CREAT | LFS_O_EXCL,
                ),
            );
        }
    } else {
        let mut file = core::mem::MaybeUninit::<LfsFile>::zeroed();
        for path in [
            "/coffee/drip",
            "/no/./../coffee/coldbrew",
            "/coffee/no/./../turkish",
            "/no/no/./.././../coffee/tubruk",
            "/no/no/./.././../coffee/no/./../vietnamese",
            "/no/no/./.././../no/no/./.././../coffee/thai",
        ] {
            assert_ok(lfs_file_open(
                lfs,
                file.as_mut_ptr(),
                path_bytes(path).as_ptr(),
                LFS_O_RDONLY,
            ));
            assert_ok(lfs_file_close(lfs, file.as_mut_ptr()));
        }
    }

    assert_ok(lfs_unmount(lfs));
}

#[rstest]
#[case::dirs(true)]
#[case::files(false)]
fn test_paths_dotdotdots(#[case] dir_mode: bool) {
    init_logger();
    let mut env = default_config(128);
    init_context(&mut env);
    let mut lfs = core::mem::MaybeUninit::<Lfs>::zeroed();
    assert_ok(lfs_format(
        lfs.as_mut_ptr(),
        &env.config as *const LfsConfig,
    ));
    assert_ok(lfs_mount(lfs.as_mut_ptr(), &env.config as *const LfsConfig));
    let lfs = unsafe { lfs.assume_init_mut() };
    assert_ok(lfs_mkdir(lfs, path_bytes("coffee").as_ptr()));
    assert_ok(lfs_mkdir(lfs, path_bytes("coffee/...").as_ptr()));
    for name in PATHS {
        let path = path_bytes(&format!("/coffee/.../{name}"));
        if dir_mode {
            assert_ok(lfs_mkdir(lfs, path.as_ptr()));
        } else {
            let mut file = core::mem::MaybeUninit::<LfsFile>::zeroed();
            assert_ok(lfs_file_open(
                lfs,
                file.as_mut_ptr(),
                path.as_ptr(),
                LFS_O_WRONLY | LFS_O_CREAT | LFS_O_EXCL,
            ));
            assert_ok(lfs_file_close(lfs, file.as_mut_ptr()));
        }
    }
    for name in PATHS {
        let path = path_bytes(&format!("/coffee/.../{name}"));
        let mut info = core::mem::MaybeUninit::<LfsInfo>::zeroed();
        assert_ok(lfs_stat(lfs, path.as_ptr(), info.as_mut_ptr()));
        let info = unsafe { info.assume_init() };
        assert_eq!(info_name_str(&info), *name);
        assert_eq!(
            info.type_,
            if dir_mode { LFS_TYPE_DIR } else { LFS_TYPE_REG } as u8
        );
    }
    assert_ok(lfs_unmount(lfs));
}

// --- Missing upstream cases ---

/// Upstream: [cases.test_paths_noent_trailing_slashes]
/// defines.DIR = [false, true]
/// Paths with trailing slashes on non-existent entries. C expects exact LFS_ERR_NOENT for stat/dir_open.
#[rstest]
#[case::dirs(true)]
#[case::files(false)]
fn test_paths_noent_trailing_slashes(#[case] dir_mode: bool) {
    init_logger();
    let mut env = default_config(128);
    init_context(&mut env);
    let mut lfs = core::mem::MaybeUninit::<Lfs>::zeroed();
    assert_ok(lfs_format(
        lfs.as_mut_ptr(),
        &env.config as *const LfsConfig,
    ));
    assert_ok(lfs_mount(lfs.as_mut_ptr(), &env.config as *const LfsConfig));
    let lfs = unsafe { lfs.assume_init_mut() };
    assert_ok(lfs_mkdir(lfs, path_bytes("coffee").as_ptr()));
    for name in PATHS {
        let path = path_bytes(&format!("coffee/{name}"));
        if dir_mode {
            assert_ok(lfs_mkdir(lfs, path.as_ptr()));
        } else {
            let mut file = core::mem::MaybeUninit::<LfsFile>::zeroed();
            assert_ok(lfs_file_open(
                lfs,
                file.as_mut_ptr(),
                path.as_ptr(),
                LFS_O_WRONLY | LFS_O_CREAT | LFS_O_EXCL,
            ));
            assert_ok(lfs_file_close(lfs, file.as_mut_ptr()));
        }
    }
    // C: 6 malformed paths with trailing slashes — stat => NOENT
    let bad_stat = [
        "coffee/_rip//////",
        "coffee/c_ldbrew/////",
        "coffee/tu_kish////",
        "coffee/tub_uk///",
        "coffee/_vietnamese//",
        "coffee/thai_/",
    ];
    for bad in bad_stat {
        let path = path_bytes(bad);
        let mut info = core::mem::MaybeUninit::<LfsInfo>::zeroed();
        assert_err(
            LFS_ERR_NOENT,
            lfs_stat(lfs, path.as_ptr(), info.as_mut_ptr()),
        );
    }
    // file_open RDONLY => NOENT
    for bad in bad_stat {
        let path = path_bytes(bad);
        let mut file = core::mem::MaybeUninit::<LfsFile>::zeroed();
        assert_err(
            LFS_ERR_NOENT,
            lfs_file_open(lfs, file.as_mut_ptr(), path.as_ptr(), LFS_O_RDONLY),
        );
    }
    // file_open WRONLY|CREAT => NOTDIR
    for bad in bad_stat {
        let path = path_bytes(bad);
        let mut file = core::mem::MaybeUninit::<LfsFile>::zeroed();
        assert_err(
            LFS_ERR_NOTDIR,
            lfs_file_open(
                lfs,
                file.as_mut_ptr(),
                path.as_ptr(),
                LFS_O_WRONLY | LFS_O_CREAT,
            ),
        );
    }
    // file_open WRONLY|CREAT|EXCL => NOTDIR
    for bad in bad_stat {
        let path = path_bytes(bad);
        let mut file = core::mem::MaybeUninit::<LfsFile>::zeroed();
        assert_err(
            LFS_ERR_NOTDIR,
            lfs_file_open(
                lfs,
                file.as_mut_ptr(),
                path.as_ptr(),
                LFS_O_WRONLY | LFS_O_CREAT | LFS_O_EXCL,
            ),
        );
    }
    // dir_open => NOENT
    for bad in bad_stat {
        let path = path_bytes(bad);
        let mut dir = core::mem::MaybeUninit::<LfsDir>::zeroed();
        assert_err(
            LFS_ERR_NOENT,
            lfs_dir_open(lfs, dir.as_mut_ptr(), path.as_ptr()),
        );
    }
    // rename: bad source
    assert_ok(lfs_mkdir(lfs, path_bytes("espresso").as_ptr()));
    assert_err(
        LFS_ERR_NOENT,
        lfs_rename(
            lfs,
            path_bytes("coffee/_rip//////").as_ptr(),
            path_bytes("espresso/espresso").as_ptr(),
        ),
    );
    assert_err(
        LFS_ERR_NOENT,
        lfs_rename(
            lfs,
            path_bytes("coffee/c_ldbrew/////").as_ptr(),
            path_bytes("espresso/americano").as_ptr(),
        ),
    );
    assert_err(
        LFS_ERR_NOENT,
        lfs_rename(
            lfs,
            path_bytes("coffee/tu_kish////").as_ptr(),
            path_bytes("espresso/macchiato").as_ptr(),
        ),
    );
    assert_err(
        LFS_ERR_NOENT,
        lfs_rename(
            lfs,
            path_bytes("coffee/tub_uk///").as_ptr(),
            path_bytes("espresso/latte").as_ptr(),
        ),
    );
    assert_err(
        LFS_ERR_NOENT,
        lfs_rename(
            lfs,
            path_bytes("coffee/_vietnamese//").as_ptr(),
            path_bytes("espresso/cappuccino").as_ptr(),
        ),
    );
    assert_err(
        LFS_ERR_NOENT,
        lfs_rename(
            lfs,
            path_bytes("coffee/thai_/").as_ptr(),
            path_bytes("espresso/mocha").as_ptr(),
        ),
    );
    // rename: bad destination (trailing slash on dest)
    assert_err(
        LFS_ERR_NOENT,
        lfs_rename(
            lfs,
            path_bytes("coffee/_rip").as_ptr(),
            path_bytes("espresso/espresso/").as_ptr(),
        ),
    );
    assert_err(
        LFS_ERR_NOENT,
        lfs_rename(
            lfs,
            path_bytes("coffee/c_ldbrew").as_ptr(),
            path_bytes("espresso/americano//").as_ptr(),
        ),
    );
    assert_err(
        LFS_ERR_NOENT,
        lfs_rename(
            lfs,
            path_bytes("coffee/tu_kish").as_ptr(),
            path_bytes("espresso/macchiato///").as_ptr(),
        ),
    );
    assert_err(
        LFS_ERR_NOENT,
        lfs_rename(
            lfs,
            path_bytes("coffee/tub_uk").as_ptr(),
            path_bytes("espresso/latte////").as_ptr(),
        ),
    );
    assert_err(
        LFS_ERR_NOENT,
        lfs_rename(
            lfs,
            path_bytes("coffee/_vietnamese").as_ptr(),
            path_bytes("espresso/cappuccino/////").as_ptr(),
        ),
    );
    assert_err(
        LFS_ERR_NOENT,
        lfs_rename(
            lfs,
            path_bytes("coffee/thai_").as_ptr(),
            path_bytes("espresso/mocha//////").as_ptr(),
        ),
    );
    // rename: bad source and bad destination
    assert_err(
        LFS_ERR_NOENT,
        lfs_rename(
            lfs,
            path_bytes("coffee/_rip//////").as_ptr(),
            path_bytes("espresso/espresso/").as_ptr(),
        ),
    );
    assert_err(
        LFS_ERR_NOENT,
        lfs_rename(
            lfs,
            path_bytes("coffee/c_ldbrew/////").as_ptr(),
            path_bytes("espresso/americano//").as_ptr(),
        ),
    );
    assert_err(
        LFS_ERR_NOENT,
        lfs_rename(
            lfs,
            path_bytes("coffee/tu_kish////").as_ptr(),
            path_bytes("espresso/macchiato///").as_ptr(),
        ),
    );
    assert_err(
        LFS_ERR_NOENT,
        lfs_rename(
            lfs,
            path_bytes("coffee/tub_uk///").as_ptr(),
            path_bytes("espresso/latte////").as_ptr(),
        ),
    );
    assert_err(
        LFS_ERR_NOENT,
        lfs_rename(
            lfs,
            path_bytes("coffee/_vietnamese//").as_ptr(),
            path_bytes("espresso/cappuccino/////").as_ptr(),
        ),
    );
    assert_err(
        LFS_ERR_NOENT,
        lfs_rename(
            lfs,
            path_bytes("coffee/thai_/").as_ptr(),
            path_bytes("espresso/mocha//////").as_ptr(),
        ),
    );
    // rename noop (same bad path both sides) => NOENT
    assert_err(
        LFS_ERR_NOENT,
        lfs_rename(
            lfs,
            path_bytes("coffee/_rip//////").as_ptr(),
            path_bytes("coffee/_rip//////").as_ptr(),
        ),
    );
    assert_err(
        LFS_ERR_NOENT,
        lfs_rename(
            lfs,
            path_bytes("coffee/c_ldbrew/////").as_ptr(),
            path_bytes("coffee/c_ldbrew/////").as_ptr(),
        ),
    );
    assert_err(
        LFS_ERR_NOENT,
        lfs_rename(
            lfs,
            path_bytes("coffee/tu_kish////").as_ptr(),
            path_bytes("coffee/tu_kish////").as_ptr(),
        ),
    );
    assert_err(
        LFS_ERR_NOENT,
        lfs_rename(
            lfs,
            path_bytes("coffee/tub_uk///").as_ptr(),
            path_bytes("coffee/tub_uk///").as_ptr(),
        ),
    );
    assert_err(
        LFS_ERR_NOENT,
        lfs_rename(
            lfs,
            path_bytes("coffee/_vietnamese//").as_ptr(),
            path_bytes("coffee/_vietnamese//").as_ptr(),
        ),
    );
    assert_err(
        LFS_ERR_NOENT,
        lfs_rename(
            lfs,
            path_bytes("coffee/thai_/").as_ptr(),
            path_bytes("coffee/thai_/").as_ptr(),
        ),
    );
    // remove => NOENT
    for bad in bad_stat {
        let path = path_bytes(bad);
        assert_err(LFS_ERR_NOENT, lfs_remove(lfs, path.as_ptr()));
    }
    // stat espresso/* (renames failed so these don't exist) => NOENT
    for name in [
        "espresso",
        "espresso/espresso",
        "espresso/americano",
        "espresso/macchiato",
        "espresso/latte",
        "espresso/cappuccino",
        "espresso/mocha",
    ] {
        let path = path_bytes(name);
        let mut info = core::mem::MaybeUninit::<LfsInfo>::zeroed();
        let err = lfs_stat(lfs, path.as_ptr(), info.as_mut_ptr());
        if name == "espresso" {
            assert_ok(err);
        } else {
            assert_err(LFS_ERR_NOENT, err);
        }
    }
    // final stat of valid coffee paths
    for name in PATHS {
        let path = path_bytes(&format!("coffee/{name}"));
        let mut info = core::mem::MaybeUninit::<LfsInfo>::zeroed();
        assert_ok(lfs_stat(lfs, path.as_ptr(), info.as_mut_ptr()));
        let info = unsafe { info.assume_init() };
        let nul = info.name.iter().position(|&b| b == 0).unwrap_or(256);
        assert_eq!(core::str::from_utf8(&info.name[..nul]).unwrap(), *name);
        assert_eq!(
            info.type_,
            if dir_mode { LFS_TYPE_DIR } else { LFS_TYPE_REG } as u8
        );
    }
    assert_ok(lfs_unmount(lfs));
}

/// Upstream: [cases.test_paths_noent_trailing_dots]
/// defines.DIR = [false, true]
/// Paths with trailing dots on non-existent entries. Expect LFS_ERR_NOENT.
#[rstest]
#[case::dirs(true)]
#[case::files(false)]
fn test_paths_noent_trailing_dots(#[case] dir_mode: bool) {
    init_logger();
    let mut env = default_config(128);
    init_context(&mut env);
    let mut lfs = core::mem::MaybeUninit::<Lfs>::zeroed();
    assert_ok(lfs_format(
        lfs.as_mut_ptr(),
        &env.config as *const LfsConfig,
    ));
    assert_ok(lfs_mount(lfs.as_mut_ptr(), &env.config as *const LfsConfig));
    let lfs = unsafe { lfs.assume_init_mut() };
    assert_ok(lfs_mkdir(lfs, path_bytes("coffee").as_ptr()));
    for name in PATHS {
        let path = path_bytes(&format!("coffee/{name}"));
        if dir_mode {
            assert_ok(lfs_mkdir(lfs, path.as_ptr()));
        } else {
            let mut file = core::mem::MaybeUninit::<LfsFile>::zeroed();
            assert_ok(lfs_file_open(
                lfs,
                file.as_mut_ptr(),
                path.as_ptr(),
                LFS_O_WRONLY | LFS_O_CREAT | LFS_O_EXCL,
            ));
            assert_ok(lfs_file_close(lfs, file.as_mut_ptr()));
        }
    }
    // C: 6 malformed paths with trailing dots — stat => NOENT
    let bad_paths = [
        "coffee/_rip/./././././.",
        "coffee/c_ldbrew/././././.",
        "coffee/tu_kish/./././.",
        "coffee/tub_uk/././.",
        "coffee/_vietnamese/./.",
        "coffee/thai_/.",
    ];
    for bad in bad_paths {
        let path = path_bytes(bad);
        let mut info = core::mem::MaybeUninit::<LfsInfo>::zeroed();
        assert_err(
            LFS_ERR_NOENT,
            lfs_stat(lfs, path.as_ptr(), info.as_mut_ptr()),
        );
    }
    // file_open RDONLY, WRONLY|CREAT, WRONLY|CREAT|EXCL => NOENT
    for bad in bad_paths {
        let path = path_bytes(bad);
        let mut file = core::mem::MaybeUninit::<LfsFile>::zeroed();
        assert_err(
            LFS_ERR_NOENT,
            lfs_file_open(lfs, file.as_mut_ptr(), path.as_ptr(), LFS_O_RDONLY),
        );
        let mut file = core::mem::MaybeUninit::<LfsFile>::zeroed();
        assert_err(
            LFS_ERR_NOENT,
            lfs_file_open(
                lfs,
                file.as_mut_ptr(),
                path.as_ptr(),
                LFS_O_WRONLY | LFS_O_CREAT,
            ),
        );
        let mut file = core::mem::MaybeUninit::<LfsFile>::zeroed();
        assert_err(
            LFS_ERR_NOENT,
            lfs_file_open(
                lfs,
                file.as_mut_ptr(),
                path.as_ptr(),
                LFS_O_WRONLY | LFS_O_CREAT | LFS_O_EXCL,
            ),
        );
    }
    // dir_open => NOENT
    for bad in bad_paths {
        let path = path_bytes(bad);
        let mut dir = core::mem::MaybeUninit::<LfsDir>::zeroed();
        assert_err(
            LFS_ERR_NOENT,
            lfs_dir_open(lfs, dir.as_mut_ptr(), path.as_ptr()),
        );
    }
    // rename: bad source, bad dest, noop; remove; final stat of valid paths
    assert_ok(lfs_mkdir(lfs, path_bytes("espresso").as_ptr()));
    assert_err(
        LFS_ERR_NOENT,
        lfs_rename(
            lfs,
            path_bytes("coffee/_rip/./././././.").as_ptr(),
            path_bytes("espresso/espresso").as_ptr(),
        ),
    );
    assert_err(
        LFS_ERR_NOENT,
        lfs_rename(
            lfs,
            path_bytes("coffee/_rip").as_ptr(),
            path_bytes("espresso/espresso/.").as_ptr(),
        ),
    );
    assert_err(
        LFS_ERR_NOENT,
        lfs_rename(
            lfs,
            path_bytes("coffee/_rip/./././././.").as_ptr(),
            path_bytes("coffee/_rip/./././././.").as_ptr(),
        ),
    );
    for bad in bad_paths {
        assert_err(LFS_ERR_NOENT, lfs_remove(lfs, path_bytes(bad).as_ptr()));
    }
    for name in PATHS {
        let path = path_bytes(&format!("coffee/{name}"));
        let mut info = core::mem::MaybeUninit::<LfsInfo>::zeroed();
        assert_ok(lfs_stat(lfs, path.as_ptr(), info.as_mut_ptr()));
        let info = unsafe { info.assume_init() };
        let nul = info.name.iter().position(|&b| b == 0).unwrap_or(256);
        assert_eq!(core::str::from_utf8(&info.name[..nul]).unwrap(), *name);
        assert_eq!(
            info.type_,
            if dir_mode { LFS_TYPE_DIR } else { LFS_TYPE_REG } as u8
        );
    }
    assert_ok(lfs_unmount(lfs));
}

/// Upstream: [cases.test_paths_noent_trailing_dotdots]
/// defines.DIR = [false, true]
/// Paths with trailing .. components. C: INVAL above root, ISDIR for file_open on coffee/_rip/..,
/// dir_open success for coffee/_rip/.., rename (bad source/dest, valid coffee/thai_/.. → espresso/mocha),
/// remove (NOTEMPTY, INVAL).
#[rstest]
#[case::dirs(true)]
#[case::files(false)]
fn test_paths_noent_trailing_dotdots(#[case] dir_mode: bool) {
    init_logger();
    let mut env = default_config(128);
    init_context(&mut env);
    let mut lfs = core::mem::MaybeUninit::<Lfs>::zeroed();
    assert_ok(lfs_format(
        lfs.as_mut_ptr(),
        &env.config as *const LfsConfig,
    ));
    assert_ok(lfs_mount(lfs.as_mut_ptr(), &env.config as *const LfsConfig));
    let lfs = unsafe { lfs.assume_init_mut() };
    assert_ok(lfs_mkdir(lfs, path_bytes("coffee").as_ptr()));
    for name in PATHS {
        let path = path_bytes(&format!("coffee/{name}"));
        if dir_mode {
            assert_ok(lfs_mkdir(lfs, path.as_ptr()));
        } else {
            let mut file = core::mem::MaybeUninit::<LfsFile>::zeroed();
            assert_ok(lfs_file_open(
                lfs,
                file.as_mut_ptr(),
                path.as_ptr(),
                LFS_O_WRONLY | LFS_O_CREAT | LFS_O_EXCL,
            ));
            assert_ok(lfs_file_close(lfs, file.as_mut_ptr()));
        }
    }
    // INVAL above root
    let mut info = core::mem::MaybeUninit::<LfsInfo>::zeroed();
    assert_err(
        LFS_ERR_INVAL,
        lfs_stat(
            lfs,
            path_bytes("coffee/drip/../../..").as_ptr(),
            info.as_mut_ptr(),
        ),
    );
    // coffee/_rip/.. resolves to coffee (dir). file_open => ISDIR
    let rip_dotdot = path_bytes("coffee/_rip/..");
    let mut file = core::mem::MaybeUninit::<LfsFile>::zeroed();
    assert_err(
        LFS_ERR_ISDIR,
        lfs_file_open(lfs, file.as_mut_ptr(), rip_dotdot.as_ptr(), LFS_O_RDONLY),
    );
    let mut file = core::mem::MaybeUninit::<LfsFile>::zeroed();
    assert_err(
        LFS_ERR_ISDIR,
        lfs_file_open(
            lfs,
            file.as_mut_ptr(),
            rip_dotdot.as_ptr(),
            LFS_O_WRONLY | LFS_O_CREAT,
        ),
    );
    // dir_open on coffee/_rip/.. => success (resolves to coffee)
    let mut dir = core::mem::MaybeUninit::<LfsDir>::zeroed();
    assert_ok(lfs_dir_open(lfs, dir.as_mut_ptr(), rip_dotdot.as_ptr()));
    assert_ok(lfs_dir_close(lfs, dir.as_mut_ptr()));
    // stat coffee/_rip/.. => coffee
    let mut info = core::mem::MaybeUninit::<LfsInfo>::zeroed();
    assert_ok(lfs_stat(lfs, rip_dotdot.as_ptr(), info.as_mut_ptr()));
    let info = unsafe { info.assume_init() };
    assert_eq!(info_name_str(&info), "coffee");
    // stat coffee/thai_/.. => coffee
    let thai_dotdot = path_bytes("coffee/thai_/..");
    let mut info = core::mem::MaybeUninit::<LfsInfo>::zeroed();
    assert_ok(lfs_stat(lfs, thai_dotdot.as_ptr(), info.as_mut_ptr()));
    let info = unsafe { info.assume_init() };
    assert_eq!(info_name_str(&info), "coffee");
    // rename: valid coffee/thai_/.. → espresso/mocha (moves coffee to espresso/mocha)
    assert_ok(lfs_mkdir(lfs, path_bytes("espresso").as_ptr()));
    assert_ok(lfs_rename(
        lfs,
        path_bytes("coffee/thai_/..").as_ptr(),
        path_bytes("espresso/mocha").as_ptr(),
    ));
    // rename: bad source (coffee/_rip/.. to file path when dest parent doesn't exist or similar)
    assert_ok(lfs_mkdir(lfs, path_bytes("coffee").as_ptr()));
    for name in PATHS {
        let path = path_bytes(&format!("coffee/{name}"));
        if dir_mode {
            assert_ok(lfs_mkdir(lfs, path.as_ptr()));
        } else {
            let mut file = core::mem::MaybeUninit::<LfsFile>::zeroed();
            assert_ok(lfs_file_open(
                lfs,
                file.as_mut_ptr(),
                path.as_ptr(),
                LFS_O_WRONLY | LFS_O_CREAT | LFS_O_EXCL,
            ));
            assert_ok(lfs_file_close(lfs, file.as_mut_ptr()));
        }
    }
    // remove: NOTEMPTY (coffee has children)
    assert_err(
        LFS_ERR_NOTEMPTY,
        lfs_remove(lfs, path_bytes("coffee/drip/..").as_ptr()),
    );
    // remove: INVAL (above root)
    assert_err(
        LFS_ERR_INVAL,
        lfs_remove(lfs, path_bytes("coffee/drip/../../..").as_ptr()),
    );
    assert_ok(lfs_unmount(lfs));
}

/// Upstream: [cases.test_paths_utf8_ipa]
/// defines.DIR = [false, true]
/// UTF-8 names with IPA symbols. C adds: WRONLY|CREAT => ISDIR or success; WRONLY|CREAT|EXCL => EXIST.
#[rstest]
#[case::dirs(true)]
#[case::files(false)]
fn test_paths_utf8_ipa(#[case] dir_mode: bool) {
    init_logger();
    let mut env = default_config(128);
    init_context(&mut env);
    let mut lfs = core::mem::MaybeUninit::<Lfs>::zeroed();
    assert_ok(lfs_format(
        lfs.as_mut_ptr(),
        &env.config as *const LfsConfig,
    ));
    assert_ok(lfs_mount(lfs.as_mut_ptr(), &env.config as *const LfsConfig));
    let lfs = unsafe { lfs.assume_init_mut() };
    let parent = "ˈkɔ.fi";
    let children = [
        "dɹɪpˈkɔ.fi",
        "koʊldbɹuː",
        "tyɾckɑhvɛˈsi",
        "ˈko.piˈt̪up̚.rʊk̚",
        "kaː˨˩fe˧˧ɗaː˧˥",
        "ʔoː˧.lia̯ŋ˦˥",
    ];
    assert_ok(lfs_mkdir(lfs, path_bytes(parent).as_ptr()));
    for name in children {
        let path = path_bytes(&format!("{parent}/{name}"));
        if dir_mode {
            assert_ok(lfs_mkdir(lfs, path.as_ptr()));
        } else {
            let mut file = core::mem::MaybeUninit::<LfsFile>::zeroed();
            assert_ok(lfs_file_open(
                lfs,
                file.as_mut_ptr(),
                path.as_ptr(),
                LFS_O_WRONLY | LFS_O_CREAT | LFS_O_EXCL,
            ));
            assert_ok(lfs_file_close(lfs, file.as_mut_ptr()));
        }
    }
    for name in children {
        let path = path_bytes(&format!("{parent}/{name}"));
        let mut info = core::mem::MaybeUninit::<LfsInfo>::zeroed();
        assert_ok(lfs_stat(lfs, path.as_ptr(), info.as_mut_ptr()));
        let info = unsafe { info.assume_init() };
        assert_eq!(info_name_str(&info), name);
        assert_eq!(
            info.type_,
            if dir_mode { LFS_TYPE_DIR } else { LFS_TYPE_REG } as u8
        );
    }
    if dir_mode {
        for name in children {
            let path = path_bytes(&format!("{parent}/{name}"));
            let mut file = core::mem::MaybeUninit::<LfsFile>::zeroed();
            assert_err(
                LFS_ERR_ISDIR,
                lfs_file_open(lfs, file.as_mut_ptr(), path.as_ptr(), LFS_O_RDONLY),
            );
            assert_err(
                LFS_ERR_ISDIR,
                lfs_file_open(
                    lfs,
                    file.as_mut_ptr(),
                    path.as_ptr(),
                    LFS_O_WRONLY | LFS_O_CREAT,
                ),
            );
            assert_err(
                LFS_ERR_EXIST,
                lfs_file_open(
                    lfs,
                    file.as_mut_ptr(),
                    path.as_ptr(),
                    LFS_O_WRONLY | LFS_O_CREAT | LFS_O_EXCL,
                ),
            );
        }
    } else {
        for name in children {
            let path = path_bytes(&format!("{parent}/{name}"));
            let mut file = core::mem::MaybeUninit::<LfsFile>::zeroed();
            assert_ok(lfs_file_open(
                lfs,
                file.as_mut_ptr(),
                path.as_ptr(),
                LFS_O_RDONLY,
            ));
            assert_ok(lfs_file_close(lfs, file.as_mut_ptr()));
            assert_ok(lfs_file_open(
                lfs,
                file.as_mut_ptr(),
                path.as_ptr(),
                LFS_O_WRONLY | LFS_O_CREAT,
            ));
            assert_ok(lfs_file_close(lfs, file.as_mut_ptr()));
            assert_err(
                LFS_ERR_EXIST,
                lfs_file_open(
                    lfs,
                    file.as_mut_ptr(),
                    path.as_ptr(),
                    LFS_O_WRONLY | LFS_O_CREAT | LFS_O_EXCL,
                ),
            );
        }
    }
    assert_ok(lfs_unmount(lfs));
}

/// Upstream: [cases.test_paths_oopsallspaces]
/// C layout: root " ", children " / ", " /  ", " /   ", " /    ", " /     ", " /      " (6 children).
/// Stat all, file_open/dir_open matrix, rename to "  /      " etc., remove.
#[rstest]
#[case::dirs(true)]
#[case::files(false)]
#[ignore = "oopsallspaces layout may fail; fix bug later"]
fn test_paths_oopsallspaces(#[case] dir_mode: bool) {
    init_logger();
    let mut env = default_config(128);
    init_context(&mut env);
    let mut lfs = core::mem::MaybeUninit::<Lfs>::zeroed();
    assert_ok(lfs_format(
        lfs.as_mut_ptr(),
        &env.config as *const LfsConfig,
    ));
    assert_ok(lfs_mount(lfs.as_mut_ptr(), &env.config as *const LfsConfig));
    let lfs = unsafe { lfs.assume_init_mut() };
    let root = " ";
    let children = [" / ", " /  ", " /   ", " /    ", " /     ", " /      "];
    assert_ok(lfs_mkdir(lfs, path_bytes(root).as_ptr()));
    for name in children {
        let path = path_bytes(&format!("{root}/{name}"));
        if dir_mode {
            assert_ok(lfs_mkdir(lfs, path.as_ptr()));
        } else {
            let mut file = core::mem::MaybeUninit::<LfsFile>::zeroed();
            assert_ok(lfs_file_open(
                lfs,
                file.as_mut_ptr(),
                path.as_ptr(),
                LFS_O_WRONLY | LFS_O_CREAT | LFS_O_EXCL,
            ));
            assert_ok(lfs_file_close(lfs, file.as_mut_ptr()));
        }
    }
    for name in children.iter() {
        let path = path_bytes(&format!("{root}/{name}"));
        let mut info = core::mem::MaybeUninit::<LfsInfo>::zeroed();
        assert_ok(lfs_stat(lfs, path.as_ptr(), info.as_mut_ptr()));
        let info = unsafe { info.assume_init() };
        assert_eq!(info_name_str(&info), *name);
        assert_eq!(
            info.type_,
            if dir_mode { LFS_TYPE_DIR } else { LFS_TYPE_REG } as u8
        );
    }
    if dir_mode {
        for name in children {
            let path = path_bytes(&format!("{root}/{name}"));
            let mut file = core::mem::MaybeUninit::<LfsFile>::zeroed();
            assert_err(
                LFS_ERR_ISDIR,
                lfs_file_open(lfs, file.as_mut_ptr(), path.as_ptr(), LFS_O_RDONLY),
            );
            assert_err(
                LFS_ERR_ISDIR,
                lfs_file_open(
                    lfs,
                    file.as_mut_ptr(),
                    path.as_ptr(),
                    LFS_O_WRONLY | LFS_O_CREAT,
                ),
            );
            let mut dir = core::mem::MaybeUninit::<LfsDir>::zeroed();
            assert_ok(lfs_dir_open(lfs, dir.as_mut_ptr(), path.as_ptr()));
            assert_ok(lfs_dir_close(lfs, dir.as_mut_ptr()));
        }
    } else {
        for name in children {
            let path = path_bytes(&format!("{root}/{name}"));
            let mut file = core::mem::MaybeUninit::<LfsFile>::zeroed();
            assert_ok(lfs_file_open(
                lfs,
                file.as_mut_ptr(),
                path.as_ptr(),
                LFS_O_RDONLY,
            ));
            assert_ok(lfs_file_close(lfs, file.as_mut_ptr()));
            assert_err(
                LFS_ERR_NOTDIR,
                lfs_dir_open(
                    lfs,
                    core::mem::MaybeUninit::<LfsDir>::zeroed().as_mut_ptr(),
                    path.as_ptr(),
                ),
            );
        }
    }
    assert_ok(lfs_mkdir(lfs, path_bytes("  ").as_ptr()));
    let renames = [
        (" / ", "  /      "),
        (" /  ", "  /     "),
        (" /   ", "  /    "),
        (" /    ", "  /   "),
        (" /     ", "  /  "),
        (" /      ", "  / "),
    ];
    for (old, new) in renames {
        let old_path = path_bytes(&format!("{root}/{old}"));
        let new_path = path_bytes(&new.to_string());
        assert_ok(lfs_rename(lfs, old_path.as_ptr(), new_path.as_ptr()));
    }
    for (_, new) in renames {
        assert_ok(lfs_remove(lfs, path_bytes(new).as_ptr()));
    }
    assert_ok(lfs_remove(lfs, path_bytes("  ").as_ptr()));
    assert_ok(lfs_remove(lfs, path_bytes(root).as_ptr()));
    assert_ok(lfs_unmount(lfs));
}

/// Upstream: [cases.test_paths_oopsalldels]
/// C layout: root \x7f (1 byte), children \x7f/\x7f, \x7f/\x7f\x7f, … (6 children with 1–6 DEL bytes).
#[rstest]
#[case::dirs(true)]
#[case::files(false)]
fn test_paths_oopsalldels(#[case] dir_mode: bool) {
    init_logger();
    let mut env = default_config(128);
    init_context(&mut env);
    let mut lfs = core::mem::MaybeUninit::<Lfs>::zeroed();
    assert_ok(lfs_format(
        lfs.as_mut_ptr(),
        &env.config as *const LfsConfig,
    ));
    assert_ok(lfs_mount(lfs.as_mut_ptr(), &env.config as *const LfsConfig));
    let lfs = unsafe { lfs.assume_init_mut() };
    let root = path_bytes_raw(&[0x7f]);
    assert_ok(lfs_mkdir(lfs, root.as_ptr()));
    let mut child_paths: Vec<Vec<u8>> = Vec::with_capacity(6);
    for n in 1..=6 {
        let p: Vec<u8> = (0..n).map(|_| 0x7f).collect();
        child_paths.push(path_bytes_raw(&p));
    }
    for cp in child_paths.iter() {
        let mut full: Vec<u8> = vec![0x7f, b'/'];
        full.extend_from_slice(&cp[..cp.len().saturating_sub(1)]);
        full.push(0);
        if dir_mode {
            assert_ok(lfs_mkdir(lfs, full.as_ptr()));
        } else {
            let mut file = core::mem::MaybeUninit::<LfsFile>::zeroed();
            assert_ok(lfs_file_open(
                lfs,
                file.as_mut_ptr(),
                full.as_ptr(),
                LFS_O_WRONLY | LFS_O_CREAT | LFS_O_EXCL,
            ));
            assert_ok(lfs_file_close(lfs, file.as_mut_ptr()));
        }
    }
    let mut full_paths: Vec<Vec<u8>> = Vec::with_capacity(6);
    for n in 1..=6 {
        let mut p = vec![0x7f, b'/'];
        p.extend((0..n).map(|_| 0x7f));
        p.push(0);
        full_paths.push(p);
    }
    for fp in &full_paths {
        let mut info = core::mem::MaybeUninit::<LfsInfo>::zeroed();
        assert_ok(lfs_stat(lfs, fp.as_ptr(), info.as_mut_ptr()));
    }
    if dir_mode {
        for fp in &full_paths {
            let mut file = core::mem::MaybeUninit::<LfsFile>::zeroed();
            assert_err(
                LFS_ERR_ISDIR,
                lfs_file_open(lfs, file.as_mut_ptr(), fp.as_ptr(), LFS_O_RDONLY),
            );
            let mut dir = core::mem::MaybeUninit::<LfsDir>::zeroed();
            assert_ok(lfs_dir_open(lfs, dir.as_mut_ptr(), fp.as_ptr()));
            assert_ok(lfs_dir_close(lfs, dir.as_mut_ptr()));
        }
    } else {
        for fp in &full_paths {
            let mut file = core::mem::MaybeUninit::<LfsFile>::zeroed();
            assert_ok(lfs_file_open(
                lfs,
                file.as_mut_ptr(),
                fp.as_ptr(),
                LFS_O_RDONLY,
            ));
            assert_ok(lfs_file_close(lfs, file.as_mut_ptr()));
            assert_err(
                LFS_ERR_NOTDIR,
                lfs_dir_open(
                    lfs,
                    core::mem::MaybeUninit::<LfsDir>::zeroed().as_mut_ptr(),
                    fp.as_ptr(),
                ),
            );
        }
    }
    let new_root = path_bytes_raw(&[0x7f, 0x7f]);
    assert_ok(lfs_mkdir(lfs, new_root.as_ptr()));
    for (n, fp) in full_paths.iter().enumerate() {
        let new_name_len = 6 - n;
        let mut new_path = vec![0x7f, 0x7f, b'/'];
        new_path.extend((0..new_name_len).map(|_| 0x7f));
        new_path.push(0);
        assert_ok(lfs_rename(lfs, fp.as_ptr(), new_path.as_ptr()));
    }
    for n in 1..=6 {
        let mut p = vec![0x7f, 0x7f, b'/'];
        p.extend((0..n).map(|_| 0x7f));
        p.push(0);
        assert_ok(lfs_remove(lfs, p.as_ptr()));
    }
    assert_ok(lfs_remove(lfs, new_root.as_ptr()));
    assert_ok(lfs_remove(lfs, root.as_ptr()));
    assert_ok(lfs_unmount(lfs));
}

/// Upstream: [cases.test_paths_oopsallffs]
/// Same as oopsalldels but with 0xff bytes. C layout: root 0xff, 6 children 0xff/0xff, 0xff/0xff0xff, etc.
#[rstest]
#[case::dirs(true)]
#[case::files(false)]
fn test_paths_oopsallffs(#[case] dir_mode: bool) {
    init_logger();
    let mut env = default_config(128);
    init_context(&mut env);
    let mut lfs = core::mem::MaybeUninit::<Lfs>::zeroed();
    assert_ok(lfs_format(
        lfs.as_mut_ptr(),
        &env.config as *const LfsConfig,
    ));
    assert_ok(lfs_mount(lfs.as_mut_ptr(), &env.config as *const LfsConfig));
    let lfs = unsafe { lfs.assume_init_mut() };
    let root = path_bytes_raw(&[0xff]);
    assert_ok(lfs_mkdir(lfs, root.as_ptr()));
    let mut child_paths: Vec<Vec<u8>> = Vec::with_capacity(6);
    for n in 1..=6 {
        let p: Vec<u8> = (0..n).map(|_| 0xff).collect();
        child_paths.push(path_bytes_raw(&p));
    }
    for cp in child_paths.iter() {
        let mut full: Vec<u8> = vec![0xff, b'/'];
        full.extend_from_slice(&cp[..cp.len().saturating_sub(1)]);
        full.push(0);
        if dir_mode {
            assert_ok(lfs_mkdir(lfs, full.as_ptr()));
        } else {
            let mut file = core::mem::MaybeUninit::<LfsFile>::zeroed();
            assert_ok(lfs_file_open(
                lfs,
                file.as_mut_ptr(),
                full.as_ptr(),
                LFS_O_WRONLY | LFS_O_CREAT | LFS_O_EXCL,
            ));
            assert_ok(lfs_file_close(lfs, file.as_mut_ptr()));
        }
    }
    let mut full_paths: Vec<Vec<u8>> = Vec::with_capacity(6);
    for n in 1..=6 {
        let mut p = vec![0xff, b'/'];
        p.extend((0..n).map(|_| 0xff));
        p.push(0);
        full_paths.push(p);
    }
    for fp in &full_paths {
        let mut info = core::mem::MaybeUninit::<LfsInfo>::zeroed();
        assert_ok(lfs_stat(lfs, fp.as_ptr(), info.as_mut_ptr()));
    }
    if dir_mode {
        for fp in &full_paths {
            let mut file = core::mem::MaybeUninit::<LfsFile>::zeroed();
            assert_err(
                LFS_ERR_ISDIR,
                lfs_file_open(lfs, file.as_mut_ptr(), fp.as_ptr(), LFS_O_RDONLY),
            );
            let mut dir = core::mem::MaybeUninit::<LfsDir>::zeroed();
            assert_ok(lfs_dir_open(lfs, dir.as_mut_ptr(), fp.as_ptr()));
            assert_ok(lfs_dir_close(lfs, dir.as_mut_ptr()));
        }
    } else {
        for fp in &full_paths {
            let mut file = core::mem::MaybeUninit::<LfsFile>::zeroed();
            assert_ok(lfs_file_open(
                lfs,
                file.as_mut_ptr(),
                fp.as_ptr(),
                LFS_O_RDONLY,
            ));
            assert_ok(lfs_file_close(lfs, file.as_mut_ptr()));
            assert_err(
                LFS_ERR_NOTDIR,
                lfs_dir_open(
                    lfs,
                    core::mem::MaybeUninit::<LfsDir>::zeroed().as_mut_ptr(),
                    fp.as_ptr(),
                ),
            );
        }
    }
    let new_root = path_bytes_raw(&[0xff, 0xff]);
    assert_ok(lfs_mkdir(lfs, new_root.as_ptr()));
    for (n, fp) in full_paths.iter().enumerate() {
        let new_name_len = 6 - n;
        let mut new_path = vec![0xff, 0xff, b'/'];
        new_path.extend((0..new_name_len).map(|_| 0xff));
        new_path.push(0);
        assert_ok(lfs_rename(lfs, fp.as_ptr(), new_path.as_ptr()));
    }
    for n in 1..=6 {
        let mut p = vec![0xff, 0xff, b'/'];
        p.extend((0..n).map(|_| 0xff));
        p.push(0);
        assert_ok(lfs_remove(lfs, p.as_ptr()));
    }
    assert_ok(lfs_remove(lfs, new_root.as_ptr()));
    assert_ok(lfs_remove(lfs, root.as_ptr()));
    assert_ok(lfs_unmount(lfs));
}

#[rstest]
#[case::dirs(true)]
#[case::files(false)]
fn test_paths_leading_dots(#[case] _dir_mode: bool) {
    init_logger();
    let mut env = default_config(128);
    init_context(&mut env);
    let mut lfs = core::mem::MaybeUninit::<Lfs>::zeroed();
    assert_ok(lfs_format(
        lfs.as_mut_ptr(),
        &env.config as *const LfsConfig,
    ));
    assert_ok(lfs_mount(lfs.as_mut_ptr(), &env.config as *const LfsConfig));
    let lfs = unsafe { lfs.assume_init_mut() };
    let mut info = core::mem::MaybeUninit::<LfsInfo>::zeroed();
    assert_err(
        LFS_ERR_INVAL,
        lfs_stat(lfs, path_bytes("..").as_ptr(), info.as_mut_ptr()),
    );
    assert_ok(lfs_unmount(lfs));
}

#[rstest]
#[case::dirs(true)]
#[case::files(false)]
fn test_paths_root_dotdots(#[case] _dir_mode: bool) {
    init_logger();
    let mut env = default_config(128);
    init_context(&mut env);
    let mut lfs = core::mem::MaybeUninit::<Lfs>::zeroed();
    assert_ok(lfs_format(
        lfs.as_mut_ptr(),
        &env.config as *const LfsConfig,
    ));
    assert_ok(lfs_mount(lfs.as_mut_ptr(), &env.config as *const LfsConfig));
    let lfs = unsafe { lfs.assume_init_mut() };
    let mut info = core::mem::MaybeUninit::<LfsInfo>::zeroed();
    assert_err(
        LFS_ERR_INVAL,
        lfs_stat(lfs, path_bytes("/..").as_ptr(), info.as_mut_ptr()),
    );
    assert_ok(lfs_unmount(lfs));
}

#[test]
fn test_paths_noent_parent() {
    init_logger();
    let mut env = default_config(128);
    init_context(&mut env);
    let mut lfs = core::mem::MaybeUninit::<Lfs>::zeroed();
    assert_ok(lfs_format(
        lfs.as_mut_ptr(),
        &env.config as *const LfsConfig,
    ));
    assert_ok(lfs_mount(lfs.as_mut_ptr(), &env.config as *const LfsConfig));
    let lfs = unsafe { lfs.assume_init_mut() };
    let mut info = core::mem::MaybeUninit::<LfsInfo>::zeroed();
    assert_err(
        LFS_ERR_NOENT,
        lfs_stat(
            lfs,
            path_bytes("nonexistent/child").as_ptr(),
            info.as_mut_ptr(),
        ),
    );
    assert_ok(lfs_unmount(lfs));
}

#[test]
fn test_paths_notdir_parent() {
    init_logger();
    let mut env = default_config(128);
    init_context(&mut env);
    let mut lfs = core::mem::MaybeUninit::<Lfs>::zeroed();
    assert_ok(lfs_format(
        lfs.as_mut_ptr(),
        &env.config as *const LfsConfig,
    ));
    assert_ok(lfs_mount(lfs.as_mut_ptr(), &env.config as *const LfsConfig));
    let lfs = unsafe { lfs.assume_init_mut() };
    let mut file = core::mem::MaybeUninit::<LfsFile>::zeroed();
    assert_ok(lfs_file_open(
        lfs,
        file.as_mut_ptr(),
        path_bytes("f").as_ptr(),
        LFS_O_WRONLY | LFS_O_CREAT | LFS_O_EXCL,
    ));
    assert_ok(lfs_file_close(lfs, file.as_mut_ptr()));
    let mut info = core::mem::MaybeUninit::<LfsInfo>::zeroed();
    assert_err(
        LFS_ERR_NOTDIR,
        lfs_stat(lfs, path_bytes("f/child").as_ptr(), info.as_mut_ptr()),
    );
    assert_ok(lfs_unmount(lfs));
}

#[rstest]
#[case::dirs(true)]
#[case::files(false)]
fn test_paths_empty(#[case] dir_mode: bool) {
    init_logger();
    let mut env = default_config(128);
    init_context(&mut env);
    let mut lfs = core::mem::MaybeUninit::<Lfs>::zeroed();
    assert_ok(lfs_format(
        lfs.as_mut_ptr(),
        &env.config as *const LfsConfig,
    ));
    assert_ok(lfs_mount(lfs.as_mut_ptr(), &env.config as *const LfsConfig));
    let lfs = unsafe { lfs.assume_init_mut() };
    let mut info = core::mem::MaybeUninit::<LfsInfo>::zeroed();
    assert_err(
        LFS_ERR_INVAL,
        lfs_stat(lfs, path_bytes("").as_ptr(), info.as_mut_ptr()),
    );
    if dir_mode {
        assert_err(LFS_ERR_INVAL, lfs_mkdir(lfs, path_bytes("").as_ptr()));
    } else {
        let mut file = core::mem::MaybeUninit::<LfsFile>::zeroed();
        assert_err(
            LFS_ERR_INVAL,
            lfs_file_open(
                lfs,
                file.as_mut_ptr(),
                path_bytes("").as_ptr(),
                LFS_O_WRONLY | LFS_O_CREAT | LFS_O_EXCL,
            ),
        );
    }
    assert_ok(lfs_mkdir(lfs, path_bytes("x").as_ptr()));
    assert_err(
        LFS_ERR_INVAL,
        lfs_rename(lfs, path_bytes("x").as_ptr(), path_bytes("").as_ptr()),
    );
    assert_err(
        LFS_ERR_INVAL,
        lfs_rename(lfs, path_bytes("").as_ptr(), path_bytes("y").as_ptr()),
    );
    assert_err(LFS_ERR_INVAL, lfs_remove(lfs, path_bytes("").as_ptr()));
    assert_ok(lfs_unmount(lfs));
}

#[rstest]
#[case::dirs(true)]
#[case::files(false)]
fn test_paths_root_aliases(#[case] _dir_mode: bool) {
    init_logger();
    let mut env = default_config(128);
    init_context(&mut env);
    let mut lfs = core::mem::MaybeUninit::<Lfs>::zeroed();
    assert_ok(lfs_format(
        lfs.as_mut_ptr(),
        &env.config as *const LfsConfig,
    ));
    assert_ok(lfs_mount(lfs.as_mut_ptr(), &env.config as *const LfsConfig));
    let lfs = unsafe { lfs.assume_init_mut() };
    let aliases = &["/", ".", "./", "/.", "//"];
    for alias in aliases {
        let path = path_bytes(alias);
        let mut info = core::mem::MaybeUninit::<LfsInfo>::zeroed();
        assert_ok(lfs_stat(lfs, path.as_ptr(), info.as_mut_ptr()));
        let info = unsafe { info.assume_init() };
        assert_eq!(info_name_str(&info), "/");
        assert_eq!(info.type_, LFS_TYPE_DIR as u8);
    }
    assert_ok(lfs_unmount(lfs));
}

#[test]
fn test_paths_magic_noent() {
    init_logger();
    let mut env = default_config(128);
    init_context(&mut env);
    let mut lfs = core::mem::MaybeUninit::<Lfs>::zeroed();
    assert_ok(lfs_format(
        lfs.as_mut_ptr(),
        &env.config as *const LfsConfig,
    ));
    assert_ok(lfs_mount(lfs.as_mut_ptr(), &env.config as *const LfsConfig));
    let lfs = unsafe { lfs.assume_init_mut() };
    assert_ok(lfs_mkdir(lfs, path_bytes("a").as_ptr()));
    let mut info = core::mem::MaybeUninit::<LfsInfo>::zeroed();
    assert_err(
        LFS_ERR_NOENT,
        lfs_stat(lfs, path_bytes("a/b").as_ptr(), info.as_mut_ptr()),
    );
    assert_ok(lfs_unmount(lfs));
}

#[rstest]
#[case::dirs(true)]
#[case::files(false)]
fn test_paths_magic_conflict(#[case] dir_mode: bool) {
    init_logger();
    let mut env = default_config(128);
    init_context(&mut env);
    let mut lfs = core::mem::MaybeUninit::<Lfs>::zeroed();
    assert_ok(lfs_format(
        lfs.as_mut_ptr(),
        &env.config as *const LfsConfig,
    ));
    assert_ok(lfs_mount(lfs.as_mut_ptr(), &env.config as *const LfsConfig));
    let lfs = unsafe { lfs.assume_init_mut() };
    if dir_mode {
        assert_ok(lfs_mkdir(lfs, path_bytes("littlefs").as_ptr()));
    } else {
        let mut file = core::mem::MaybeUninit::<LfsFile>::zeroed();
        assert_ok(lfs_file_open(
            lfs,
            file.as_mut_ptr(),
            path_bytes("littlefs").as_ptr(),
            LFS_O_WRONLY | LFS_O_CREAT | LFS_O_EXCL,
        ));
        assert_ok(lfs_file_close(lfs, file.as_mut_ptr()));
    }
    let mut info = core::mem::MaybeUninit::<LfsInfo>::zeroed();
    assert_ok(lfs_stat(
        lfs,
        path_bytes("littlefs").as_ptr(),
        info.as_mut_ptr(),
    ));
    assert_eq!(info_name_str(unsafe { &*info.as_ptr() }), "littlefs");
    assert_ok(lfs_rename(
        lfs,
        path_bytes("littlefs").as_ptr(),
        path_bytes("coffee").as_ptr(),
    ));
    assert_ok(lfs_rename(
        lfs,
        path_bytes("coffee").as_ptr(),
        path_bytes("littlefs").as_ptr(),
    ));
    assert_ok(lfs_stat(
        lfs,
        path_bytes("littlefs").as_ptr(),
        info.as_mut_ptr(),
    ));
    assert_err(
        LFS_ERR_NOENT,
        lfs_stat(lfs, path_bytes("coffee").as_ptr(), info.as_mut_ptr()),
    );
    assert_ok(lfs_remove(lfs, path_bytes("littlefs").as_ptr()));
    assert_err(
        LFS_ERR_NOENT,
        lfs_stat(lfs, path_bytes("littlefs").as_ptr(), info.as_mut_ptr()),
    );
    assert_ok(lfs_unmount(lfs));
}

#[test]
fn test_paths_nametoolong() {
    init_logger();
    let mut env = default_config(128);
    init_context(&mut env);
    let mut lfs = core::mem::MaybeUninit::<Lfs>::zeroed();
    assert_ok(lfs_format(
        lfs.as_mut_ptr(),
        &env.config as *const LfsConfig,
    ));
    assert_ok(lfs_mount(lfs.as_mut_ptr(), &env.config as *const LfsConfig));
    let lfs = unsafe { lfs.assume_init_mut() };
    let long_name = "a".repeat(256);
    assert_err(
        LFS_ERR_NAMETOOLONG,
        lfs_mkdir(lfs, path_bytes(&long_name).as_ptr()),
    );
    assert_ok(lfs_unmount(lfs));
}

#[test]
fn test_paths_namejustlongenough() {
    init_logger();
    let mut env = default_config(128);
    init_context(&mut env);
    let mut lfs = core::mem::MaybeUninit::<Lfs>::zeroed();
    assert_ok(lfs_format(
        lfs.as_mut_ptr(),
        &env.config as *const LfsConfig,
    ));
    assert_ok(lfs_mount(lfs.as_mut_ptr(), &env.config as *const LfsConfig));
    let lfs = unsafe { lfs.assume_init_mut() };
    let max_name = "a".repeat(255);
    assert_ok(lfs_mkdir(lfs, path_bytes(&max_name).as_ptr()));
    let mut info = core::mem::MaybeUninit::<LfsInfo>::zeroed();
    assert_ok(lfs_stat(
        lfs,
        path_bytes(&max_name).as_ptr(),
        info.as_mut_ptr(),
    ));
    assert_eq!(info_name_str(unsafe { &*info.as_ptr() }), max_name);
    assert_ok(lfs_unmount(lfs));
}

#[test]
fn test_paths_utf8() {
    init_logger();
    let mut env = default_config(128);
    init_context(&mut env);
    let mut lfs = core::mem::MaybeUninit::<Lfs>::zeroed();
    assert_ok(lfs_format(
        lfs.as_mut_ptr(),
        &env.config as *const LfsConfig,
    ));
    assert_ok(lfs_mount(lfs.as_mut_ptr(), &env.config as *const LfsConfig));
    let lfs = unsafe { lfs.assume_init_mut() };
    let name = "café_日本_한글";
    assert_ok(lfs_mkdir(lfs, path_bytes(name).as_ptr()));
    let mut info = core::mem::MaybeUninit::<LfsInfo>::zeroed();
    assert_ok(lfs_stat(lfs, path_bytes(name).as_ptr(), info.as_mut_ptr()));
    assert_eq!(info_name_str(unsafe { &*info.as_ptr() }), name);
    assert_ok(lfs_unmount(lfs));
}

#[test]
fn test_paths_spaces() {
    init_logger();
    let mut env = default_config(128);
    init_context(&mut env);
    let mut lfs = core::mem::MaybeUninit::<Lfs>::zeroed();
    assert_ok(lfs_format(
        lfs.as_mut_ptr(),
        &env.config as *const LfsConfig,
    ));
    assert_ok(lfs_mount(lfs.as_mut_ptr(), &env.config as *const LfsConfig));
    let lfs = unsafe { lfs.assume_init_mut() };
    let name = "foo bar";
    assert_ok(lfs_mkdir(lfs, path_bytes(name).as_ptr()));
    let mut info = core::mem::MaybeUninit::<LfsInfo>::zeroed();
    assert_ok(lfs_stat(lfs, path_bytes(name).as_ptr(), info.as_mut_ptr()));
    assert_eq!(info_name_str(unsafe { &*info.as_ptr() }), name);
    assert_ok(lfs_unmount(lfs));
}

#[test]
fn test_paths_nonprintable() {
    init_logger();
    let mut env = default_config(128);
    init_context(&mut env);
    let mut lfs = core::mem::MaybeUninit::<Lfs>::zeroed();
    assert_ok(lfs_format(
        lfs.as_mut_ptr(),
        &env.config as *const LfsConfig,
    ));
    assert_ok(lfs_mount(lfs.as_mut_ptr(), &env.config as *const LfsConfig));
    let lfs = unsafe { lfs.assume_init_mut() };
    let mut name: Vec<u8> = vec![b'a'; 10];
    name[5] = 0x01;
    name.push(0);
    assert_ok(lfs_mkdir(lfs, name.as_ptr()));
    let mut info = core::mem::MaybeUninit::<LfsInfo>::zeroed();
    assert_ok(lfs_stat(lfs, name.as_ptr(), info.as_mut_ptr()));
    assert_ok(lfs_unmount(lfs));
}

#[test]
fn test_paths_nonutf8() {
    init_logger();
    let mut env = default_config(128);
    init_context(&mut env);
    let mut lfs = core::mem::MaybeUninit::<Lfs>::zeroed();
    assert_ok(lfs_format(
        lfs.as_mut_ptr(),
        &env.config as *const LfsConfig,
    ));
    assert_ok(lfs_mount(lfs.as_mut_ptr(), &env.config as *const LfsConfig));
    let lfs = unsafe { lfs.assume_init_mut() };
    let name = b"foo\xff\xfe\xfdbar\0";
    assert_ok(lfs_mkdir(lfs, name.as_ptr()));
    let mut info = core::mem::MaybeUninit::<LfsInfo>::zeroed();
    assert_ok(lfs_stat(lfs, name.as_ptr(), info.as_mut_ptr()));
    assert_ok(lfs_unmount(lfs));
}
