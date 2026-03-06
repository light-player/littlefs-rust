//! File handle. Per lfs.h lfs_file_t.

use crate::bd::LfsCache;
use crate::dir::LfsMdir;
use crate::lfs_info::LfsFileConfig;
use crate::types::{lfs_block_t, lfs_off_t};

use super::lfs_ctz::LfsCtz;

/// Per lfs.h typedef struct lfs_file
#[repr(C)]
pub struct LfsFile {
    pub next: *mut LfsFile,
    pub id: u16,
    pub type_: u8,
    pub m: LfsMdir,
    pub ctz: LfsCtz,
    pub flags: u32,
    pub pos: lfs_off_t,
    pub block: lfs_block_t,
    pub off: lfs_off_t,
    pub cache: LfsCache,
    pub cfg: *const LfsFileConfig,
}
