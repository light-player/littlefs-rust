//! LittleFS filesystem implementation.

mod alloc;
mod commit;
mod ctz;
mod dir;
mod file;
mod format;
mod metadata;
mod mount;
mod parent;
mod path;
mod traverse;

use ::alloc::vec::Vec;

use crate::block::BlockDevice;
use crate::config::Config;
use crate::error::Error;
use crate::info::{FsInfo, Info};
use crate::trace;

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

    fn require_mounted_mut(&mut self) -> Result<&mut mount::MountState, Error> {
        self.mounted.as_mut().ok_or(Error::Badf)
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
        trace!("stat path={:?}", path);
        let state = self.require_mounted()?;
        let (dir, id) = path::dir_find(bd, config, state.root, path, state.name_max)?;
        trace!("stat dir_find returned id={}", id);

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

    pub fn file_open<B: BlockDevice>(
        &mut self,
        bd: &B,
        config: &Config,
        path: &str,
        flags: crate::info::OpenFlags,
    ) -> Result<file::File, Error> {
        let state = self.require_mounted_mut()?;
        file::File::open(
            bd,
            config,
            &mut state.root,
            &mut state.lookahead,
            path,
            state.name_max,
            state.inline_max,
            flags,
        )
    }

    pub fn file_read<B: BlockDevice>(
        &self,
        bd: &B,
        config: &Config,
        file: &mut file::File,
        buf: &mut [u8],
    ) -> Result<usize, Error> {
        self.require_mounted()?;
        file.read(bd, config, buf)
    }

    pub fn file_seek<B: BlockDevice>(
        &self,
        _bd: &B,
        _config: &Config,
        file: &mut file::File,
        off: i64,
        whence: crate::info::SeekWhence,
    ) -> Result<i64, Error> {
        self.require_mounted()?;
        file.seek(off, whence)
    }

    pub fn file_tell(&self, file: &file::File) -> Result<i64, Error> {
        self.require_mounted()?;
        Ok(file.tell())
    }

    pub fn file_size(&self, file: &file::File) -> Result<i64, Error> {
        self.require_mounted()?;
        Ok(file.size())
    }

    pub fn file_write<B: BlockDevice>(
        &mut self,
        bd: &B,
        config: &Config,
        file: &mut file::File,
        data: &[u8],
    ) -> Result<usize, Error> {
        let state = self.require_mounted_mut()?;
        file.write(
            bd,
            config,
            state.root,
            &mut state.lookahead,
            state.inline_max,
            state.file_max,
            data,
        )
    }

    pub fn file_sync<B: BlockDevice>(
        &mut self,
        bd: &B,
        config: &Config,
        file: &mut file::File,
    ) -> Result<(), Error> {
        let state = self.require_mounted_mut()?;
        file.sync(
            bd,
            config,
            &mut state.root,
            &mut state.lookahead,
            state.name_max,
        )
    }

    pub fn file_truncate<B: BlockDevice>(
        &mut self,
        bd: &B,
        config: &Config,
        file: &mut file::File,
        size: u64,
    ) -> Result<(), Error> {
        let state = self.require_mounted_mut()?;
        file.truncate(
            bd,
            config,
            &mut state.root,
            &mut state.lookahead,
            state.name_max,
            size,
            state.inline_max,
        )
    }

    pub fn file_rewind<B: BlockDevice>(
        &mut self,
        _bd: &B,
        _config: &Config,
        file: &mut file::File,
    ) -> Result<(), Error> {
        self.require_mounted()?;
        file.rewind()
    }

    pub fn file_close<B: BlockDevice>(
        &mut self,
        bd: &B,
        config: &Config,
        file: file::File,
    ) -> Result<(), Error> {
        let state = self.require_mounted_mut()?;
        file.close(
            bd,
            config,
            &mut state.root,
            &mut state.lookahead,
            state.name_max,
        )
    }

    pub fn mkdir<B: BlockDevice>(
        &mut self,
        bd: &B,
        config: &Config,
        path: &str,
    ) -> Result<(), Error> {
        trace!("mkdir path={:?}", path);
        let state = self.require_mounted_mut()?;
        let (cwd, id, name) =
            path::dir_find_for_create(bd, config, state.root, path, state.name_max)?;
        trace!("mkdir cwd.pair={:?} id={} name={:?}", cwd.pair, id, name);

        let name_len = name.len();
        if name_len > state.name_max as usize {
            return Err(Error::Nametoolong);
        }

        state.lookahead.alloc_ckpoint(state.block_count);

        let mut new_dir = commit::dir_alloc(bd, config, state.root, &mut state.lookahead)?;

        let mut pred = cwd.clone();
        while pred.split {
            pred = metadata::fetch_metadata_pair(bd, config, pred.tail)?;
        }

        let pred_tail = pred.tail;
        commit::dir_orphaningcommit(
            bd,
            config,
            &mut new_dir,
            &[commit::CommitAttr::soft_tail(pred_tail)],
            &mut state.root,
            &mut state.lookahead,
            state.name_max,
        )?;

        let new_pair = new_dir.pair;

        if cwd.split {
            let mut pred_mut = metadata::fetch_metadata_pair(bd, config, pred.pair)?;
            commit::dir_orphaningcommit(
                bd,
                config,
                &mut pred_mut,
                &[commit::CommitAttr::soft_tail(new_pair)],
                &mut state.root,
                &mut state.lookahead,
                state.name_max,
            )?;
        }

        let mut cwd_mut = metadata::fetch_metadata_pair(bd, config, cwd.pair)?;
        let mut attrs: Vec<commit::CommitAttr> = Vec::new();
        attrs.push(commit::CommitAttr::create(id));
        attrs.push(commit::CommitAttr::name_dir(id, name.as_bytes()));
        attrs.push(commit::CommitAttr::dir_struct(id, new_pair));
        if !cwd_mut.split {
            attrs.push(commit::CommitAttr::soft_tail(new_pair));
        }
        commit::dir_orphaningcommit(
            bd,
            config,
            &mut cwd_mut,
            &attrs,
            &mut state.root,
            &mut state.lookahead,
            state.name_max,
        )?;

        state.lookahead.alloc_ckpoint(state.block_count);

        bd.sync()?;
        Ok(())
    }

    pub fn remove<B: BlockDevice>(
        &mut self,
        bd: &B,
        config: &Config,
        path: &str,
    ) -> Result<(), Error> {
        let state = self.require_mounted_mut()?;
        let (cwd, id) = path::dir_find(bd, config, state.root, path, state.name_max)?;

        if id == 0x3ff {
            return Err(Error::Inval);
        }

        let info = metadata::get_entry_info(&cwd, id, state.name_max)?;
        if info.typ == crate::info::FileType::Dir {
            let pair = metadata::get_struct(&cwd, id)?;
            let dir_pair = [
                u32::from_le_bytes(pair[0..4].try_into().unwrap()),
                u32::from_le_bytes(pair[4..8].try_into().unwrap()),
            ];
            let child = metadata::fetch_metadata_pair(bd, config, dir_pair)?;
            if child.count != 0 || child.split {
                return Err(Error::NotEmpty);
            }
        }

        let mut cwd_mut = metadata::fetch_metadata_pair(bd, config, cwd.pair)?;
        commit::dir_orphaningcommit(
            bd,
            config,
            &mut cwd_mut,
            &[commit::CommitAttr::delete(id)],
            &mut state.root,
            &mut state.lookahead,
            state.name_max,
        )?;

        bd.sync()?;
        Ok(())
    }

    pub fn rename<B: BlockDevice>(
        &mut self,
        bd: &B,
        config: &Config,
        old_path: &str,
        new_path: &str,
    ) -> Result<(), Error> {
        trace!("rename old={:?} new={:?}", old_path, new_path);
        let state = self.require_mounted_mut()?;
        let (old_cwd, old_id) = path::dir_find(bd, config, state.root, old_path, state.name_max)?;
        trace!("rename old_cwd.pair={:?} old_id={}", old_cwd.pair, old_id);

        if old_id == 0x3ff {
            return Err(Error::Inval);
        }

        let old_info = metadata::get_entry_info(&old_cwd, old_id, state.name_max)?;
        let old_pair = metadata::get_struct(&old_cwd, old_id)?;
        let dir_pair = [
            u32::from_le_bytes(old_pair[0..4].try_into().unwrap()),
            u32::from_le_bytes(old_pair[4..8].try_into().unwrap()),
        ];

        let (new_cwd, new_id, new_name) =
            path::dir_find_for_create(bd, config, state.root, new_path, state.name_max)?;
        trace!(
            "rename new_cwd.pair={:?} new_id={} new_name={:?}",
            new_cwd.pair,
            new_id,
            new_name
        );

        if new_name.len() > state.name_max as usize {
            return Err(Error::Nametoolong);
        }

        if old_info.typ != crate::info::FileType::Dir {
            return Err(Error::Inval);
        }

        let same_pair = old_cwd.pair[0] == new_cwd.pair[0] && old_cwd.pair[1] == new_cwd.pair[1];
        trace!(
            "rename same_pair={} attrs: create={} name_dir={} dir_struct delete={}",
            same_pair,
            new_id,
            new_id,
            old_id
        );
        if !same_pair {
            return Err(Error::Inval);
        }

        let attrs = [
            commit::CommitAttr::create(new_id),
            commit::CommitAttr::name_dir(new_id, new_name.as_bytes()),
            commit::CommitAttr::dir_struct(new_id, dir_pair),
            commit::CommitAttr::delete(old_id),
        ];

        let mut new_cwd_mut = metadata::fetch_metadata_pair(bd, config, new_cwd.pair)?;
        commit::dir_orphaningcommit(
            bd,
            config,
            &mut new_cwd_mut,
            &attrs,
            &mut state.root,
            &mut state.lookahead,
            state.name_max,
        )?;
        trace!("rename commit done, syncing");

        bd.sync()?;
        Ok(())
    }
}

/// Open directory handle for iteration.
#[allow(dead_code)]
pub use file::File;

/// Create an inline file. Requires a formatted fs (not mounted).
/// Used by integration tests and for seeding filesystems.
pub fn create_inline_file<B: BlockDevice>(
    bd: &B,
    config: &Config,
    path: &str,
    content: &[u8],
) -> Result<(), Error> {
    let mut root = [0u32, 1];
    let (cwd, id, name) = path::dir_find_for_create(bd, config, root, path, 255)?;
    let mut cwd_mut = metadata::fetch_metadata_pair(bd, config, cwd.pair)?;
    let attrs = [
        commit::CommitAttr::create(id),
        commit::CommitAttr::name_reg(id, name.as_bytes()),
        commit::CommitAttr::inline_struct(id, content),
    ];
    let mut lookahead = alloc::Lookahead::new(config);
    commit::dir_orphaningcommit(
        bd,
        config,
        &mut cwd_mut,
        &attrs,
        &mut root,
        &mut lookahead,
        255,
    )?;
    bd.sync()
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
