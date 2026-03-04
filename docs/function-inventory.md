# C-to-Rust Function Inventory

Complete mapping of `reference/lfs.c` and `reference/lfs_util.c` to target .rs files. Each entry includes C line range, target file, and feature flags. Used for stub generation and implementation tracking.

## How to produce / verify

1. **List all functions**: `grep -n '^static' reference/lfs.c` for static declarations; also grep for forward-declared (`static int foo(...);`) and definitions.
2. **Feature flags**: Search for `#ifndef LFS_READONLY`, `#ifdef LFS_MULTIVERSION` etc. around each function.
3. **Module mapping**: Use Option B layout (bd, block_alloc, dir, file, fs) and place each function by responsibility (bd → bd.rs, alloc → alloc.rs, dir ops → dir/, file ops → file/, fs ops → fs/).
4. **Line numbers**: Use grep output or `rg '^static.*lfs_' reference/lfs.c`; read surrounding `#if` blocks for feature boundaries.

## Feature flags (C preprocessor → Rust cfg)

| C define | Rust feature | When to use |
|----------|--------------|-------------|
| `LFS_READONLY` | `readonly` | Omit write ops; format, mount read-only, dir read, file read only |
| `LFS_NO_MALLOC` | `no_malloc` | Use `lfs_file_opencfg` only; `lfs_file_open` returns LFS_ERR_NOMEM |
| `LFS_MULTIVERSION` | `multiversion` | `disk_version` in config, `lfs_fs_disk_version*` |
| `LFS_SHRINKNONRELOCATING` | `shrink` | `lfs_shrink_checkblock`, shrink path in `lfs_fs_grow_` |
| `LFS_MIGRATE` | (defer) | v1→v2 migration; skip for initial port |

**Initial port**: No readonly, no_malloc, multiversion, shrink. Add later if needed.

---

## lfs_util.c

| C function | Lines | Target .rs | Notes |
|------------|-------|------------|-------|
| `lfs_crc` | 18-35 | `crc.rs` | Already ported |

lfs_util.h: `lfs_min`, `lfs_max`, `lfs_align*`, `lfs_npw2`, `lfs_ctz`, `lfs_popc`, `lfs_scmp`, `lfs_fromle32`, `lfs_tole32`, `lfs_frombe32`, `lfs_tobe32` → already in `util.rs`.

---

## Inline helpers (lfs.c top)

| C function / macro | Lines | Target | Notes |
|--------------------|-------|--------|-------|
| `lfs_path_namelen`, `lfs_path_islast`, `lfs_path_isdir` | 288-300 | `util.rs` or `dir/path.rs` | path parsing |
| `lfs_pair_*`, `lfs_tag_*`, `lfs_gstate_*` | 302-455 | `util.rs`, `tag.rs`, `lfs_gstate.rs` | Already split |
| `lfs_superblock_fromle32`, `lfs_superblock_tole32` | 487-505 | `lfs_superblock.rs` | |

---

## bd/ — Block device layer

| C function | Lines | Target | Feature |
|------------|-------|--------|---------|
| `lfs_cache_drop` | 33-37 | `bd/bd.rs` | (inline) |
| `lfs_cache_zero` | 39-44 | `bd/bd.rs` | `!readonly` |
| `lfs_bd_read` | 46-126 | `bd/bd.rs` | |
| `lfs_bd_cmp` | 128-153 | `bd/bd.rs` | |
| `lfs_bd_crc` | 155-175 | `bd/bd.rs` | |
| `lfs_bd_flush` | 177-210 | `bd/bd.rs` | `!readonly` |
| `lfs_bd_sync` | 212-226 | `bd/bd.rs` | `!readonly` |
| `lfs_bd_prog` | 228-274 | `bd/bd.rs` | `!readonly` |
| `lfs_bd_erase` | 276-282 | `bd/bd.rs` | `!readonly` |

---

## block_alloc/ — Block allocation

| C function | Lines | Target | Feature |
|------------|-------|--------|---------|
| `lfs_alloc_ckpoint` | 614-618 | `block_alloc/alloc.rs` | `!readonly` |
| `lfs_alloc_drop` | 620-625 | `block_alloc/alloc.rs` | `!readonly` |
| `lfs_alloc_lookahead` | 627-639 | `block_alloc/alloc.rs` | `!readonly` |
| `lfs_alloc_scan` | 641-663 | `block_alloc/alloc.rs` | `!readonly` |
| `lfs_alloc` | 666-791 | `block_alloc/alloc.rs` | `!readonly` |

---

## dir/ — Metadata, directory, commit

### dir/fetch.rs

| C function | Lines | Notes |
|------------|-------|-------|
| `lfs_dir_fetchmatch` | 1107-1385 | core fetch; used by lfs_dir_fetch, mount, deorphan |
| `lfs_dir_fetch` | 1387-1393 | thin wrapper over lfs_dir_fetchmatch |
| `lfs_dir_getgstate` | 1395-1411 | |
| `lfs_dir_getinfo` | 1413-1451 | |

### dir/find.rs

| C function | Lines | Notes |
|------------|-------|-------|
| `lfs_dir_find_match` | 1453-1475 | callback for traverse |
| `lfs_dir_find` | 1483-1590 | path resolution |

### dir/traverse.rs (getslice, get, getread)

| C function | Lines | Notes |
|------------|-------|-------|
| `lfs_dir_getslice` | 719-784 | core metadata fetch |
| `lfs_dir_get` | 786-791 | wrapper over getslice |
| `lfs_dir_getread` | 793-850 | read from dir for inline/copy |
| `lfs_dir_traverse_filter` | 852-910 | callback |
| `lfs_dir_traverse` | 912-1385 | iterate dir tags |

### dir/commit.rs

| C function | Lines | Feature |
|------------|-------|---------|
| `lfs_dir_commitprog` | 1604-1618 | `!readonly` |
| `lfs_dir_commitattr` | 1621-1666 | `!readonly` |
| `lfs_dir_commitcrc` | 1669-1812 | `!readonly` |
| `lfs_dir_alloc` | 1815-1857 | `!readonly` |
| `lfs_dir_drop` | 1859-1878 | `!readonly` |
| `lfs_dir_split` | 1880-1913 | `!readonly` |
| `lfs_dir_commit_size` | 1915-1923 | `!readonly` callback |
| `lfs_dir_commit_commit` | 1932-1936 | `!readonly` callback |
| `lfs_dir_needsrelocation` | 1939-1949 | `!readonly` |
| `lfs_dir_compact` | 1952-2123 | `!readonly` |
| `lfs_dir_splittingcompact` | 2125-2232 | `!readonly` |
| `lfs_dir_relocatingcommit` | 2234-2406 | `!readonly` |
| `lfs_dir_orphaningcommit` | 2408-2599 | `!readonly` |
| `lfs_dir_commit` | 2601-2623 | `!readonly` |

### dir/open.rs (dir handle ops)

| C function | Lines | Feature |
|------------|-------|---------|
| `lfs_dir_open_` | 2721-2763 | |
| `lfs_dir_close_` | 2765-2770 | |
| `lfs_dir_read_` | 2772-2815 | |
| `lfs_dir_seek_` | 2817-2857 | |
| `lfs_dir_tell_` | 2854-2857 | |
| `lfs_dir_rewind_` | 2859-2861 | |

### dir/lfs_fcrc.rs, dir/lfs_mlist

| C function | Lines | Target | Feature |
|------------|-------|--------|---------|
| `lfs_fcrc_fromle32` | 462-466 | `dir/lfs_fcrc.rs` | |
| `lfs_fcrc_tole32` | 468-473 | `dir/lfs_fcrc.rs` | `!readonly` |
| `lfs_mlist_isopen` | 508-518 | `dir/` or new `mlist.rs` | debug only |
| `lfs_mlist_remove` | 520-527 | `dir/` or `fs/` | |
| `lfs_mlist_append` | 529-533 | `dir/` or `fs/` | |

---

## file/ — File operations

### file/ctz.rs (CTZ block list)

| C function | Lines | Feature |
|------------|-------|---------|
| `lfs_ctz_fromle32` | 475-479 | (in lfs_ctz.rs) |
| `lfs_ctz_tole32` | 481-486 | (in lfs_ctz.rs) `!readonly` |
| `lfs_ctz_index` | 2873-2884 | |
| `lfs_ctz_find` | 2886-2919 | |
| `lfs_ctz_extend` | 2921-3018 | `!readonly` |
| `lfs_ctz_traverse` | 3020-3063 | |

### file/ops.rs

| C function | Lines | Feature |
|------------|-------|---------|
| `lfs_file_flushedwrite` | 564-566, 3572-3654 | `!readonly` |
| `lfs_file_write_` | 566-568, 3656-3698 | `!readonly` |
| `lfs_file_flushedread` | 592-594, 3492-3551 | |
| `lfs_file_read_` | 594-596, 3553-3570 | |
| `lfs_file_opencfg_` | 3065-3236 | |
| `lfs_file_open_` | 3238-3244 | `!no_malloc` |
| `lfs_file_close_` | 3246-3264 | `!readonly` branch |
| `lfs_file_relocate` | 3266-3335 | `!readonly` |
| `lfs_file_outline` | 3337-3348 | `!readonly` |
| `lfs_file_flush` | 3350-3429 | `!readonly` |
| `lfs_file_sync_` | 3431-3490 | `!readonly` |
| `lfs_file_truncate_` | 3753-3838 | `!readonly` |
| `lfs_file_tell_` | 3835-3838 | |
| `lfs_file_rewind_` | 3840-3850 | |
| `lfs_file_size_` | 3849-3851 | |
| `lfs_file_seek_` | 3700-3751 | |

---

## fs/ — High-level filesystem

### fs/init.rs

| C function | Lines | Feature |
|------------|-------|---------|
| `lfs_init` | 4198-4369 | |
| `lfs_deinit` | 4371-4389 | |

### fs/format.rs

| C function | Lines | Feature |
|------------|-------|---------|
| `lfs_format_` | 4391-4462 | `!readonly` |

### fs/mount.rs

| C function | Lines | Feature |
|------------|-------|---------|
| `lfs_tortoise_detectcycles` | 4464-4480 | |
| `lfs_mount_` | 4482-4645 | |
| `lfs_unmount_` | 4647-4651 | |

### fs/stat.rs (new)

| C function | Lines | Notes |
|------------|-------|-------|
| `lfs_fs_stat_` | 4653-4691 | |
| `lfs_fs_size_` | 5179-5188 | |
| `lfs_fs_size_count` | 5172-5177 | callback |

### fs/traverse.rs (new)

| C function | Lines | Notes |
|------------|-------|-------|
| `lfs_fs_traverse_` | 4693-4794 | public API; no static |

### fs/parent.rs (new)

| C function | Lines | Feature |
|------------|-------|---------|
| `lfs_fs_pred` | 4796-4833 | |
| `lfs_fs_parent_match` | 4835-4853 | callback |
| `lfs_fs_parent` | 4856-4892 | uses lfs_dir_fetchmatch + lfs_fs_parent_match |

### fs/superblock.rs (new — lfs_fs_prepsuperblock)

| C function | Lines | Feature |
|------------|-------|---------|
| `lfs_fs_prepsuperblock` | 4888-4892 | `!readonly` |
| `lfs_fs_preporphans` | 4894-4904 | `!readonly` |
| `lfs_fs_prepmove` | 4906-4914 | `!readonly` |
| `lfs_fs_desuperblock` | 4916-4953 | `!readonly` |
| `lfs_fs_demove` | 4955-4989 | `!readonly` |
| `lfs_fs_deorphan` | 4991-5120 | `!readonly` |
| `lfs_fs_forceconsistency` | 5122-5140 | `!readonly` |
| `lfs_fs_mkconsistent_` | 5143-5170 | `!readonly` |
| `lfs_fs_gc_` | 5191-5240 | `!readonly` |
| `lfs_fs_grow_` | 5253-5303 | `!readonly` |
| `lfs_shrink_checkblock` | 5244-5251 | `shrink` |
| `lfs_fs_disk_version` | 535-546 | `multiversion` |
| `lfs_fs_disk_version_major` | 547-550 | `multiversion` |
| `lfs_fs_disk_version_minor` | 552-555 | `multiversion` |

### fs/mkdir.rs, remove.rs, rename.rs, stat.rs, attr.rs

| C function | Lines | Target | Feature |
|------------|-------|--------|---------|
| `lfs_mkdir_` | 2625-2719 | `fs/mkdir.rs` | `!readonly` |
| `lfs_remove_` | 3880-3960 | `fs/remove.rs` | `!readonly` |
| `lfs_rename_` | 3961-4138 | `fs/rename.rs` | `!readonly` |
| `lfs_stat_` | 3863-3878 | `fs/stat.rs` | |
| `lfs_getattr_` | 4107-4135 | `fs/attr.rs` | |
| `lfs_commitattr` | 4141-4163 | `fs/attr.rs` | `!readonly` |
| `lfs_setattr_` | 4165-4174 | `fs/attr.rs` | `!readonly` |
| `lfs_removeattr_` | 4176-4196 | `fs/attr.rs` | `!readonly` |

---

## Public API wrappers (lib.rs or fs/)

lfs.h exposes these; they call the `_` internals:

| Public API | Calls | lfs.c lines |
|-------------|-------|-------------|
| `lfs_format` | `lfs_format_` | (wrapper) |
| `lfs_mount` | `lfs_mount_` | (wrapper) |
| `lfs_unmount` | `lfs_unmount_` | (wrapper) |
| `lfs_remove` | `lfs_remove_` | 6193-6195 |
| `lfs_rename` | `lfs_rename_` | 6227-6231 |
| `lfs_stat` | `lfs_stat_` | 6263-6267 |
| `lfs_getattr` | `lfs_getattr_` | 6090-6105 |
| `lfs_setattr` | `lfs_setattr_` | 6471-6475 |
| `lfs_removeattr` | `lfs_removeattr_` | 6487-6491 |
| `lfs_file_open` | `lfs_file_opencfg` | 6140-6146 (`!no_malloc`) |
| `lfs_file_opencfg` | `lfs_file_opencfg_` | 6193-6197 |
| `lfs_file_close` | `lfs_file_close_` | 6227-6231 |
| `lfs_file_sync` | `lfs_file_sync_` | 6263-6267 |
| `lfs_file_read` | `lfs_file_read_` | 6210-6224 |
| `lfs_file_write` | `lfs_file_write_` | 6228-6242 |
| `lfs_file_seek` | `lfs_file_seek_` | 6246-6260 |
| `lfs_file_tell` | `lfs_file_seek_(..., 0, SEEK_CUR)` | |
| `lfs_file_truncate` | `lfs_file_truncate_` | 6471-6475 |
| `lfs_file_rewind` | `lfs_file_rewind_` | 6487-6491 |
| `lfs_file_size` | `lfs_file_size_` | 6495-6499 |
| `lfs_mkdir` | `lfs_mkdir_` | 6503-6507 |
| `lfs_dir_open` | `lfs_dir_open_` | 6511-6515 |
| `lfs_dir_close` | `lfs_dir_close_` | |
| `lfs_dir_read` | `lfs_dir_read_` | |
| `lfs_dir_seek` | `lfs_dir_seek_` | |
| `lfs_dir_tell` | `lfs_dir_tell_` | 6400-6412 |
| `lfs_dir_rewind` | `lfs_dir_rewind_` | |
| `lfs_fs_stat` | `lfs_fs_stat_` | 6449-6453 |
| `lfs_fs_size` | `lfs_fs_size_` | 6449-6453 |
| `lfs_fs_traverse` | `lfs_fs_traverse_` | (direct) |
| `lfs_fs_mkconsistent` | `lfs_fs_mkconsistent_` | 6479-6483 |
| `lfs_fs_gc` | `lfs_fs_gc_` | 6495-6499 |
| `lfs_fs_grow` | `lfs_fs_grow_` | 6511-6515 |

---

## Implementation order (vertical slices)

1. **Format + mount** (minimal): bd (read, prog, erase, sync, flush, cache), block_alloc, dir (fetch, get, getgstate, alloc, commit*), fs (init, format_, mount_, unmount_, prepsuperblock)
2. **Dir read**: dir traverse, getslice, getread, getinfo, find, open, read, seek, rewind
3. **File read**: file open, read, seek, tell, size, close
4. **File write**: file write, flush, sync, truncate, ctz extend
5. **mkdir, remove, rename**: dir_find, mkdir_, remove_, rename_
6. **Attr**: commitattr, setattr, removeattr
7. **Consistency**: deorphan, forceconsistency, mkconsistent, gc, grow
8. **Optional**: multiversion, shrink

---

## New files to add (from this inventory)

- `fs/stat.rs` — lfs_stat_, lfs_fs_stat_
- `fs/traverse.rs` — lfs_fs_traverse_, lfs_fs_size_, lfs_fs_size_count
- `fs/parent.rs` — lfs_fs_pred, lfs_fs_parent_match
- `fs/superblock.rs` — lfs_fs_prepsuperblock, preporphans, prepmove, desuperblock, demove
- `fs/consistent.rs` — lfs_fs_deorphan, forceconsistency, mkconsistent_, gc_
- `fs/grow.rs` — lfs_fs_grow_
- `fs/mkdir.rs`, `fs/remove.rs`, `fs/rename.rs`, `fs/attr.rs` (or merge into fs/ops.rs)
- `dir/getslice.rs` or extend `dir/traverse.rs` with getslice, get, getread
