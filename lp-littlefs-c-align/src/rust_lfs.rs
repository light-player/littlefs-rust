//! Rust (lp-littlefs) helper for alignment tests.

use lp_littlefs::{BlockDevice, Config, Dir, Error, FileType, Info, LittleFs, OpenFlags};

use crate::storage::AlignStorage;

/// Format only.
pub fn format(storage: &AlignStorage, config: &Config) -> Result<(), Error> {
    let mut lfs = LittleFs::new();
    lfs.format(storage, config)?;
    Ok(())
}

/// Format, mount, file_open(CREAT|EXCL), mkdir, file_close, unmount.
/// Reverse order: file first, then dir.
pub fn format_file_mkdir_unmount(
    storage: &AlignStorage,
    config: &Config,
    file_name: &str,
    dir_name: &str,
) -> Result<(), Error> {
    let mut lfs = LittleFs::new();
    lfs.format(storage, config)?;
    lfs.mount(storage, config)?;
    let file = lfs.file_open(
        storage,
        config,
        file_name,
        OpenFlags::new(OpenFlags::WRONLY | OpenFlags::CREAT | OpenFlags::EXCL),
    )?;
    lfs.file_close(storage, config, file)?;
    lfs.mkdir(storage, config, dir_name)?;
    lfs.unmount(storage, config)?;
    Ok(())
}

/// Format, mount, mkdir, file_open(CREAT|EXCL), file_close, unmount.
pub fn format_mkdir_file_unmount(
    storage: &AlignStorage,
    config: &Config,
    dir_name: &str,
    file_name: &str,
) -> Result<(), Error> {
    let mut lfs = LittleFs::new();
    lfs.format(storage, config)?;
    lfs.mount(storage, config)?;
    lfs.mkdir(storage, config, dir_name)?;
    let file = lfs.file_open(
        storage,
        config,
        file_name,
        OpenFlags::new(OpenFlags::WRONLY | OpenFlags::CREAT | OpenFlags::EXCL),
    )?;
    lfs.file_close(storage, config, file)?;
    lfs.unmount(storage, config)?;
    Ok(())
}

/// Format, mount, create three entries: "aaa", "zzz", "mmm" (insert-before order).
pub fn format_create_three_unmount(storage: &AlignStorage, config: &Config) -> Result<(), Error> {
    let mut lfs = LittleFs::new();
    lfs.format(storage, config)?;
    lfs.mount(storage, config)?;
    for name in ["aaa", "zzz", "mmm"] {
        let file = lfs.file_open(
            storage,
            config,
            name,
            OpenFlags::new(OpenFlags::WRONLY | OpenFlags::CREAT | OpenFlags::EXCL),
        )?;
        lfs.file_close(storage, config, file)?;
    }
    lfs.unmount(storage, config)?;
    Ok(())
}

/// Mount, dir_open, dir_read (skip ".", ".."), collect names, unmount.
pub fn mount_dir_names(storage: &AlignStorage, config: &Config) -> Result<Vec<String>, Error> {
    mount_dir_names_at(storage, config, "/")
}

/// Mount, dir_open(path), dir_read (skip ".", ".."), collect names, unmount.
pub fn mount_dir_names_at(
    storage: &AlignStorage,
    config: &Config,
    path: &str,
) -> Result<Vec<String>, Error> {
    let mut lfs = LittleFs::new();
    lfs.mount(storage, config)?;
    let path_ = if path == "/" { "/" } else { path };
    let names = dir_entry_names(&mut lfs, storage, config, path_)?;
    lfs.unmount(storage, config)?;
    Ok(names)
}

/// Format, mount, create file, rename old->new, unmount.
pub fn format_create_rename_unmount(
    storage: &AlignStorage,
    config: &Config,
    old_name: &str,
    new_name: &str,
) -> Result<(), Error> {
    let mut lfs = LittleFs::new();
    lfs.format(storage, config)?;
    lfs.mount(storage, config)?;
    let file = lfs.file_open(
        storage,
        config,
        old_name,
        OpenFlags::new(OpenFlags::WRONLY | OpenFlags::CREAT | OpenFlags::EXCL),
    )?;
    lfs.file_close(storage, config, file)?;
    lfs.rename(storage, config, old_name, new_name)?;
    lfs.unmount(storage, config)?;
    Ok(())
}

/// Format, mount, create file, remove, unmount.
pub fn format_create_remove_unmount(
    storage: &AlignStorage,
    config: &Config,
    path: &str,
) -> Result<(), Error> {
    let mut lfs = LittleFs::new();
    lfs.format(storage, config)?;
    lfs.mount(storage, config)?;
    let file = lfs.file_open(
        storage,
        config,
        path,
        OpenFlags::new(OpenFlags::WRONLY | OpenFlags::CREAT | OpenFlags::EXCL),
    )?;
    lfs.file_close(storage, config, file)?;
    lfs.remove(storage, config, path)?;
    lfs.unmount(storage, config)?;
    Ok(())
}

/// Format, mount, create file, write content, close, unmount.
pub fn format_create_write_unmount(
    storage: &AlignStorage,
    config: &Config,
    path: &str,
    content: &[u8],
) -> Result<(), Error> {
    let mut lfs = LittleFs::new();
    lfs.format(storage, config)?;
    lfs.mount(storage, config)?;
    let mut file = lfs.file_open(
        storage,
        config,
        path,
        OpenFlags::new(OpenFlags::WRONLY | OpenFlags::CREAT | OpenFlags::EXCL),
    )?;
    lfs.file_write(storage, config, &mut file, content)?;
    lfs.file_close(storage, config, file)?;
    lfs.unmount(storage, config)?;
    Ok(())
}

/// Mount, open path for read, read full content into vec, unmount.
pub fn mount_read_file(
    storage: &AlignStorage,
    config: &Config,
    path: &str,
) -> Result<Vec<u8>, Error> {
    let mut lfs = LittleFs::new();
    lfs.mount(storage, config)?;
    let mut file = lfs.file_open(storage, config, path, OpenFlags::new(OpenFlags::RDONLY))?;
    let mut buf = Vec::new();
    let mut chunk = [0u8; 256];
    loop {
        let n = lfs.file_read(storage, config, &mut file, &mut chunk)?;
        if n == 0 {
            break;
        }
        buf.extend_from_slice(&chunk[..n]);
    }
    lfs.file_close(storage, config, file)?;
    lfs.unmount(storage, config)?;
    Ok(buf)
}

/// Format, mount, mkdir parent, mkdir parent/child, create parent/child/file, unmount.
pub fn format_nested_dir_file_unmount(
    storage: &AlignStorage,
    config: &Config,
    parent: &str,
    child: &str,
    file_name: &str,
) -> Result<(), Error> {
    let mut lfs = LittleFs::new();
    lfs.format(storage, config)?;
    lfs.mount(storage, config)?;
    lfs.mkdir(storage, config, parent)?;
    let child_path = format!("{}/{}", parent, child);
    lfs.mkdir(storage, config, &child_path)?;
    let file_path = format!("{}/{}", child_path, file_name);
    let file = lfs.file_open(
        storage,
        config,
        &file_path,
        OpenFlags::new(OpenFlags::WRONLY | OpenFlags::CREAT | OpenFlags::EXCL),
    )?;
    lfs.file_close(storage, config, file)?;
    lfs.unmount(storage, config)?;
    Ok(())
}

/// Format, mount, mkdir, create file in dir, remove file, rmdir, unmount.
pub fn format_mkdir_file_rmdir_unmount(
    storage: &AlignStorage,
    config: &Config,
    dir_name: &str,
    file_name: &str,
) -> Result<(), Error> {
    let mut lfs = LittleFs::new();
    lfs.format(storage, config)?;
    lfs.mount(storage, config)?;
    lfs.mkdir(storage, config, dir_name)?;
    let file_path = format!("{}/{}", dir_name, file_name);
    let file = lfs.file_open(
        storage,
        config,
        &file_path,
        OpenFlags::new(OpenFlags::WRONLY | OpenFlags::CREAT | OpenFlags::EXCL),
    )?;
    lfs.file_close(storage, config, file)?;
    lfs.remove(storage, config, &file_path)?;
    lfs.remove(storage, config, dir_name)?;
    lfs.unmount(storage, config)?;
    Ok(())
}

fn dir_entry_names<B: BlockDevice>(
    lfs: &mut LittleFs,
    bd: &B,
    config: &Config,
    path: &str,
) -> Result<Vec<String>, Error> {
    let mut dir: Dir = lfs.dir_open(bd, config, path)?;
    let mut info = Info::new(FileType::Reg, 0);

    let _ = lfs.dir_read(bd, config, &mut dir, &mut info)?;
    let _ = lfs.dir_read(bd, config, &mut dir, &mut info)?;

    let mut names = Vec::new();
    loop {
        let n = lfs.dir_read(bd, config, &mut dir, &mut info)?;
        if n == 0 {
            break;
        }
        names.push(info.name().unwrap().to_string());
    }
    Ok(names)
}
