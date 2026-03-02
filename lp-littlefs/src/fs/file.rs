//! File operations: open, read, write, seek, tell, size, sync, truncate, rewind, close.
//!
//! Per lfs.c lfs_file_opencfg_, lfs_file_flushedread, lfs_file_flushedwrite, etc.

use crate::block::BlockDevice;
use crate::config::Config;
use crate::error::Error;
use crate::info::OpenFlags;

use ::alloc::vec;
use ::alloc::vec::Vec;

use super::alloc;
use super::commit;
use super::ctz;
use super::gstate::GState;
use super::metadata;
use super::path;

const BLOCK_INLINE: u32 = 0xffff_fffe;
const BLOCK_NULL: u32 = 0xffff_ffff;

/// Open file handle.
pub struct File {
    id: u16,
    mdir: metadata::MdDir,
    ctz_head: u32,
    ctz_size: u64,
    inline: bool,
    pos: u64,
    block: u32,
    off: u32,
    block_size: u32,
    /// Buffer for inline file content when writing.
    inline_buffer: Vec<u8>,
    /// True if we have unflushed writes (CTZ) or uncommitted data (inline).
    dirty: bool,
    /// True if we are currently in a "writing" block (CTZ) and have prog data.
    writing: bool,
}

impl File {
    /// Open a file. Supports RDONLY, WRONLY, RDWR, CREAT, EXCL, TRUNC, APPEND.
    pub fn open<B: BlockDevice>(
        bd: &B,
        config: &Config,
        root: &mut [u32; 2],
        lookahead: &mut alloc::Lookahead,
        path: &str,
        name_max: u32,
        inline_max: u32,
        flags: OpenFlags,
        gstate: &GState,
        gdisk: &mut GState,
        gdelta: &mut GState,
    ) -> Result<Self, Error> {
        let can_write = flags.contains(OpenFlags::WRONLY) || flags.contains(OpenFlags::RDWR);
        let is_rdonly = !can_write;

        let (dir, id, created) = if is_rdonly {
            let (dir, id) = path::dir_find(bd, config, *root, path, name_max)?;
            if id == 0x3ff {
                return Err(Error::IsDir);
            }
            let info = metadata::get_entry_info(&dir, id, name_max)?;
            if info.typ != crate::info::FileType::Reg {
                return Err(Error::IsDir);
            }
            (dir, id, false)
        } else {
            match path::dir_find(bd, config, *root, path, name_max) {
                Ok((dir, id)) => {
                    if id == 0x3ff {
                        return Err(Error::IsDir);
                    }
                    let info = metadata::get_entry_info(&dir, id, name_max)?;
                    if info.typ != crate::info::FileType::Reg {
                        return Err(Error::IsDir);
                    }
                    if flags.contains(OpenFlags::EXCL) {
                        return Err(Error::Exist);
                    }
                    (dir, id, false)
                }
                Err(Error::Noent) => {
                    if !flags.contains(OpenFlags::CREAT) {
                        return Err(Error::Noent);
                    }
                    let (cwd, id, name) =
                        path::dir_find_for_create(bd, config, *root, path, name_max)?;
                    if name.len() > name_max as usize {
                        return Err(Error::Nametoolong);
                    }
                    let mut cwd_mut = metadata::fetch_metadata_pair(bd, config, cwd.pair)?;
                    let attrs = [
                        commit::CommitAttr::create(id),
                        commit::CommitAttr::name_reg(id, name.as_bytes()),
                        commit::CommitAttr::inline_struct(id, &[]),
                    ];
                    commit::dir_orphaningcommit(
                        bd,
                        config,
                        &mut cwd_mut,
                        &attrs,
                        root,
                        lookahead,
                        name_max,
                        gstate,
                        gdisk,
                        gdelta,
                        false,
                    )?;
                    bd.sync()?;
                    let dir = metadata::fetch_metadata_pair(bd, config, cwd.pair)?;
                    (dir, id, true)
                }
                Err(e) => return Err(e),
            }
        };

        let (inline_, head, size) = if created || (can_write && flags.contains(OpenFlags::TRUNC)) {
            (true, BLOCK_INLINE, 0u64)
        } else {
            metadata::get_file_struct(&dir, id)?
        };

        let mut pos = 0u64;
        if can_write && flags.contains(OpenFlags::APPEND) {
            pos = size;
        }

        let (block, off) = if inline_ {
            (BLOCK_INLINE, 0u32)
        } else {
            (BLOCK_NULL, 0)
        };

        let mut inline_buffer = Vec::new();
        if inline_ && can_write {
            inline_buffer.reserve(inline_max as usize);
            if size > 0 && !created && !flags.contains(OpenFlags::TRUNC) {
                let mut buf = vec![0u8; size as usize];
                let n = metadata::get_inline_slice(&dir, id, 0, &mut buf)?;
                inline_buffer.extend_from_slice(&buf[..n]);
            }
        }

        let dirty = can_write && (created || flags.contains(OpenFlags::TRUNC));
        let writing = false;

        Ok(Self {
            id,
            mdir: dir,
            ctz_head: head,
            ctz_size: size,
            inline: inline_,
            pos,
            block,
            off,
            block_size: config.block_size,
            inline_buffer,
            dirty,
            writing,
        })
    }

    /// Read bytes from file. Returns 0 at EOF.
    pub fn read<B: BlockDevice>(
        &mut self,
        bd: &B,
        config: &Config,
        buf: &mut [u8],
    ) -> Result<usize, Error> {
        let size = self.size_for_read();
        if self.pos >= size {
            return Ok(0);
        }

        let to_read = (buf.len() as u64).min(size - self.pos) as usize;
        if to_read == 0 {
            return Ok(0);
        }

        if self.inline {
            if self.dirty {
                let offset = self.pos as usize;
                let avail = self.inline_buffer.len().saturating_sub(offset);
                let n = to_read.min(avail);
                buf[..n].copy_from_slice(&self.inline_buffer[offset..][..n]);
                self.pos += n as u64;
                return Ok(n);
            }
            let n = metadata::get_inline_slice(
                &self.mdir,
                self.id,
                self.pos as usize,
                &mut buf[..to_read],
            )?;
            self.pos += n as u64;
            return Ok(n);
        }

        let mut buf_pos = 0;

        while buf_pos < to_read {
            let need_block = self.block == BLOCK_NULL || self.off >= self.block_size;
            if need_block {
                let (block, off) =
                    ctz::ctz_find(bd, config, self.ctz_head, self.ctz_size, self.pos)?;
                self.block = block;
                self.off = off;
            }

            let avail_in_block = (self.block_size - self.off) as usize;
            let want = (to_read - buf_pos).min(avail_in_block);

            bd.read(self.block, self.off, &mut buf[buf_pos..buf_pos + want])?;

            self.pos += want as u64;
            self.off += want as u32;
            buf_pos += want;
        }

        Ok(buf_pos)
    }

    fn size_for_read(&self) -> u64 {
        if self.inline && self.dirty {
            self.inline_buffer.len() as u64
        } else {
            self.ctz_size
        }
    }

    /// Write bytes to file. Buffered until sync.
    #[allow(clippy::too_many_arguments)]
    pub fn write<B: BlockDevice>(
        &mut self,
        bd: &B,
        config: &Config,
        root: [u32; 2],
        lookahead: &mut alloc::Lookahead,
        inline_max: u32,
        file_max: u32,
        data: &[u8],
    ) -> Result<usize, Error> {
        if self.pos + data.len() as u64 > file_max as u64 {
            return Err(Error::Fbig);
        }

        if self.inline {
            let new_size = (self.pos + data.len() as u64) as usize;
            if new_size as u32 > inline_max {
                // Copy data into buffer before outline so we migrate the new content,
                // not the old. Outline will copy inline_buffer to the first CTZ block.
                if new_size > self.inline_buffer.len() {
                    self.inline_buffer.resize(new_size, 0);
                }
                let pos = self.pos as usize;
                self.inline_buffer[pos..][..data.len()].copy_from_slice(data);
                self.pos += data.len() as u64;
                self.outline(bd, config, root, lookahead)?;
                return Ok(data.len());
            }
            if new_size > self.inline_buffer.len() {
                self.inline_buffer.resize(new_size, 0);
            }
            let pos = self.pos as usize;
            self.inline_buffer[pos..][..data.len()].copy_from_slice(data);
            self.pos += data.len() as u64;
            self.dirty = true;
            return Ok(data.len());
        }

        if self.pos > self.ctz_size {
            let zeros = (self.pos - self.ctz_size) as usize;
            for _ in 0..zeros {
                self.write_one_ctz(bd, config, root, lookahead, 0)?;
            }
        }

        let mut written = 0;
        for byte in data {
            self.write_one_ctz(bd, config, root, lookahead, *byte)?;
            written += 1;
        }
        Ok(written)
    }

    fn outline<B: BlockDevice>(
        &mut self,
        bd: &B,
        config: &Config,
        root: [u32; 2],
        lookahead: &mut alloc::Lookahead,
    ) -> Result<(), Error> {
        lookahead.alloc_ckpoint(config.block_count);

        let block = alloc::alloc(bd, config, root, lookahead)?;
        bd.erase(block)?;

        let size = self.inline_buffer.len();
        for (i, &b) in self.inline_buffer.iter().enumerate() {
            bd.prog(block, i as u32, &[b])?;
        }

        self.ctz_head = block;
        self.ctz_size = size as u64;
        self.block = block;
        self.off = size as u32;
        self.inline = false;
        self.inline_buffer.clear();
        self.dirty = true;
        self.writing = true;
        Ok(())
    }

    fn write_one_ctz<B: BlockDevice>(
        &mut self,
        bd: &B,
        config: &Config,
        root: [u32; 2],
        lookahead: &mut alloc::Lookahead,
        byte: u8,
    ) -> Result<(), Error> {
        let need_block = !self.writing || self.off >= self.block_size;
        if need_block {
            if !self.writing && self.pos == 0 && self.ctz_size == 0 {
                self.block = self.ctz_head;
                self.off = 0;
            } else if !self.writing && self.pos == 0 && self.ctz_size > 0 {
                let (block, off) = ctz::ctz_find(bd, config, self.ctz_head, self.ctz_size, 0)?;
                self.block = block;
                self.off = off;
            } else if !self.writing && self.pos > 0 {
                let (block, _) =
                    ctz::ctz_find(bd, config, self.ctz_head, self.ctz_size, self.pos - 1)?;
                self.block = block;
                self.off = self.pos as u32 % self.block_size;
                if self.off == 0 {
                    let mut noff = self.pos - 1;
                    let _ = ctz::ctz_index(config.block_size, &mut noff);
                    self.off = (noff + 1) as u32;
                }
            } else {
                lookahead.alloc_ckpoint(config.block_count);
                let (block, off) =
                    ctz::ctz_extend(bd, config, root, lookahead, self.block, self.pos)?;
                self.block = block;
                self.ctz_head = block;
                self.off = off;
            }
            self.writing = true;
        }

        bd.prog(self.block, self.off, &[byte])?;
        self.pos += 1;
        self.off += 1;
        self.ctz_size = self.ctz_size.max(self.pos);
        self.dirty = true;

        lookahead.alloc_ckpoint(config.block_count);
        Ok(())
    }

    /// Flush buffer to storage and commit metadata.
    pub fn sync<B: BlockDevice>(
        &mut self,
        bd: &B,
        config: &Config,
        root: &mut [u32; 2],
        lookahead: &mut alloc::Lookahead,
        name_max: u32,
        gstate: &GState,
        gdisk: &mut GState,
        gdelta: &mut GState,
    ) -> Result<(), Error> {
        if !self.dirty {
            return Ok(());
        }

        bd.sync()?;

        if self.inline {
            let mut dir = metadata::fetch_metadata_pair(bd, config, self.mdir.pair)?;
            commit::dir_orphaningcommit(
                bd,
                config,
                &mut dir,
                &[commit::CommitAttr::inline_struct(
                    self.id,
                    &self.inline_buffer,
                )],
                root,
                lookahead,
                name_max,
                gstate,
                gdisk,
                gdelta,
                false,
            )?;
        } else {
            let mut dir = metadata::fetch_metadata_pair(bd, config, self.mdir.pair)?;
            commit::dir_orphaningcommit(
                bd,
                config,
                &mut dir,
                &[commit::CommitAttr::ctz_struct(
                    self.id,
                    self.ctz_head,
                    self.ctz_size as u32,
                )],
                root,
                lookahead,
                name_max,
                gstate,
                gdisk,
                gdelta,
                false,
            )?;
        }

        bd.sync()?;
        self.mdir = metadata::fetch_metadata_pair(bd, config, self.mdir.pair)?;
        self.dirty = false;
        self.writing = false;
        Ok(())
    }

    /// Truncate file to given size.
    pub fn truncate<B: BlockDevice>(
        &mut self,
        bd: &B,
        config: &Config,
        root: &mut [u32; 2],
        lookahead: &mut alloc::Lookahead,
        name_max: u32,
        size: u64,
        inline_max: u32,
        gstate: &GState,
        gdisk: &mut GState,
        gdelta: &mut GState,
    ) -> Result<(), Error> {
        let old_pos = self.pos;
        let old_size = self.size_for_read();

        if size < old_size {
            if size <= inline_max as u64 {
                if self.inline {
                    self.inline_buffer.truncate(size as usize);
                } else {
                    if self.dirty {
                        self.sync(bd, config, root, lookahead, name_max, gstate, gdisk, gdelta)?;
                    }
                    self.seek(0, crate::info::SeekWhence::Set)?;
                    let mut buf = vec![0u8; size as usize];
                    let mut pos = 0u64;
                    while pos < size {
                        let n = self.read(bd, config, &mut buf[pos as usize..])?;
                        if n == 0 {
                            break;
                        }
                        pos += n as u64;
                    }
                    self.inline = true;
                    self.ctz_head = BLOCK_INLINE;
                    self.ctz_size = size;
                    self.inline_buffer = buf;
                    self.block = BLOCK_INLINE;
                    self.off = 0;
                }
            } else {
                self.sync(bd, config, root, lookahead, name_max, gstate, gdisk, gdelta)?;
                let (block, _) = ctz::ctz_find(bd, config, self.ctz_head, self.ctz_size, size - 1)?;
                self.ctz_head = block;
                self.ctz_size = size;
                self.pos = size;
                self.block = BLOCK_NULL;
                self.dirty = true;
                self.writing = false;
            }
        } else if size > old_size {
            self.seek(0, crate::info::SeekWhence::End)?;
            let to_fill = (size - old_size) as usize;
            let file_max = 2_147_483_647u32;
            for _ in 0..to_fill {
                self.write(bd, config, *root, lookahead, inline_max, file_max, &[0])?;
            }
        }

        self.seek(old_pos.min(size) as i64, crate::info::SeekWhence::Set)?;
        Ok(())
    }

    /// Current read position.
    pub fn tell(&self) -> i64 {
        self.pos as i64
    }

    /// File size in bytes.
    pub fn size(&self) -> i64 {
        if self.inline && self.dirty {
            self.inline_buffer.len() as i64
        } else {
            self.ctz_size.max(self.pos) as i64
        }
    }

    /// Seek to position. Clamps to [0, size].
    pub fn seek(&mut self, off: i64, whence: crate::info::SeekWhence) -> Result<i64, Error> {
        let size = self.size_for_read();
        let pos = match whence {
            crate::info::SeekWhence::Set => off,
            crate::info::SeekWhence::Cur => self.pos as i64 + off,
            crate::info::SeekWhence::End => size as i64 + off,
        };

        let pos = pos.max(0).min(size as i64) as u64;
        self.pos = pos;

        if !self.inline {
            self.block = BLOCK_NULL;
        }

        Ok(self.pos as i64)
    }

    /// Seek to start of file.
    pub fn rewind(&mut self) -> Result<(), Error> {
        self.seek(0, crate::info::SeekWhence::Set)?;
        Ok(())
    }

    /// Close file. Syncs if dirty.
    pub fn close<B: BlockDevice>(
        mut self,
        bd: &B,
        config: &Config,
        root: &mut [u32; 2],
        lookahead: &mut alloc::Lookahead,
        name_max: u32,
        gstate: &GState,
        gdisk: &mut GState,
        gdelta: &mut GState,
    ) -> Result<(), Error> {
        if self.dirty {
            self.sync(bd, config, root, lookahead, name_max, gstate, gdisk, gdelta)?;
        }
        Ok(())
    }
}
