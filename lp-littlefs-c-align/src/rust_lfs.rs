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
    let mut lfs = LittleFs::new();
    lfs.mount(storage, config)?;
    let names = dir_entry_names(&mut lfs, storage, config, "/")?;
    lfs.unmount(storage, config)?;
    Ok(names)
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
