//! C (littlefs2-sys) helper for alignment tests.

use std::ffi::{CStr, CString};
use std::os::raw::c_int;

use lp_littlefs::Config;

use crate::storage::AlignStorage;

const LFS_O_WRONLY: c_int = 2;
const LFS_O_CREAT: c_int = 0x0100;
const LFS_O_EXCL: c_int = 0x0200;
const LFS_ERR_EXIST: c_int = -17;

/// Format using C littlefs, then unmount (format only).
pub fn format(storage: &AlignStorage, config: &Config) -> Result<(), c_int> {
    let mut lfs = unsafe { std::mem::zeroed::<littlefs2_sys::lfs_t>() };
    let lfs_config = storage.build_lfs_config(config, None, None, None);

    let res = unsafe { littlefs2_sys::lfs_format(&mut lfs, &lfs_config) };
    if res != 0 {
        return Err(res);
    }

    let res = unsafe { littlefs2_sys::lfs_mount(&mut lfs, &lfs_config) };
    if res != 0 {
        return Err(res);
    }

    let res = unsafe { littlefs2_sys::lfs_unmount(&mut lfs) };
    if res != 0 {
        return Err(res);
    }

    Ok(())
}

/// Format, mount, file_open(CREAT|EXCL), mkdir, file_close, unmount.
/// Reverse order: file first, then dir (tests creation order independence).
pub fn format_file_mkdir_unmount(
    storage: &AlignStorage,
    config: &Config,
    file_name: &str,
    dir_name: &str,
) -> Result<(), c_int> {
    let mut lfs = unsafe { std::mem::zeroed::<littlefs2_sys::lfs_t>() };
    let lfs_config = storage.build_lfs_config(config, None, None, None);

    let res = unsafe { littlefs2_sys::lfs_format(&mut lfs, &lfs_config) };
    if res != 0 {
        return Err(res);
    }

    let res = unsafe { littlefs2_sys::lfs_mount(&mut lfs, &lfs_config) };
    if res != 0 {
        return Err(res);
    }

    let file_c = CString::new(file_name).map_err(|_| -22)?;
    let flags = LFS_O_WRONLY | LFS_O_CREAT | LFS_O_EXCL;
    let mut file = unsafe { std::mem::zeroed::<littlefs2_sys::lfs_file_t>() };
    let res = unsafe { littlefs2_sys::lfs_file_open(&mut lfs, &mut file, file_c.as_ptr(), flags) };
    if res != 0 {
        let _ = unsafe { littlefs2_sys::lfs_unmount(&mut lfs) };
        return Err(res);
    }

    let res = unsafe { littlefs2_sys::lfs_file_close(&mut lfs, &mut file) };
    if res != 0 {
        let _ = unsafe { littlefs2_sys::lfs_unmount(&mut lfs) };
        return Err(res);
    }

    let dir_c = CString::new(dir_name).map_err(|_| -22)?;
    let res = unsafe { littlefs2_sys::lfs_mkdir(&mut lfs, dir_c.as_ptr()) };
    if res != 0 {
        let _ = unsafe { littlefs2_sys::lfs_unmount(&mut lfs) };
        return Err(res);
    }

    let res = unsafe { littlefs2_sys::lfs_unmount(&mut lfs) };
    if res != 0 {
        return Err(res);
    }

    Ok(())
}

/// Format, mount, mkdir, file_open(CREAT|EXCL), file_close, unmount.
pub fn format_mkdir_file_unmount(
    storage: &AlignStorage,
    config: &Config,
    dir_name: &str,
    file_name: &str,
) -> Result<(), c_int> {
    let mut lfs = unsafe { std::mem::zeroed::<littlefs2_sys::lfs_t>() };
    let lfs_config = storage.build_lfs_config(config, None, None, None);

    let res = unsafe { littlefs2_sys::lfs_format(&mut lfs, &lfs_config) };
    if res != 0 {
        return Err(res);
    }

    let res = unsafe { littlefs2_sys::lfs_mount(&mut lfs, &lfs_config) };
    if res != 0 {
        return Err(res);
    }

    let dir_c = CString::new(dir_name).map_err(|_| -22)?; // LFS_ERR_INVAL
    let res = unsafe { littlefs2_sys::lfs_mkdir(&mut lfs, dir_c.as_ptr()) };
    if res != 0 {
        let _ = unsafe { littlefs2_sys::lfs_unmount(&mut lfs) };
        return Err(res);
    }

    let file_c = CString::new(file_name).map_err(|_| -22)?;
    let flags = LFS_O_WRONLY | LFS_O_CREAT | LFS_O_EXCL;
    let mut file = unsafe { std::mem::zeroed::<littlefs2_sys::lfs_file_t>() };
    let res = unsafe { littlefs2_sys::lfs_file_open(&mut lfs, &mut file, file_c.as_ptr(), flags) };
    if res != 0 {
        let _ = unsafe { littlefs2_sys::lfs_unmount(&mut lfs) };
        return Err(res);
    }

    let res = unsafe { littlefs2_sys::lfs_file_close(&mut lfs, &mut file) };
    if res != 0 {
        let _ = unsafe { littlefs2_sys::lfs_unmount(&mut lfs) };
        return Err(res);
    }

    let res = unsafe { littlefs2_sys::lfs_unmount(&mut lfs) };
    if res != 0 {
        return Err(res);
    }

    Ok(())
}

/// Mount, dir_open("/"), dir_read (skip ".", ".."), collect names, unmount.
pub fn mount_dir_names(storage: &AlignStorage, config: &Config) -> Result<Vec<String>, c_int> {
    let mut lfs = unsafe { std::mem::zeroed::<littlefs2_sys::lfs_t>() };
    let lfs_config = storage.build_lfs_config(config, None, None, None);

    let res = unsafe { littlefs2_sys::lfs_mount(&mut lfs, &lfs_config) };
    if res != 0 {
        return Err(res);
    }

    let root = CString::new("/").unwrap();
    let mut dir = unsafe { std::mem::zeroed::<littlefs2_sys::lfs_dir_t>() };
    let res = unsafe { littlefs2_sys::lfs_dir_open(&mut lfs, &mut dir, root.as_ptr()) };
    if res != 0 {
        let _ = unsafe { littlefs2_sys::lfs_unmount(&mut lfs) };
        return Err(res);
    }

    let mut names = Vec::new();
    let mut info = unsafe { std::mem::zeroed::<littlefs2_sys::lfs_info>() };

    loop {
        let res = unsafe { littlefs2_sys::lfs_dir_read(&mut lfs, &mut dir, &mut info) };
        if res == 0 {
            break;
        }
        if res < 0 {
            let _ = unsafe { littlefs2_sys::lfs_dir_close(&mut lfs, &mut dir) };
            let _ = unsafe { littlefs2_sys::lfs_unmount(&mut lfs) };
            return Err(res);
        }
        let name = unsafe { CStr::from_ptr(info.name.as_ptr()) }
            .to_str()
            .unwrap_or("")
            .to_string();
        if name != "." && name != ".." {
            names.push(name);
        }
    }

    let _ = unsafe { littlefs2_sys::lfs_dir_close(&mut lfs, &mut dir) };
    let res = unsafe { littlefs2_sys::lfs_unmount(&mut lfs) };
    if res != 0 {
        return Err(res);
    }

    Ok(names)
}

/// Format, mount, mkdir only, unmount.
pub fn format_mkdir_unmount(
    storage: &AlignStorage,
    config: &Config,
    dir_name: &str,
) -> Result<(), c_int> {
    let mut lfs = unsafe { std::mem::zeroed::<littlefs2_sys::lfs_t>() };
    let lfs_config = storage.build_lfs_config(config, None, None, None);

    let res = unsafe { littlefs2_sys::lfs_format(&mut lfs, &lfs_config) };
    if res != 0 {
        return Err(res);
    }

    let res = unsafe { littlefs2_sys::lfs_mount(&mut lfs, &lfs_config) };
    if res != 0 {
        return Err(res);
    }

    let dir_c = CString::new(dir_name).map_err(|_| -22)?;
    let res = unsafe { littlefs2_sys::lfs_mkdir(&mut lfs, dir_c.as_ptr()) };
    if res != 0 {
        let _ = unsafe { littlefs2_sys::lfs_unmount(&mut lfs) };
        return Err(res);
    }

    let res = unsafe { littlefs2_sys::lfs_unmount(&mut lfs) };
    if res != 0 {
        return Err(res);
    }

    Ok(())
}

/// Format, mount, create three entries in insert-before order: "aaa", "zzz", "mmm".
/// "mmm" sorts between "aaa" and "zzz", exercising insert-before when C creates it.
pub fn format_create_three_unmount(storage: &AlignStorage, config: &Config) -> Result<(), c_int> {
    let mut lfs = unsafe { std::mem::zeroed::<littlefs2_sys::lfs_t>() };
    let lfs_config = storage.build_lfs_config(config, None, None, None);

    let res = unsafe { littlefs2_sys::lfs_format(&mut lfs, &lfs_config) };
    if res != 0 {
        return Err(res);
    }

    let res = unsafe { littlefs2_sys::lfs_mount(&mut lfs, &lfs_config) };
    if res != 0 {
        return Err(res);
    }

    for name in ["aaa", "zzz", "mmm"] {
        let name_c = CString::new(name).map_err(|_| -22)?;
        let flags = LFS_O_WRONLY | LFS_O_CREAT | LFS_O_EXCL;
        let mut file = unsafe { std::mem::zeroed::<littlefs2_sys::lfs_file_t>() };
        let res =
            unsafe { littlefs2_sys::lfs_file_open(&mut lfs, &mut file, name_c.as_ptr(), flags) };
        if res != 0 {
            let _ = unsafe { littlefs2_sys::lfs_unmount(&mut lfs) };
            return Err(res);
        }
        let res = unsafe { littlefs2_sys::lfs_file_close(&mut lfs, &mut file) };
        if res != 0 {
            let _ = unsafe { littlefs2_sys::lfs_unmount(&mut lfs) };
            return Err(res);
        }
    }

    let res = unsafe { littlefs2_sys::lfs_unmount(&mut lfs) };
    if res != 0 {
        return Err(res);
    }

    Ok(())
}

/// Mount, mkdir(path), return Ok(()) if mkdir fails with LFS_ERR_EXIST.
pub fn mount_mkdir_expect_exist(
    storage: &AlignStorage,
    config: &Config,
    path: &str,
) -> Result<(), c_int> {
    let mut lfs = unsafe { std::mem::zeroed::<littlefs2_sys::lfs_t>() };
    let lfs_config = storage.build_lfs_config(config, None, None, None);

    let res = unsafe { littlefs2_sys::lfs_mount(&mut lfs, &lfs_config) };
    if res != 0 {
        return Err(res);
    }

    let path_c = CString::new(path).map_err(|_| -22)?;
    let res = unsafe { littlefs2_sys::lfs_mkdir(&mut lfs, path_c.as_ptr()) };
    let _ = unsafe { littlefs2_sys::lfs_unmount(&mut lfs) };

    if res == LFS_ERR_EXIST {
        Ok(())
    } else if res == 0 {
        Err(-1) // Unexpected success
    } else {
        Err(res)
    }
}
