//! C (littlefs2-sys) helper for alignment tests.

use std::ffi::{CStr, CString};
use std::os::raw::{c_int, c_void};

use lp_littlefs::Config;

use crate::storage::AlignStorage;

const LFS_O_RDONLY: c_int = 1;
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
    mount_dir_names_at(storage, config, "/")
}

/// Mount, dir_open(path), dir_read (skip ".", ".."), collect names, unmount.
pub fn mount_dir_names_at(
    storage: &AlignStorage,
    config: &Config,
    path: &str,
) -> Result<Vec<String>, c_int> {
    let mut lfs = unsafe { std::mem::zeroed::<littlefs2_sys::lfs_t>() };
    let lfs_config = storage.build_lfs_config(config, None, None, None);

    let res = unsafe { littlefs2_sys::lfs_mount(&mut lfs, &lfs_config) };
    if res != 0 {
        return Err(res);
    }

    let path_c = CString::new(path).map_err(|_| -22)?;
    let mut dir = unsafe { std::mem::zeroed::<littlefs2_sys::lfs_dir_t>() };
    let res = unsafe { littlefs2_sys::lfs_dir_open(&mut lfs, &mut dir, path_c.as_ptr()) };
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

/// Format, mount, create file, rename old->new, unmount.
pub fn format_create_rename_unmount(
    storage: &AlignStorage,
    config: &Config,
    old_name: &str,
    new_name: &str,
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

    let old_c = CString::new(old_name).map_err(|_| -22)?;
    let flags = LFS_O_WRONLY | LFS_O_CREAT | LFS_O_EXCL;
    let mut file = unsafe { std::mem::zeroed::<littlefs2_sys::lfs_file_t>() };
    let res = unsafe { littlefs2_sys::lfs_file_open(&mut lfs, &mut file, old_c.as_ptr(), flags) };
    if res != 0 {
        let _ = unsafe { littlefs2_sys::lfs_unmount(&mut lfs) };
        return Err(res);
    }

    let res = unsafe { littlefs2_sys::lfs_file_close(&mut lfs, &mut file) };
    if res != 0 {
        let _ = unsafe { littlefs2_sys::lfs_unmount(&mut lfs) };
        return Err(res);
    }

    let old_c = CString::new(old_name).map_err(|_| -22)?;
    let new_c = CString::new(new_name).map_err(|_| -22)?;
    let res = unsafe { littlefs2_sys::lfs_rename(&mut lfs, old_c.as_ptr(), new_c.as_ptr()) };
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

/// Format, mount, create file, remove, unmount.
pub fn format_create_remove_unmount(
    storage: &AlignStorage,
    config: &Config,
    path: &str,
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

    let path_c = CString::new(path).map_err(|_| -22)?;
    let flags = LFS_O_WRONLY | LFS_O_CREAT | LFS_O_EXCL;
    let mut file = unsafe { std::mem::zeroed::<littlefs2_sys::lfs_file_t>() };
    let res = unsafe { littlefs2_sys::lfs_file_open(&mut lfs, &mut file, path_c.as_ptr(), flags) };
    if res != 0 {
        let _ = unsafe { littlefs2_sys::lfs_unmount(&mut lfs) };
        return Err(res);
    }

    let res = unsafe { littlefs2_sys::lfs_file_close(&mut lfs, &mut file) };
    if res != 0 {
        let _ = unsafe { littlefs2_sys::lfs_unmount(&mut lfs) };
        return Err(res);
    }

    let path_c = CString::new(path).map_err(|_| -22)?;
    let res = unsafe { littlefs2_sys::lfs_remove(&mut lfs, path_c.as_ptr()) };
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

/// Format, mount, create file, write content, close, unmount.
pub fn format_create_write_unmount(
    storage: &AlignStorage,
    config: &Config,
    path: &str,
    content: &[u8],
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

    let path_c = CString::new(path).map_err(|_| -22)?;
    let flags = LFS_O_WRONLY | LFS_O_CREAT | LFS_O_EXCL;
    let mut file = unsafe { std::mem::zeroed::<littlefs2_sys::lfs_file_t>() };
    let res = unsafe { littlefs2_sys::lfs_file_open(&mut lfs, &mut file, path_c.as_ptr(), flags) };
    if res != 0 {
        let _ = unsafe { littlefs2_sys::lfs_unmount(&mut lfs) };
        return Err(res);
    }

    let written = unsafe {
        littlefs2_sys::lfs_file_write(
            &mut lfs,
            &mut file,
            content.as_ptr() as *const c_void,
            content.len() as littlefs2_sys::lfs_size_t,
        )
    };
    if written < 0 || written as usize != content.len() {
        let _ = unsafe { littlefs2_sys::lfs_file_close(&mut lfs, &mut file) };
        let _ = unsafe { littlefs2_sys::lfs_unmount(&mut lfs) };
        return Err(if written < 0 { written as c_int } else { -5 });
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

/// Mount, open path for read, read full content into vec, unmount.
pub fn mount_read_file(
    storage: &AlignStorage,
    config: &Config,
    path: &str,
) -> Result<Vec<u8>, c_int> {
    let mut lfs = unsafe { std::mem::zeroed::<littlefs2_sys::lfs_t>() };
    let lfs_config = storage.build_lfs_config(config, None, None, None);

    let res = unsafe { littlefs2_sys::lfs_mount(&mut lfs, &lfs_config) };
    if res != 0 {
        return Err(res);
    }

    let path_c = CString::new(path).map_err(|_| -22)?;
    let mut file = unsafe { std::mem::zeroed::<littlefs2_sys::lfs_file_t>() };
    let res =
        unsafe { littlefs2_sys::lfs_file_open(&mut lfs, &mut file, path_c.as_ptr(), LFS_O_RDONLY) };
    if res != 0 {
        let _ = unsafe { littlefs2_sys::lfs_unmount(&mut lfs) };
        return Err(res);
    }

    let mut buf = Vec::new();
    let mut chunk = [0u8; 256];
    loop {
        let n = unsafe {
            littlefs2_sys::lfs_file_read(
                &mut lfs,
                &mut file,
                chunk.as_mut_ptr() as *mut c_void,
                chunk.len() as littlefs2_sys::lfs_size_t,
            )
        };
        if n < 0 {
            let _ = unsafe { littlefs2_sys::lfs_file_close(&mut lfs, &mut file) };
            let _ = unsafe { littlefs2_sys::lfs_unmount(&mut lfs) };
            return Err(n as c_int);
        }
        if n == 0 {
            break;
        }
        buf.extend_from_slice(&chunk[..n as usize]);
    }

    let _ = unsafe { littlefs2_sys::lfs_file_close(&mut lfs, &mut file) };
    let res = unsafe { littlefs2_sys::lfs_unmount(&mut lfs) };
    if res != 0 {
        return Err(res);
    }

    Ok(buf)
}

/// Format, mount, mkdir parent, mkdir parent/child, create parent/child/file, unmount.
pub fn format_nested_dir_file_unmount(
    storage: &AlignStorage,
    config: &Config,
    parent: &str,
    child: &str,
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

    let parent_c = CString::new(parent).map_err(|_| -22)?;
    let res = unsafe { littlefs2_sys::lfs_mkdir(&mut lfs, parent_c.as_ptr()) };
    if res != 0 {
        let _ = unsafe { littlefs2_sys::lfs_unmount(&mut lfs) };
        return Err(res);
    }

    let child_path = format!("{}/{}", parent, child);
    let child_c = CString::new(child_path).map_err(|_| -22)?;
    let res = unsafe { littlefs2_sys::lfs_mkdir(&mut lfs, child_c.as_ptr()) };
    if res != 0 {
        let _ = unsafe { littlefs2_sys::lfs_unmount(&mut lfs) };
        return Err(res);
    }

    let file_path = format!("{}/{}/{}", parent, child, file_name);
    let file_c = CString::new(file_path).map_err(|_| -22)?;
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

/// Format, mount, mkdir, create file in dir, remove file, rmdir, unmount.
pub fn format_mkdir_file_rmdir_unmount(
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

    let dir_c = CString::new(dir_name).map_err(|_| -22)?;
    let res = unsafe { littlefs2_sys::lfs_mkdir(&mut lfs, dir_c.as_ptr()) };
    if res != 0 {
        let _ = unsafe { littlefs2_sys::lfs_unmount(&mut lfs) };
        return Err(res);
    }

    let file_path = format!("{}/{}", dir_name, file_name);
    let file_c = CString::new(file_path).map_err(|_| -22)?;
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

    let file_path = format!("{}/{}", dir_name, file_name);
    let file_c = CString::new(file_path).map_err(|_| -22)?;
    let res = unsafe { littlefs2_sys::lfs_remove(&mut lfs, file_c.as_ptr()) };
    if res != 0 {
        let _ = unsafe { littlefs2_sys::lfs_unmount(&mut lfs) };
        return Err(res);
    }

    let dir_c = CString::new(dir_name).map_err(|_| -22)?;
    let res = unsafe { littlefs2_sys::lfs_remove(&mut lfs, dir_c.as_ptr()) };
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
