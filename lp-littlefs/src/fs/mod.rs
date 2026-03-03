//! LittleFS filesystem implementation.

mod alloc;
mod bdcache;
mod commit;
mod consistent;
mod ctz;
#[cfg(feature = "trace")]
mod debug;
mod dir;
mod file;
mod format;
mod gstate;
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

use self::bdcache::BdContext;

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

    pub fn unmount<B: BlockDevice>(&mut self, bd: &B, config: &Config) -> Result<(), Error> {
        if let Some(state) = &self.mounted {
            trace!("unmount bd_sync then drop caches");
            bdcache::bd_sync(bd, config, &state.rcache, &state.pcache)?;
        }
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

    /// Garbage collection: force consistency, compact metadata pairs exceeding
    /// compact_thresh, refill lookahead buffer. Per lfs_fs_gc (lfs.h:752).
    pub fn fs_gc<B: BlockDevice>(&mut self, bd: &B, config: &Config) -> Result<(), Error> {
        let state = self.require_mounted_mut()?;
        let ctx = BdContext::new(bd, config, &state.rcache, &state.pcache);

        consistent::force_consistency(
            &ctx,
            &mut state.root,
            &mut state.gstate,
            &mut state.gdisk,
            &mut state.gdelta,
            &mut state.lookahead,
            state.name_max,
            state.block_count,
            state.file_max,
            state.attr_max,
        )?;

        let block_size = state.block_size as usize;
        let prog_size = config.prog_size as usize;
        let compact_thresh = if config.compact_thresh < 0 {
            block_size
        } else if config.compact_thresh == 0 {
            block_size - block_size / 8
        } else {
            config.compact_thresh as usize
        };

        if config.compact_thresh >= 0 && (config.compact_thresh as usize) < block_size - prog_size {
            let mut tail = state.root;
            while tail[0] != 0xffff_ffff || tail[1] != 0xffff_ffff {
                let mut dir = metadata::fetch_metadata_pair(&ctx, tail)?;
                let needs_compact = !dir.erased || dir.off > compact_thresh;
                if needs_compact {
                    dir.erased = false;
                    commit::dir_orphaningcommit(
                        &ctx,
                        &mut dir,
                        &[],
                        &mut state.root,
                        &mut state.lookahead,
                        state.name_max,
                        &state.gstate,
                        &mut state.gdisk,
                        &mut state.gdelta,
                        false,
                    )?;
                }
                tail = dir.tail;
            }
        }

        let lookahead_bits = config.lookahead_size.saturating_mul(8);
        let lookahead_full = state.lookahead.size >= lookahead_bits.min(state.block_count);
        if !lookahead_full {
            alloc::alloc_scan(&ctx, state.root, &mut state.lookahead)?;
        }

        Ok(())
    }

    /// Traverse all used blocks. Calls `cb` for each block.
    /// Per lfs_fs_traverse (lfs.h:519).
    pub fn fs_traverse<B: BlockDevice, F>(
        &self,
        bd: &B,
        config: &Config,
        cb: F,
    ) -> Result<(), Error>
    where
        F: FnMut(u32) -> Result<(), Error>,
    {
        let state = self.require_mounted()?;
        let ctx = BdContext::new(bd, config, &state.rcache, &state.pcache);
        traverse::fs_traverse(&ctx, state.root, false, cb)
    }

    /// Number of allocated blocks. Per lfs_fs_size (lfs.h:510).
    pub fn fs_size<B: BlockDevice>(&self, bd: &B, config: &Config) -> Result<i64, Error> {
        let mut count: i64 = 0;
        self.fs_traverse(bd, config, |_block| {
            count += 1;
            Ok(())
        })?;
        Ok(count)
    }

    /// Adjust orphan count. Per lfs_fs_preporphans. For testing power-loss paths.
    pub fn fs_preporphans(&mut self, delta: i8) -> Result<(), Error> {
        let state = self.require_mounted_mut()?;
        gstate::preporphans(&mut state.gstate, delta)
    }

    /// True if gstate has orphan count. For testing. Per lfs_gstate_hasorphans.
    pub fn fs_has_orphans<B: BlockDevice>(&self, _bd: &B, _config: &Config) -> Result<bool, Error> {
        let state = self.require_mounted()?;
        Ok(state.gstate.hasorphans())
    }

    /// Dump root metadata state and entry list. For debugging. Requires `trace` feature.
    #[cfg(feature = "trace")]
    pub fn fs_debug_dump<B: BlockDevice>(
        &self,
        bd: &B,
        config: &Config,
    ) -> Result<::alloc::string::String, Error> {
        let state = self.require_mounted()?;
        let ctx = BdContext::new(bd, config, &state.rcache, &state.pcache);
        let dir = metadata::fetch_metadata_pair(&ctx, state.root)?;
        debug::fs_debug_dump_impl(&dir, state.name_max)
    }

    /// Deorphan, complete moves, persist gstate. Per lfs_fs_mkconsistent (lfs.h:529).
    pub fn fs_mkconsistent<B: BlockDevice>(
        &mut self,
        bd: &B,
        config: &Config,
    ) -> Result<(), Error> {
        let state = self.require_mounted_mut()?;
        let ctx = BdContext::new(bd, config, &state.rcache, &state.pcache);

        consistent::force_consistency(
            &ctx,
            &mut state.root,
            &mut state.gstate,
            &mut state.gdisk,
            &mut state.gdelta,
            &mut state.lookahead,
            state.name_max,
            state.block_count,
            state.file_max,
            state.attr_max,
        )?;

        // Per upstream: if gdisk != gstate, commit root with empty attrs.
        // The shared commit path adds MOVESTATE internally.
        let mut delta = gstate::GState::zero();
        delta.xor(&state.gdisk);
        delta.xor(&state.gstate);
        if !delta.iszero() {
            let mut root_dir = metadata::fetch_metadata_pair(&ctx, state.root)?;
            commit::dir_orphaningcommit(
                &ctx,
                &mut root_dir,
                &[],
                &mut state.root,
                &mut state.lookahead,
                state.name_max,
                &state.gstate,
                &mut state.gdisk,
                &mut state.gdelta,
                true, // skip_dir_adjust for explicit persist
            )?;
        }

        trace!("fs_mkconsistent done, bd_sync");
        bdcache::bd_sync(bd, config, &state.rcache, &state.pcache)?;
        Ok(())
    }

    pub fn stat<B: BlockDevice>(&self, bd: &B, config: &Config, path: &str) -> Result<Info, Error> {
        trace!("stat path={:?}", path);
        let state = self.require_mounted()?;
        let ctx = BdContext::new(bd, config, &state.rcache, &state.pcache);
        let (dir, id) = path::dir_find(&ctx, state.root, path, state.name_max)?;
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
        trace!("dir_open path={:?}", path);
        let state = self.require_mounted()?;
        let ctx = BdContext::new(bd, config, &state.rcache, &state.pcache);
        let (dir, id) = path::dir_find(&ctx, state.root, path, state.name_max)?;

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

        trace!("dir_open fetch_metadata_pair pair={:?}", pair);
        let md = metadata::fetch_metadata_pair(&ctx, pair)?;
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
        config: &Config,
        dir: &mut Dir,
        info: &mut Info,
    ) -> Result<u32, Error> {
        let state = self.require_mounted()?;
        let ctx = BdContext::new(bd, config, &state.rcache, &state.pcache);
        dir::dir_read(&ctx, dir, info, dir.name_max)
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
        let ctx = BdContext::new(bd, config, &state.rcache, &state.pcache);
        if (flags.contains(crate::info::OpenFlags::WRONLY)
            || flags.contains(crate::info::OpenFlags::RDWR))
            && !flags.contains(crate::info::OpenFlags::CREAT)
        {
            consistent::force_consistency(
                &ctx,
                &mut state.root,
                &mut state.gstate,
                &mut state.gdisk,
                &mut state.gdelta,
                &mut state.lookahead,
                state.name_max,
                state.block_count,
                state.file_max,
                state.attr_max,
            )?;
        }
        file::File::open(
            &ctx,
            &mut state.root,
            &mut state.lookahead,
            path,
            state.name_max,
            state.inline_max,
            flags,
            &state.gstate,
            &mut state.gdisk,
            &mut state.gdelta,
        )
    }

    pub fn file_read<B: BlockDevice>(
        &self,
        bd: &B,
        config: &Config,
        file: &mut file::File,
        buf: &mut [u8],
    ) -> Result<usize, Error> {
        let state = self.require_mounted()?;
        let ctx = BdContext::new(bd, config, &state.rcache, &state.pcache);
        file.read(&ctx, buf)
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
        let ctx = BdContext::new(bd, config, &state.rcache, &state.pcache);
        file.write(
            &ctx,
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
        let ctx = BdContext::new(bd, config, &state.rcache, &state.pcache);
        file.sync(
            &ctx,
            &mut state.root,
            &mut state.lookahead,
            state.name_max,
            &state.gstate,
            &mut state.gdisk,
            &mut state.gdelta,
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
        let ctx = BdContext::new(bd, config, &state.rcache, &state.pcache);
        file.truncate(
            &ctx,
            &mut state.root,
            &mut state.lookahead,
            state.name_max,
            size,
            state.inline_max,
            &state.gstate,
            &mut state.gdisk,
            &mut state.gdelta,
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
        let ctx = BdContext::new(bd, config, &state.rcache, &state.pcache);
        file.close(
            &ctx,
            &mut state.root,
            &mut state.lookahead,
            state.name_max,
            &state.gstate,
            &mut state.gdisk,
            &mut state.gdelta,
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
        let ctx = BdContext::new(bd, config, &state.rcache, &state.pcache);
        consistent::force_consistency(
            &ctx,
            &mut state.root,
            &mut state.gstate,
            &mut state.gdisk,
            &mut state.gdelta,
            &mut state.lookahead,
            state.name_max,
            state.block_count,
            state.file_max,
            state.attr_max,
        )?;
        let (cwd, id, name) = path::dir_find_for_create(&ctx, state.root, path, state.name_max)?;
        trace!("mkdir cwd.pair={:?} id={} name={:?}", cwd.pair, id, name);

        let name_len = name.len();
        if name_len > state.name_max as usize {
            return Err(Error::Nametoolong);
        }

        state.lookahead.alloc_ckpoint(state.block_count);

        let mut new_dir = commit::dir_alloc(&ctx, state.root, &mut state.lookahead)?;

        let mut pred = cwd.clone();
        while pred.split {
            pred = metadata::fetch_metadata_pair(&ctx, pred.tail)?;
        }

        let pred_tail = pred.tail;
        commit::dir_orphaningcommit(
            &ctx,
            &mut new_dir,
            &[commit::CommitAttr::soft_tail(pred_tail)],
            &mut state.root,
            &mut state.lookahead,
            state.name_max,
            &state.gstate,
            &mut state.gdisk,
            &mut state.gdelta,
            false,
        )?;

        let new_pair = new_dir.pair;

        if cwd.split {
            let mut pred_mut = metadata::fetch_metadata_pair(&ctx, pred.pair)?;
            commit::dir_orphaningcommit(
                &ctx,
                &mut pred_mut,
                &[commit::CommitAttr::soft_tail(new_pair)],
                &mut state.root,
                &mut state.lookahead,
                state.name_max,
                &state.gstate,
                &mut state.gdisk,
                &mut state.gdelta,
                false,
            )?;
        }

        let mut cwd_mut = metadata::fetch_metadata_pair(&ctx, cwd.pair)?;
        let mut attrs: Vec<commit::CommitAttr> = Vec::new();
        attrs.push(commit::CommitAttr::create(id));
        attrs.push(commit::CommitAttr::name_dir(id, name.as_bytes()));
        attrs.push(commit::CommitAttr::dir_struct(id, new_pair));
        if !cwd_mut.split {
            attrs.push(commit::CommitAttr::soft_tail(new_pair));
        }
        commit::dir_orphaningcommit(
            &ctx,
            &mut cwd_mut,
            &attrs,
            &mut state.root,
            &mut state.lookahead,
            state.name_max,
            &state.gstate,
            &mut state.gdisk,
            &mut state.gdelta,
            false,
        )?;

        state.lookahead.alloc_ckpoint(state.block_count);

        trace!("mkdir done, bd_sync root={:?}", state.root);
        bdcache::bd_sync(bd, config, &state.rcache, &state.pcache)?;
        Ok(())
    }

    pub fn remove<B: BlockDevice>(
        &mut self,
        bd: &B,
        config: &Config,
        path: &str,
    ) -> Result<(), Error> {
        let state = self.require_mounted_mut()?;
        let ctx = BdContext::new(bd, config, &state.rcache, &state.pcache);
        consistent::force_consistency(
            &ctx,
            &mut state.root,
            &mut state.gstate,
            &mut state.gdisk,
            &mut state.gdelta,
            &mut state.lookahead,
            state.name_max,
            state.block_count,
            state.file_max,
            state.attr_max,
        )?;
        let (cwd, id) = path::dir_find(&ctx, state.root, path, state.name_max)?;

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
            let child = metadata::fetch_metadata_pair(&ctx, dir_pair)?;
            if child.count != 0 || child.split {
                return Err(Error::NotEmpty);
            }
        }

        let mut cwd_mut = metadata::fetch_metadata_pair(&ctx, cwd.pair)?;
        commit::dir_orphaningcommit(
            &ctx,
            &mut cwd_mut,
            &[commit::CommitAttr::delete(id)],
            &mut state.root,
            &mut state.lookahead,
            state.name_max,
            &state.gstate,
            &mut state.gdisk,
            &mut state.gdelta,
            false,
        )?;

        trace!("remove done, bd_sync");
        bdcache::bd_sync(bd, config, &state.rcache, &state.pcache)?;
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
        let ctx = BdContext::new(bd, config, &state.rcache, &state.pcache);
        consistent::force_consistency(
            &ctx,
            &mut state.root,
            &mut state.gstate,
            &mut state.gdisk,
            &mut state.gdelta,
            &mut state.lookahead,
            state.name_max,
            state.block_count,
            state.file_max,
            state.attr_max,
        )?;
        let (old_cwd, old_id) = path::dir_find(&ctx, state.root, old_path, state.name_max)?;
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

        // Rename dir into itself (littlefs#1162)
        if matches!(old_info.typ, crate::info::FileType::Dir)
            && path::path_is_descendant(old_path, new_path)
        {
            return Err(Error::Inval);
        }

        // Trailing slash on new path when source is file -> Notdir
        if matches!(old_info.typ, crate::info::FileType::Reg)
            && new_path.trim_end_matches('/') != new_path
        {
            return Err(Error::NotDir);
        }

        let (new_cwd, new_id, new_name, overwrite_dir_orphan) =
            match path::dir_find(&ctx, state.root, new_path, state.name_max) {
                Ok((cwd, id)) => {
                    let name = path::path_last_component(new_path).unwrap_or("").as_bytes();
                    let new_info = metadata::get_entry_info(&cwd, id, state.name_max)?;
                    if new_info.typ != old_info.typ {
                        return Err(if matches!(new_info.typ, crate::info::FileType::Dir) {
                            Error::IsDir
                        } else {
                            Error::NotDir
                        });
                    }
                    let same_pair =
                        old_cwd.pair[0] == cwd.pair[0] && old_cwd.pair[1] == cwd.pair[1];
                    if same_pair && id == old_id {
                        return Ok(());
                    }
                    let mut orphan = None;
                    if matches!(new_info.typ, crate::info::FileType::Dir) {
                        let prev_pair_bytes = metadata::get_struct(&cwd, id)?;
                        let prev_pair = [
                            u32::from_le_bytes(prev_pair_bytes[0..4].try_into().unwrap()),
                            u32::from_le_bytes(prev_pair_bytes[4..8].try_into().unwrap()),
                        ];
                        let prevdir = metadata::fetch_metadata_pair(&ctx, prev_pair)?;
                        if prevdir.count > 0 || prevdir.split {
                            return Err(Error::NotEmpty);
                        }
                        gstate::preporphans(&mut state.gstate, 1)?;
                        orphan = Some(prevdir);
                    }
                    (cwd, id, name, orphan)
                }
                Err(Error::Noent) => {
                    let (cwd, id, name) =
                        path::dir_find_for_create(&ctx, state.root, new_path, state.name_max)?;
                    (cwd, id, name.as_bytes(), None)
                }
                Err(e) => return Err(e),
            };

        if new_name.len() > state.name_max as usize {
            return Err(Error::Nametoolong);
        }

        let same_pair = old_cwd.pair[0] == new_cwd.pair[0] && old_cwd.pair[1] == new_cwd.pair[1];
        let mut newoldid = old_id;
        if same_pair && new_id <= old_id {
            newoldid = old_id + 1;
        }

        if !same_pair {
            gstate::prepmove(&mut state.gstate, newoldid, old_cwd.pair);
        }

        let mut new_cwd_mut = metadata::fetch_metadata_pair(&ctx, new_cwd.pair)?;

        let (delete_overwrite, delete_old) = if overwrite_dir_orphan.is_some() {
            (true, false)
        } else if same_pair {
            (false, true)
        } else {
            (false, false)
        };

        let inline_data = match old_info.typ {
            crate::info::FileType::Reg => {
                let (is_inline, _, _) = metadata::get_file_struct(&old_cwd, old_id)?;
                if is_inline {
                    let mut buf = Vec::new();
                    buf.resize(state.inline_max as usize, 0);
                    let n = metadata::get_inline_slice(&old_cwd, old_id, 0, &mut buf)?;
                    let mut data = Vec::new();
                    data.extend_from_slice(&buf[..n]);
                    Some(data)
                } else {
                    None
                }
            }
            _ => None,
        };

        let attrs: Vec<commit::CommitAttr> = {
            let mut v = Vec::new();
            if delete_overwrite {
                v.push(commit::CommitAttr::delete(new_id));
            }
            v.push(commit::CommitAttr::create(new_id));
            match old_info.typ {
                crate::info::FileType::Dir => {
                    v.push(commit::CommitAttr::name_dir(new_id, new_name));
                    v.push(commit::CommitAttr::dir_struct(new_id, dir_pair));
                }
                crate::info::FileType::Reg => {
                    let (is_inline, head, size) = metadata::get_file_struct(&old_cwd, old_id)?;
                    v.push(commit::CommitAttr::name_reg(new_id, new_name));
                    if is_inline {
                        v.push(commit::CommitAttr::inline_struct(
                            new_id,
                            inline_data.as_ref().unwrap(),
                        ));
                    } else {
                        v.push(commit::CommitAttr::ctz_struct(new_id, head, size as u32));
                    }
                }
            }
            if delete_old {
                v.push(commit::CommitAttr::delete(old_id));
            }
            v
        };

        commit::dir_orphaningcommit(
            &ctx,
            &mut new_cwd_mut,
            &attrs,
            &mut state.root,
            &mut state.lookahead,
            state.name_max,
            &state.gstate,
            &mut state.gdisk,
            &mut state.gdelta,
            false,
        )?;

        if !same_pair && state.gstate.hasmove() {
            gstate::prepmove(&mut state.gstate, 0x3ff, [0, 0]);
            let mut old_cwd_mut = metadata::fetch_metadata_pair(&ctx, old_cwd.pair)?;
            commit::dir_orphaningcommit(
                &ctx,
                &mut old_cwd_mut,
                &[commit::CommitAttr::delete(old_id)],
                &mut state.root,
                &mut state.lookahead,
                state.name_max,
                &state.gstate,
                &mut state.gdisk,
                &mut state.gdelta,
                false,
            )?;
        }

        if let Some(orphan) = overwrite_dir_orphan {
            gstate::preporphans(&mut state.gstate, -1)?;
            if let Some(mut pred) = parent::fs_pred(&ctx, state.root, orphan.pair)? {
                commit::dir_drop(
                    &ctx,
                    &mut pred,
                    &orphan,
                    &mut state.root,
                    &mut state.lookahead,
                    state.name_max,
                    &state.gstate,
                    &mut state.gdisk,
                    &mut state.gdelta,
                )?;
            }
        }

        trace!("rename commit done, syncing");
        bdcache::bd_sync(bd, config, &state.rcache, &state.pcache)?;
        Ok(())
    }
}

/// Open directory handle for iteration.
#[allow(dead_code)]
pub use file::File;

/// Create an inline file. Requires a formatted fs (not mounted).
/// Used by integration tests and for seeding filesystems.
#[doc(hidden)]
pub fn create_inline_file<B: BlockDevice>(
    bd: &B,
    config: &Config,
    path: &str,
    content: &[u8],
) -> Result<(), Error> {
    use core::cell::RefCell;

    let rcache = RefCell::new(bdcache::new_read_cache(config)?);
    let pcache = RefCell::new(bdcache::new_prog_cache(config)?);
    let ctx = BdContext::new(bd, config, &rcache, &pcache);

    let mut root = [0u32, 1];
    let (cwd, id, name) = path::dir_find_for_create(&ctx, root, path, 255)?;
    let mut cwd_mut = metadata::fetch_metadata_pair(&ctx, cwd.pair)?;
    let attrs = [
        commit::CommitAttr::create(id),
        commit::CommitAttr::name_reg(id, name.as_bytes()),
        commit::CommitAttr::inline_struct(id, content),
    ];
    let mut lookahead = alloc::Lookahead::new(config);
    let gstate = gstate::GState::zero();
    let mut gdisk = gstate::GState::zero();
    let mut gdelta = gstate::GState::zero();
    commit::dir_orphaningcommit(
        &ctx,
        &mut cwd_mut,
        &attrs,
        &mut root,
        &mut lookahead,
        255,
        &gstate,
        &mut gdisk,
        &mut gdelta,
        false,
    )?;
    bdcache::bd_sync(bd, config, &rcache, &pcache)
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

impl Dir {
    /// Metadata block pair for this directory. For power-loss simulation.
    #[doc(hidden)]
    pub fn pair(&self) -> [u32; 2] {
        self.head
    }

    /// Revision of the block we read from. For power-loss simulation.
    #[doc(hidden)]
    pub fn revision(&self) -> u32 {
        self.mdir.rev
    }
}
