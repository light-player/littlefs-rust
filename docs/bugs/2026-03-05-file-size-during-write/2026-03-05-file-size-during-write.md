# Bug: lfs_file_size returns 0 during active writes

## Summary

`lfs_file_size` returns `file->ctz.size` unconditionally, but `ctz.size` is
only updated on flush/sync/close. During active writes (before any flush),
`ctz.size` remains at its initial value (0 for new files, or the on-disk size
for existing files). The C reference checks `LFS_F_WRITING` and returns
`lfs_max(file->pos, file->ctz.size)` when the file has unflushed writes.

## Severity

High. Affects all callers of `lfs_file_size` and `lfs_file_seek` with
`LFS_SEEK_END` while a file has unflushed writes.

## Root cause

The Rust `lfs_file_size_` is a direct translation of only the fallback path:

```rust
// lp-littlefs/src/file/ops.rs:1684-1689
pub fn lfs_file_size_(
    _lfs: *const core::ffi::c_void,
    file: *const LfsFile,
) -> lfs_soff_t {
    unsafe { (*file).ctz.size as lfs_soff_t }
}
```

The C reference (lfs.c:3849-3858) has:

```c
static lfs_soff_t lfs_file_size_(lfs_t *lfs, lfs_file_t *file) {
    (void)lfs;
#ifndef LFS_READONLY
    if (file->flags & LFS_F_WRITING) {
        return lfs_max(file->pos, file->ctz.size);
    }
#endif
    return file->ctz.size;
}
```

The `LFS_F_WRITING` branch was not translated.

## Minimal reproducer

```
format + mount
open "f" with O_WRONLY | O_CREAT
write 4 bytes of "hair"
lfs_file_size => expected 4, actual 0    ← BUG
```

Smallest failing test: `test_truncate_simple::case_01` (medium=31, large=32).
After writing 32 bytes, `lfs_file_size` returns 0 instead of 32.

## Impact

### Direct callers

- **`lfs_file_size` (public API)**: Returns wrong size for any file with
  unflushed writes. Previously masked by extra `lfs_file_sync` calls inserted
  in tests.

- **`lfs_file_seek_` with `LFS_SEEK_END`**: Uses `lfs_file_size_` to compute
  the end position. Seeking to `SEEK_END + offset` during active writes would
  compute from the stale `ctz.size` instead of the actual written extent.

### Workaround already in place

`lfs_file_truncate_` (ops.rs:1566) already works around this:
```rust
let oldsize = crate::util::lfs_max(file_ref.pos, file_ref.ctz.size);
```
This local fix should be removed once the root cause is fixed, replaced with a
call to `lfs_file_size_`.

## Fix

Add the `LFS_F_WRITING` check to `lfs_file_size_`:

```rust
pub fn lfs_file_size_(
    _lfs: *const core::ffi::c_void,
    file: *const LfsFile,
) -> lfs_soff_t {
    unsafe {
        if ((*file).flags as i32 & LFS_F_WRITING) != 0 {
            return crate::util::lfs_max((*file).pos, (*file).ctz.size) as lfs_soff_t;
        }
        (*file).ctz.size as lfs_soff_t
    }
}
```

And update `lfs_file_truncate_` to use `lfs_file_size_()` instead of its
local `lfs_max` workaround.

## Tests affected

After removing the masking `lfs_file_sync` calls in the previous commit
(88fa0d1), these tests now fail and expose this bug:

- `test_truncate_simple`: 8 of 10 cases
- `test_truncate_read`: 3 of 4 cases
- `test_truncate_write`: all 3 cases
- `test_truncate_write_read`: 1 case
- `test_truncate_nop`: all 8 cases
