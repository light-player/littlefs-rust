//! File operations: open, read, seek, tell, size, close.
//!
//! Per lfs.c lfs_file_opencfg_, lfs_file_flushedread, lfs_file_seek, etc.

use crate::block::BlockDevice;
use crate::config::Config;
use crate::error::Error;
use crate::info::OpenFlags;

use super::ctz;
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
}

impl File {
    /// Open a file for reading.
    ///
    /// Only RDONLY is supported in this phase.
    pub fn open<B: BlockDevice>(
        bd: &B,
        config: &Config,
        root: [u32; 2],
        path: &str,
        name_max: u32,
        flags: OpenFlags,
    ) -> Result<Self, Error> {
        if !flags.contains(OpenFlags::RDONLY) {
            return Err(Error::Inval);
        }

        let (dir, id) = path::dir_find(bd, config, root, path, name_max)?;
        if id == 0x3ff {
            return Err(Error::IsDir);
        }

        let info = metadata::get_entry_info(&dir, id, name_max)?;
        if info.typ != crate::info::FileType::Reg {
            return Err(Error::IsDir);
        }

        let (inline_, head, size) = metadata::get_file_struct(&dir, id)?;

        let (block, off) = if inline_ {
            (BLOCK_INLINE, 0u32)
        } else {
            (BLOCK_NULL, 0)
        };

        Ok(Self {
            id,
            mdir: dir,
            ctz_head: head,
            ctz_size: size,
            inline: inline_,
            pos: 0,
            block,
            off,
            block_size: config.block_size,
        })
    }

    /// Read bytes from file. Returns 0 at EOF.
    pub fn read<B: BlockDevice>(
        &mut self,
        bd: &B,
        config: &Config,
        buf: &mut [u8],
    ) -> Result<usize, Error> {
        if self.pos >= self.ctz_size {
            return Ok(0);
        }

        let to_read = (buf.len() as u64).min(self.ctz_size - self.pos) as usize;
        if to_read == 0 {
            return Ok(0);
        }

        if self.inline {
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

    /// Current read position.
    pub fn tell(&self) -> i64 {
        self.pos as i64
    }

    /// File size in bytes.
    pub fn size(&self) -> i64 {
        self.ctz_size as i64
    }

    /// Seek to position. Clamps to [0, size].
    pub fn seek(&mut self, off: i64, whence: crate::info::SeekWhence) -> Result<i64, Error> {
        let pos = match whence {
            crate::info::SeekWhence::Set => off,
            crate::info::SeekWhence::Cur => self.pos as i64 + off,
            crate::info::SeekWhence::End => self.ctz_size as i64 + off,
        };

        let pos = pos.max(0).min(self.ctz_size as i64) as u64;
        self.pos = pos;

        if !self.inline {
            self.block = BLOCK_NULL;
        }

        Ok(self.pos as i64)
    }

    /// Close file. No-op for read-only.
    pub fn close(self) -> Result<(), Error> {
        Ok(())
    }
}
