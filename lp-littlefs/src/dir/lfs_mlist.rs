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

/// Per lfs.c lfs_mlist_isopen (lines 508-518)
///
/// C:
/// ```c
/// static bool lfs_mlist_isopen(struct lfs_mlist *head,
///         struct lfs_mlist *node) {
///     for (struct lfs_mlist **p = &head; *p; p = &(*p)->next) {
///         if (*p == (struct lfs_mlist*)node) {
///             return true;
///         }
///     }
///
///     return false;
/// }
/// ```
pub fn lfs_mlist_isopen(_head: *mut LfsMlist, _node: *const LfsMlist) -> bool {
    todo!("lfs_mlist_isopen")
}

/// Per lfs.c lfs_mlist_remove (lines 520-527)
///
/// C:
/// ```c
/// static void lfs_mlist_remove(lfs_t *lfs, struct lfs_mlist *mlist) {
///     for (struct lfs_mlist **p = &lfs->mlist; *p; p = &(*p)->next) {
///         if (*p == mlist) {
///             *p = (*p)->next;
///             break;
///         }
///     }
/// }
/// ```
pub fn lfs_mlist_remove(lfs: *mut crate::fs::Lfs, mlist: *mut LfsMlist) {
    if lfs.is_null() || mlist.is_null() {
        return;
    }
    unsafe {
        let mut p = &mut (*lfs).mlist;
        #[cfg(feature = "loop_limits")]
        const MAX_MLIST_REMOVE_ITER: u32 = 256;
        #[cfg(feature = "loop_limits")]
        let mut iter: u32 = 0;
        while !(*p).is_null() {
            #[cfg(feature = "loop_limits")]
            {
                if iter >= MAX_MLIST_REMOVE_ITER {
                    panic!("loop_limits: MAX_MLIST_REMOVE_ITER ({}) exceeded", MAX_MLIST_REMOVE_ITER);
                }
                iter += 1;
            }
            if core::ptr::eq(*p, mlist) {
                *p = (*mlist).next;
                break;
            }
            p = &mut (*(*p)).next;
        }
    }
}

/// Per lfs.c lfs_mlist_append (lines 529-533)
///
/// C:
/// ```c
/// static void lfs_mlist_append(lfs_t *lfs, struct lfs_mlist *mlist) {
///     mlist->next = lfs->mlist;
///     lfs->mlist = mlist;
/// }
/// ```
pub fn lfs_mlist_append(lfs: *mut crate::fs::Lfs, mlist: *mut LfsMlist) {
    if lfs.is_null() || mlist.is_null() {
        return;
    }
    unsafe {
        let head = (*lfs).mlist;
        (*mlist).next = head;
        (*lfs).mlist = mlist;
    }
}
