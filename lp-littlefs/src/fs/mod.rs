//! LittleFS filesystem implementation.

mod dir;
mod format;
mod metadata;
mod mount;
mod path;

use crate::block::BlockDevice;
use crate::config::Config;
use crate::error::Error;
use crate::info::{FsInfo, Info};

pub struct LittleFs {
    mounted: Option<mount::MountState>,
}

impl Default for LittleFs {
    fn default() -> Self {
        Self::new()
    }
}

impl LittleFs {
    pub fn new() -> Self {
        Self { mounted: None }
    }

    pub fn format<B: BlockDevice>(&mut self, bd: &B, config: &Config) -> Result<(), Error> {
        format::format(bd, config)
    }

    pub fn mount<B: BlockDevice>(&mut self, bd: &B, config: &Config) -> Result<(), Error> {
        let state = mount::mount(bd, config)?;
        self.mounted = Some(state);
        Ok(())
    }

    pub fn unmount(&mut self) -> Result<(), Error> {
        self.mounted = None;
        Ok(())
    }

    fn require_mounted(&self) -> Result<&mount::MountState, Error> {
        self.mounted.as_ref().ok_or(Error::Badf)
    }

    pub fn fs_stat<B: BlockDevice>(&self, _bd: &B, _config: &Config) -> Result<FsInfo, Error> {
        let state = self.require_mounted()?;
        Ok(FsInfo {
            disk_version: state.disk_version,
            block_size: state.block_size,
            block_count: state.block_count,
            name_max: state.name_max,
            file_max: state.file_max,
            attr_max: state.attr_max,
        })
    }

    pub fn stat<B: BlockDevice>(&self, bd: &B, config: &Config, path: &str) -> Result<Info, Error> {
        let state = self.require_mounted()?;
        let (dir, id) = path::dir_find(bd, config, state.root, path, state.name_max)?;

        if id == 0x3ff {
            let mut info = Info::new(crate::info::FileType::Dir, 0);
            info.set_name(b"/");
            return Ok(info);
        }

        let info = metadata::get_entry_info(&dir, id, state.name_max)?;

        if path.ends_with('/') && info.typ != crate::info::FileType::Dir {
            return Err(Error::NotDir);
        }

        Ok(info)
    }

    pub fn dir_open<B: BlockDevice>(
        &self,
        bd: &B,
        config: &Config,
        path: &str,
    ) -> Result<Dir, Error> {
        let state = self.require_mounted()?;
        let (dir, id) = path::dir_find(bd, config, state.root, path, state.name_max)?;

        if id != 0x3ff {
            let info = metadata::get_entry_info(&dir, id, state.name_max)?;
            if info.typ != crate::info::FileType::Dir {
                return Err(Error::NotDir);
            }
        }

        let pair = if id == 0x3ff {
            state.root
        } else {
            let bytes = metadata::get_struct(&dir, id)?;
            [
                u32::from_le_bytes(bytes[0..4].try_into().unwrap()),
                u32::from_le_bytes(bytes[4..8].try_into().unwrap()),
            ]
        };

        let md = metadata::fetch_metadata_pair(bd, config, pair)?;
        let is_root = pair[0] == 0 || pair[0] == 1;

        Ok(Dir {
            head: pair,
            mdir: md,
            id: if is_root { 1 } else { 0 },
            pos: 0,
            is_root,
            name_max: state.name_max,
        })
    }

    pub fn dir_read<B: BlockDevice>(
        &self,
        bd: &B,
        _config: &Config,
        dir: &mut Dir,
        info: &mut Info,
    ) -> Result<u32, Error> {
        self.require_mounted()?;
        dir::dir_read(bd, _config, dir, info, dir.name_max)
    }

    pub fn dir_close(&self, _dir: Dir) -> Result<(), Error> {
        Ok(())
    }
}

/// Open directory handle for iteration.
#[allow(dead_code)]
pub struct Dir {
    pub(crate) head: [u32; 2],
    pub(crate) mdir: metadata::MdDir,
    pub(crate) id: u16,
    pub(crate) pos: u32,
    pub(crate) is_root: bool,
    pub(crate) name_max: u32,
}
