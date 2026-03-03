//! Open list node. Per lfs.h struct lfs_mlist.

use super::lfs_mdir::LfsMdir;

/// Per lfs.h struct lfs_mlist
#[repr(C)]
pub struct LfsMlist {
    pub next: *mut LfsMlist,
    pub id: u16,
    pub type_: u8,
    pub m: LfsMdir,
}
