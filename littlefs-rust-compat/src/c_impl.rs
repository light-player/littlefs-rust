//! C (littlefs2-sys) wrapper for compat tests.

use std::ffi::{CStr, CString};
use std::os::raw::{c_int, c_void};

use crate::storage::{check, prng_verify, test_prng, SharedStorage};

const LFS_O_RDONLY: c_int = 1;
const LFS_O_WRONLY: c_int = 2;
const LFS_O_CREAT: c_int = 0x0100;
const LFS_O_EXCL: c_int = 0x0200;
const LFS_ERR_EXIST: c_int = -17;

// ── Operation-level helpers (phase 2) ───────────────────────────────────

pub fn format(storage: &SharedStorage) -> Result<(), i32> {
    let config = storage.build_c_config();
    let mut lfs = unsafe { std::mem::zeroed::<littlefs2_sys::lfs_t>() };

    check(unsafe { littlefs2_sys::lfs_format(&mut lfs, &config) })?;
    check(unsafe { littlefs2_sys::lfs_mount(&mut lfs, &config) })?;
    check(unsafe { littlefs2_sys::lfs_unmount(&mut lfs) })?;
    Ok(())
}

pub fn mount_dir_names(storage: &SharedStorage, path: &str) -> Result<Vec<String>, i32> {
    let config = storage.build_c_config();
    let mut lfs = unsafe { std::mem::zeroed::<littlefs2_sys::lfs_t>() };

    check(unsafe { littlefs2_sys::lfs_mount(&mut lfs, &config) })?;
    let names = dir_names_mounted(&mut lfs, path)?;
    check(unsafe { littlefs2_sys::lfs_unmount(&mut lfs) })?;
    Ok(names)
}

pub fn mount_read_file(storage: &SharedStorage, path: &str) -> Result<Vec<u8>, i32> {
    let config = storage.build_c_config();
    let mut lfs = unsafe { std::mem::zeroed::<littlefs2_sys::lfs_t>() };

    check(unsafe { littlefs2_sys::lfs_mount(&mut lfs, &config) })?;
    let data = read_file_mounted(&mut lfs, path)?;
    check(unsafe { littlefs2_sys::lfs_unmount(&mut lfs) })?;
    Ok(data)
}

pub fn format_mkdir_unmount(storage: &SharedStorage, dir_name: &str) -> Result<(), i32> {
    let config = storage.build_c_config();
    let mut lfs = unsafe { std::mem::zeroed::<littlefs2_sys::lfs_t>() };

    check(unsafe { littlefs2_sys::lfs_format(&mut lfs, &config) })?;
    check(unsafe { littlefs2_sys::lfs_mount(&mut lfs, &config) })?;
    mkdir_mounted(&mut lfs, dir_name)?;
    check(unsafe { littlefs2_sys::lfs_unmount(&mut lfs) })?;
    Ok(())
}

pub fn format_mkdir_file_unmount(
    storage: &SharedStorage,
    dir_name: &str,
    file_name: &str,
) -> Result<(), i32> {
    let config = storage.build_c_config();
    let mut lfs = unsafe { std::mem::zeroed::<littlefs2_sys::lfs_t>() };

    check(unsafe { littlefs2_sys::lfs_format(&mut lfs, &config) })?;
    check(unsafe { littlefs2_sys::lfs_mount(&mut lfs, &config) })?;
    mkdir_mounted(&mut lfs, dir_name)?;
    create_empty_file_mounted(&mut lfs, file_name)?;
    check(unsafe { littlefs2_sys::lfs_unmount(&mut lfs) })?;
    Ok(())
}

pub fn format_file_mkdir_unmount(
    storage: &SharedStorage,
    file_name: &str,
    dir_name: &str,
) -> Result<(), i32> {
    let config = storage.build_c_config();
    let mut lfs = unsafe { std::mem::zeroed::<littlefs2_sys::lfs_t>() };

    check(unsafe { littlefs2_sys::lfs_format(&mut lfs, &config) })?;
    check(unsafe { littlefs2_sys::lfs_mount(&mut lfs, &config) })?;
    create_empty_file_mounted(&mut lfs, file_name)?;
    mkdir_mounted(&mut lfs, dir_name)?;
    check(unsafe { littlefs2_sys::lfs_unmount(&mut lfs) })?;
    Ok(())
}

pub fn format_create_three_unmount(storage: &SharedStorage) -> Result<(), i32> {
    let config = storage.build_c_config();
    let mut lfs = unsafe { std::mem::zeroed::<littlefs2_sys::lfs_t>() };

    check(unsafe { littlefs2_sys::lfs_format(&mut lfs, &config) })?;
    check(unsafe { littlefs2_sys::lfs_mount(&mut lfs, &config) })?;
    for name in ["aaa", "zzz", "mmm"] {
        create_empty_file_mounted(&mut lfs, name)?;
    }
    check(unsafe { littlefs2_sys::lfs_unmount(&mut lfs) })?;
    Ok(())
}

pub fn format_create_rename_unmount(
    storage: &SharedStorage,
    old_name: &str,
    new_name: &str,
) -> Result<(), i32> {
    let config = storage.build_c_config();
    let mut lfs = unsafe { std::mem::zeroed::<littlefs2_sys::lfs_t>() };

    check(unsafe { littlefs2_sys::lfs_format(&mut lfs, &config) })?;
    check(unsafe { littlefs2_sys::lfs_mount(&mut lfs, &config) })?;
    create_empty_file_mounted(&mut lfs, old_name)?;
    let old_c = CString::new(old_name).unwrap();
    let new_c = CString::new(new_name).unwrap();
    check(unsafe { littlefs2_sys::lfs_rename(&mut lfs, old_c.as_ptr(), new_c.as_ptr()) })?;
    check(unsafe { littlefs2_sys::lfs_unmount(&mut lfs) })?;
    Ok(())
}

pub fn format_create_remove_unmount(storage: &SharedStorage, path: &str) -> Result<(), i32> {
    let config = storage.build_c_config();
    let mut lfs = unsafe { std::mem::zeroed::<littlefs2_sys::lfs_t>() };

    check(unsafe { littlefs2_sys::lfs_format(&mut lfs, &config) })?;
    check(unsafe { littlefs2_sys::lfs_mount(&mut lfs, &config) })?;
    create_empty_file_mounted(&mut lfs, path)?;
    let path_c = CString::new(path).unwrap();
    check(unsafe { littlefs2_sys::lfs_remove(&mut lfs, path_c.as_ptr()) })?;
    check(unsafe { littlefs2_sys::lfs_unmount(&mut lfs) })?;
    Ok(())
}

pub fn format_create_write_unmount(
    storage: &SharedStorage,
    path: &str,
    content: &[u8],
) -> Result<(), i32> {
    let config = storage.build_c_config();
    let mut lfs = unsafe { std::mem::zeroed::<littlefs2_sys::lfs_t>() };

    check(unsafe { littlefs2_sys::lfs_format(&mut lfs, &config) })?;
    check(unsafe { littlefs2_sys::lfs_mount(&mut lfs, &config) })?;
    write_file_mounted(&mut lfs, path, content)?;
    check(unsafe { littlefs2_sys::lfs_unmount(&mut lfs) })?;
    Ok(())
}

pub fn format_nested_dir_file_unmount(
    storage: &SharedStorage,
    parent: &str,
    child: &str,
    file_name: &str,
) -> Result<(), i32> {
    let config = storage.build_c_config();
    let mut lfs = unsafe { std::mem::zeroed::<littlefs2_sys::lfs_t>() };

    check(unsafe { littlefs2_sys::lfs_format(&mut lfs, &config) })?;
    check(unsafe { littlefs2_sys::lfs_mount(&mut lfs, &config) })?;
    mkdir_mounted(&mut lfs, parent)?;
    let child_path = format!("{parent}/{child}");
    mkdir_mounted(&mut lfs, &child_path)?;
    let file_path = format!("{child_path}/{file_name}");
    create_empty_file_mounted(&mut lfs, &file_path)?;
    check(unsafe { littlefs2_sys::lfs_unmount(&mut lfs) })?;
    Ok(())
}

pub fn format_mkdir_file_rmdir_unmount(
    storage: &SharedStorage,
    dir_name: &str,
    file_name: &str,
) -> Result<(), i32> {
    let config = storage.build_c_config();
    let mut lfs = unsafe { std::mem::zeroed::<littlefs2_sys::lfs_t>() };

    check(unsafe { littlefs2_sys::lfs_format(&mut lfs, &config) })?;
    check(unsafe { littlefs2_sys::lfs_mount(&mut lfs, &config) })?;
    mkdir_mounted(&mut lfs, dir_name)?;
    let file_path = format!("{dir_name}/{file_name}");
    create_empty_file_mounted(&mut lfs, &file_path)?;
    let fp_c = CString::new(file_path).unwrap();
    check(unsafe { littlefs2_sys::lfs_remove(&mut lfs, fp_c.as_ptr()) })?;
    let dir_c = CString::new(dir_name).unwrap();
    check(unsafe { littlefs2_sys::lfs_remove(&mut lfs, dir_c.as_ptr()) })?;
    check(unsafe { littlefs2_sys::lfs_unmount(&mut lfs) })?;
    Ok(())
}

pub fn mount_mkdir_expect_exist(storage: &SharedStorage, path: &str) -> Result<(), i32> {
    let config = storage.build_c_config();
    let mut lfs = unsafe { std::mem::zeroed::<littlefs2_sys::lfs_t>() };

    check(unsafe { littlefs2_sys::lfs_mount(&mut lfs, &config) })?;
    let path_c = CString::new(path).unwrap();
    let res = unsafe { littlefs2_sys::lfs_mkdir(&mut lfs, path_c.as_ptr()) };
    check(unsafe { littlefs2_sys::lfs_unmount(&mut lfs) })?;
    if res == LFS_ERR_EXIST {
        Ok(())
    } else if res == 0 {
        Err(-1)
    } else {
        Err(res)
    }
}

// ── Compat-level helpers (phase 3) ──────────────────────────────────────

/// Format only (no mount cycle).
pub fn format_only(storage: &SharedStorage) -> Result<(), i32> {
    let config = storage.build_c_config();
    let mut lfs = unsafe { std::mem::zeroed::<littlefs2_sys::lfs_t>() };
    check(unsafe { littlefs2_sys::lfs_format(&mut lfs, &config) })?;
    Ok(())
}

/// Format, mount, create `count` dirs named "dir0".."dir{count-1}", unmount.
pub fn format_create_n_dirs(storage: &SharedStorage, count: usize) -> Result<(), i32> {
    let config = storage.build_c_config();
    let mut lfs = unsafe { std::mem::zeroed::<littlefs2_sys::lfs_t>() };

    check(unsafe { littlefs2_sys::lfs_format(&mut lfs, &config) })?;
    check(unsafe { littlefs2_sys::lfs_mount(&mut lfs, &config) })?;
    for i in 0..count {
        mkdir_mounted(&mut lfs, &format!("dir{i}"))?;
    }
    check(unsafe { littlefs2_sys::lfs_unmount(&mut lfs) })?;
    Ok(())
}

/// Format, mount, create `count` files with PRNG data, unmount.
/// File names: "file0".."file{count-1}". Seed for file i = i+1.
pub fn format_create_n_files_prng(
    storage: &SharedStorage,
    count: usize,
    size: u32,
    chunk: u32,
) -> Result<(), i32> {
    let config = storage.build_c_config();
    let mut lfs = unsafe { std::mem::zeroed::<littlefs2_sys::lfs_t>() };

    check(unsafe { littlefs2_sys::lfs_format(&mut lfs, &config) })?;
    check(unsafe { littlefs2_sys::lfs_mount(&mut lfs, &config) })?;
    for i in 0..count {
        write_prng_file_mounted(&mut lfs, &format!("file{i}"), size, chunk, (i + 1) as u32)?;
    }
    check(unsafe { littlefs2_sys::lfs_unmount(&mut lfs) })?;
    Ok(())
}

/// Format, mount, create `count` dirs each with a PRNG file, unmount.
/// Dir names: "dir0".."dir{count-1}", file name: "file" in each dir.
pub fn format_create_n_dirs_with_files_prng(
    storage: &SharedStorage,
    count: usize,
    size: u32,
    chunk: u32,
) -> Result<(), i32> {
    let config = storage.build_c_config();
    let mut lfs = unsafe { std::mem::zeroed::<littlefs2_sys::lfs_t>() };

    check(unsafe { littlefs2_sys::lfs_format(&mut lfs, &config) })?;
    check(unsafe { littlefs2_sys::lfs_mount(&mut lfs, &config) })?;
    for i in 0..count {
        let dir = format!("dir{i}");
        mkdir_mounted(&mut lfs, &dir)?;
        write_prng_file_mounted(
            &mut lfs,
            &format!("{dir}/file"),
            size,
            chunk,
            (i + 1) as u32,
        )?;
    }
    check(unsafe { littlefs2_sys::lfs_unmount(&mut lfs) })?;
    Ok(())
}

/// Mount, verify `count` dirs exist at root, each empty (only . and ..), unmount.
pub fn mount_verify_n_empty_dirs(storage: &SharedStorage, count: usize) -> Result<(), i32> {
    let config = storage.build_c_config();
    let mut lfs = unsafe { std::mem::zeroed::<littlefs2_sys::lfs_t>() };

    check(unsafe { littlefs2_sys::lfs_mount(&mut lfs, &config) })?;
    let root = dir_names_mounted(&mut lfs, "/")?;
    assert_eq!(
        root.len(),
        count,
        "expected {count} dirs, got {}",
        root.len()
    );
    for i in 0..count {
        let name = format!("dir{i}");
        assert!(root.contains(&name), "missing {name}");
        let contents = dir_names_mounted(&mut lfs, &name)?;
        assert!(
            contents.is_empty(),
            "dir {name} should be empty, got {contents:?}"
        );
    }
    check(unsafe { littlefs2_sys::lfs_unmount(&mut lfs) })?;
    Ok(())
}

/// Mount, verify `count` files with PRNG content, unmount.
pub fn mount_verify_n_files_prng(
    storage: &SharedStorage,
    count: usize,
    size: u32,
    _chunk: u32,
) -> Result<(), i32> {
    let config = storage.build_c_config();
    let mut lfs = unsafe { std::mem::zeroed::<littlefs2_sys::lfs_t>() };

    check(unsafe { littlefs2_sys::lfs_mount(&mut lfs, &config) })?;
    let root = dir_names_mounted(&mut lfs, "/")?;
    assert_eq!(
        root.len(),
        count,
        "expected {count} files, got {}",
        root.len()
    );
    for i in 0..count {
        let path = format!("file{i}");
        let data = read_file_mounted(&mut lfs, &path)?;
        assert_eq!(data.len(), size as usize, "file {path} size mismatch");
        prng_verify(&data, (i + 1) as u32);
    }
    check(unsafe { littlefs2_sys::lfs_unmount(&mut lfs) })?;
    Ok(())
}

/// Mount, verify `count` dirs each with a PRNG file, unmount.
pub fn mount_verify_n_dirs_with_files_prng(
    storage: &SharedStorage,
    count: usize,
    size: u32,
    _chunk: u32,
) -> Result<(), i32> {
    let config = storage.build_c_config();
    let mut lfs = unsafe { std::mem::zeroed::<littlefs2_sys::lfs_t>() };

    check(unsafe { littlefs2_sys::lfs_mount(&mut lfs, &config) })?;
    let root = dir_names_mounted(&mut lfs, "/")?;
    assert_eq!(
        root.len(),
        count,
        "expected {count} dirs, got {}",
        root.len()
    );
    for i in 0..count {
        let dir = format!("dir{i}");
        let contents = dir_names_mounted(&mut lfs, &dir)?;
        assert_eq!(contents.len(), 1, "dir {dir} should have 1 file");
        let data = read_file_mounted(&mut lfs, &format!("{dir}/file"))?;
        assert_eq!(data.len(), size as usize);
        prng_verify(&data, (i + 1) as u32);
    }
    check(unsafe { littlefs2_sys::lfs_unmount(&mut lfs) })?;
    Ok(())
}

/// Mount, create dirs `start..start+count`, list root (expect `expected` total), unmount.
pub fn mount_create_dirs_and_list(
    storage: &SharedStorage,
    start: usize,
    count: usize,
    expected: usize,
) -> Result<Vec<String>, i32> {
    let config = storage.build_c_config();
    let mut lfs = unsafe { std::mem::zeroed::<littlefs2_sys::lfs_t>() };

    check(unsafe { littlefs2_sys::lfs_mount(&mut lfs, &config) })?;
    for i in start..(start + count) {
        mkdir_mounted(&mut lfs, &format!("dir{i}"))?;
    }
    let root = dir_names_mounted(&mut lfs, "/")?;
    assert_eq!(
        root.len(),
        expected,
        "expected {expected} entries, got {}",
        root.len()
    );
    check(unsafe { littlefs2_sys::lfs_unmount(&mut lfs) })?;
    Ok(root)
}

/// Mount, create files `start..start+count` with PRNG, read+verify all `total` files, unmount.
pub fn mount_create_files_prng_and_verify_all(
    storage: &SharedStorage,
    start: usize,
    count: usize,
    total: usize,
    size: u32,
    chunk: u32,
) -> Result<(), i32> {
    let config = storage.build_c_config();
    let mut lfs = unsafe { std::mem::zeroed::<littlefs2_sys::lfs_t>() };

    check(unsafe { littlefs2_sys::lfs_mount(&mut lfs, &config) })?;
    for i in start..(start + count) {
        write_prng_file_mounted(&mut lfs, &format!("file{i}"), size, chunk, (i + 1) as u32)?;
    }
    let root = dir_names_mounted(&mut lfs, "/")?;
    assert_eq!(
        root.len(),
        total,
        "expected {total} files, got {}",
        root.len()
    );
    for i in 0..total {
        let data = read_file_mounted(&mut lfs, &format!("file{i}"))?;
        assert_eq!(data.len(), size as usize);
        prng_verify(&data, (i + 1) as u32);
    }
    check(unsafe { littlefs2_sys::lfs_unmount(&mut lfs) })?;
    Ok(())
}

/// Mount, create dirs+files `start..start+count`, verify all `total` dirs+files, unmount.
pub fn mount_create_dirs_files_prng_and_verify_all(
    storage: &SharedStorage,
    start: usize,
    count: usize,
    total: usize,
    size: u32,
    chunk: u32,
) -> Result<(), i32> {
    let config = storage.build_c_config();
    let mut lfs = unsafe { std::mem::zeroed::<littlefs2_sys::lfs_t>() };

    check(unsafe { littlefs2_sys::lfs_mount(&mut lfs, &config) })?;
    for i in start..(start + count) {
        let dir = format!("dir{i}");
        mkdir_mounted(&mut lfs, &dir)?;
        write_prng_file_mounted(
            &mut lfs,
            &format!("{dir}/file"),
            size,
            chunk,
            (i + 1) as u32,
        )?;
    }
    let root = dir_names_mounted(&mut lfs, "/")?;
    assert_eq!(
        root.len(),
        total,
        "expected {total} dirs, got {}",
        root.len()
    );
    for i in 0..total {
        let dir = format!("dir{i}");
        let data = read_file_mounted(&mut lfs, &format!("{dir}/file"))?;
        assert_eq!(data.len(), size as usize);
        prng_verify(&data, (i + 1) as u32);
    }
    check(unsafe { littlefs2_sys::lfs_unmount(&mut lfs) })?;
    Ok(())
}

// ── Internal helpers ────────────────────────────────────────────────────

fn mkdir_mounted(lfs: &mut littlefs2_sys::lfs_t, path: &str) -> Result<(), i32> {
    let path_c = CString::new(path).unwrap();
    check(unsafe { littlefs2_sys::lfs_mkdir(lfs, path_c.as_ptr()) })
}

fn create_empty_file_mounted(lfs: &mut littlefs2_sys::lfs_t, path: &str) -> Result<(), i32> {
    let path_c = CString::new(path).unwrap();
    let flags = LFS_O_WRONLY | LFS_O_CREAT | LFS_O_EXCL;
    let mut file = unsafe { std::mem::zeroed::<littlefs2_sys::lfs_file_t>() };
    check(unsafe { littlefs2_sys::lfs_file_open(lfs, &mut file, path_c.as_ptr(), flags) })?;
    check(unsafe { littlefs2_sys::lfs_file_close(lfs, &mut file) })
}

fn write_file_mounted(
    lfs: &mut littlefs2_sys::lfs_t,
    path: &str,
    content: &[u8],
) -> Result<(), i32> {
    let path_c = CString::new(path).unwrap();
    let flags = LFS_O_WRONLY | LFS_O_CREAT | LFS_O_EXCL;
    let mut file = unsafe { std::mem::zeroed::<littlefs2_sys::lfs_file_t>() };
    check(unsafe { littlefs2_sys::lfs_file_open(lfs, &mut file, path_c.as_ptr(), flags) })?;
    let written = unsafe {
        littlefs2_sys::lfs_file_write(
            lfs,
            &mut file,
            content.as_ptr() as *const c_void,
            content.len() as littlefs2_sys::lfs_size_t,
        )
    };
    check(unsafe { littlefs2_sys::lfs_file_close(lfs, &mut file) })?;
    if written < 0 {
        return Err(written as i32);
    }
    assert_eq!(written as usize, content.len(), "short write");
    Ok(())
}

fn write_prng_file_mounted(
    lfs: &mut littlefs2_sys::lfs_t,
    path: &str,
    size: u32,
    chunk: u32,
    seed: u32,
) -> Result<(), i32> {
    let path_c = CString::new(path).unwrap();
    let flags = LFS_O_WRONLY | LFS_O_CREAT | LFS_O_EXCL;
    let mut file = unsafe { std::mem::zeroed::<littlefs2_sys::lfs_file_t>() };
    check(unsafe { littlefs2_sys::lfs_file_open(lfs, &mut file, path_c.as_ptr(), flags) })?;

    let mut prng = seed;
    let mut buf = vec![0u8; chunk as usize];
    let mut i: u32 = 0;
    while i < size {
        let c = std::cmp::min(chunk, size - i);
        for slot in buf[..c as usize].iter_mut() {
            *slot = (test_prng(&mut prng) & 0xff) as u8;
        }
        let n = unsafe {
            littlefs2_sys::lfs_file_write(
                lfs,
                &mut file,
                buf.as_ptr() as *const c_void,
                c as littlefs2_sys::lfs_size_t,
            )
        };
        assert_eq!(n, c as i32, "short write at offset {i}");
        i += c;
    }
    check(unsafe { littlefs2_sys::lfs_file_close(lfs, &mut file) })
}

fn read_file_mounted(lfs: &mut littlefs2_sys::lfs_t, path: &str) -> Result<Vec<u8>, i32> {
    let path_c = CString::new(path).unwrap();
    let mut file = unsafe { std::mem::zeroed::<littlefs2_sys::lfs_file_t>() };
    check(unsafe { littlefs2_sys::lfs_file_open(lfs, &mut file, path_c.as_ptr(), LFS_O_RDONLY) })?;

    let mut buf = Vec::new();
    let mut chunk = [0u8; 256];
    loop {
        let n = unsafe {
            littlefs2_sys::lfs_file_read(
                lfs,
                &mut file,
                chunk.as_mut_ptr() as *mut c_void,
                chunk.len() as littlefs2_sys::lfs_size_t,
            )
        };
        if n < 0 {
            let _ = unsafe { littlefs2_sys::lfs_file_close(lfs, &mut file) };
            return Err(n as i32);
        }
        if n == 0 {
            break;
        }
        buf.extend_from_slice(&chunk[..n as usize]);
    }
    check(unsafe { littlefs2_sys::lfs_file_close(lfs, &mut file) })?;
    Ok(buf)
}

fn dir_names_mounted(lfs: &mut littlefs2_sys::lfs_t, path: &str) -> Result<Vec<String>, i32> {
    let path_c = CString::new(path).unwrap();
    let mut dir = unsafe { std::mem::zeroed::<littlefs2_sys::lfs_dir_t>() };
    check(unsafe { littlefs2_sys::lfs_dir_open(lfs, &mut dir, path_c.as_ptr()) })?;

    let mut names = Vec::new();
    let mut info = unsafe { std::mem::zeroed::<littlefs2_sys::lfs_info>() };
    loop {
        let res = unsafe { littlefs2_sys::lfs_dir_read(lfs, &mut dir, &mut info) };
        if res == 0 {
            break;
        }
        if res < 0 {
            let _ = unsafe { littlefs2_sys::lfs_dir_close(lfs, &mut dir) };
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
    let _ = unsafe { littlefs2_sys::lfs_dir_close(lfs, &mut dir) };
    Ok(names)
}
