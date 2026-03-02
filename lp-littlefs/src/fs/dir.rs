//! Directory iteration.

use crate::block::BlockDevice;
use crate::config::Config;
use crate::error::Error;
use crate::info::{FileType, Info};

use super::metadata;

/// Read next directory entry. Returns 1 on success, 0 at end of directory.
pub fn dir_read<B: BlockDevice>(
    bd: &B,
    config: &Config,
    dir: &mut super::Dir,
    info: &mut Info,
    name_max: u32,
) -> Result<u32, Error> {
    if dir.pos == 0 {
        *info = Info::new(FileType::Dir, 0);
        info.set_name(b".");
        dir.pos = 1;
        return Ok(1);
    }
    if dir.pos == 1 {
        *info = Info::new(FileType::Dir, 0);
        info.set_name(b"..");
        dir.pos = 2;
        return Ok(1);
    }

    loop {
        if dir.id >= dir.mdir.count {
            if !dir.mdir.split {
                return Ok(0);
            }
            dir.mdir = metadata::fetch_metadata_pair(bd, config, dir.mdir.tail)?;
            dir.id = 0;
        }

        match metadata::get_entry_info(&dir.mdir, dir.id, name_max) {
            Ok(entry_info) => {
                *info = entry_info;
                dir.id += 1;
                dir.pos += 1;
                return Ok(1);
            }
            Err(Error::Noent) => {
                dir.id += 1;
                continue;
            }
            Err(e) => return Err(e),
        }
    }
}
