# metadata.rs vs reference/lfs.c — Line-by-Line Analysis

Comparison of `fetch_metadata_pair` (Rust) with `lfs_dir_fetchmatch` (reference/lfs.c) and `get_entry_info` with `lfs_dir_getinfo`.

## fetch_metadata_pair vs lfs_dir_fetchmatch

### Block selection (revs)

| C (lfs.c:1122–1137) | Rust (metadata.rs:91–113) |
|---------------------|---------------------------|
| Read revs from both blocks, pick block with `lfs_scmp(revs[i], revs[(i+1)%2]) > 0` | Same: `(revs[0] as i32).wrapping_sub(revs[1] as i32) >= 0` → pick block 0 or 1 |
| `dir->pair[0]` = winning block | `block` = winning block, `block_idx` = pair[r] |

### Loop structure

C advances before read: `off += lfs_tag_dsize(ptag)` then read at `off`.  
Rust starts at `off = 4`, reads at `off`, then does `off += dsize`. Same semantics.

### Tag decoding

| C | Rust |
|---|------|
| `tag = lfs_frombe32(stored) ^ ptag` | `tag = (stored_tag ^ ptag) & 0x7fff_ffff` |
| `lfs_tag_type1(t) = (t & 0x70000000) >> 20` | `tag_type1(t) = (t & 0x7000_0000) >> 20` |
| `lfs_tag_type2(t) = (t & 0x78000000) >> 20` | `tag_type2(t) = (t & 0x7800_0000) >> 20` |
| `lfs_tag_id(t) = (t & 0x000ffc00) >> 10` | `tag_id(t) = ((t >> 10) & 0x3ff) as u16` |
| `lfs_tag_dsize(t)` uses `lfs_tag_size(t + lfs_tag_isdelete(t))` | `tag_dsize` handles 0x3ff (delete) as 0-byte data |

### CRC block (TYPE_CCRC)

| C (1194–1233) | Rust (144–163) |
|---------------|----------------|
| Check `crc == dcrc`, `ptag ^= (chunk & 1) << 31` | Same |
| `dir->count = tempcount` | `tempcount = max(tempcount, max_id+1)` (see below) |

### Count logic (critical difference)

**C (1248–1255):**

```c
if (lfs_tag_type1(tag) == LFS_TYPE_NAME) {
    if (lfs_tag_id(tag) >= tempcount)
        tempcount = lfs_tag_id(tag) + 1;
} else if (lfs_tag_type1(tag) == LFS_TYPE_SPLICE) {
    tempcount += lfs_tag_splice(tag);  // CREATE +1, DELETE -1
}
```

C uses only splice for the final count. For a rename commit (CREATE 2, NAME 2, DIRSTRUCT 2, DELETE 1):

- CREATE 2: tempcount += 1 → 3 (from 2)
- NAME 2: id 2 < 3, no change
- DELETE 1: tempcount += -1 → 2

So C yields `count = 2`. With `dir_read` iterating ids 0..count (1..count for root), id 2 is never checked.

**Rust iteration:** `find_name_in_dir_pair` iterates `start_id..dir.count` and `dir_read` uses `dir.id < dir.mdir.count`. Both need `count >= max_id + 1` to reach all entries.

**Fix:** Track `max_id` over NAME and SPLICE tags. At CRC: `tempcount = max(tempcount, max_id + 1)` so `count >= max_id + 1` for iteration.

**Caveat:** Only apply `max_id + 1` when we've seen at least one NAME or SPLICE tag (`seen_name_or_splice`). Empty child dirs (only SOFTTAIL, no entries) have no NAME/SPLICE; without this guard we'd force `count = 1`, causing `remove` to incorrectly return NotEmpty.

### NAME / SPLICE / TAIL handling

| C | Rust |
|---|------|
| `LFS_TYPE_NAME` = 0x000 (type1) — REG 0x001 and DIR 0x002 both match | Same: `tag_type1 == TYPE_NAME` matches REG/DIR |
| SPLICE: `tempcount += lfs_tag_splice(tag)`; chunk as int8_t | Same: `tag_splice` = `tag_chunk as i8 as i32` |
| TAIL: read 8-byte pair, `tempsplit = chunk & 1` | Same |

## get_entry_info vs lfs_dir_getinfo

| C (1413–1445) | Rust (240–338) |
|---------------|----------------|
| Root id 0x3ff → "/" | Same |
| NAME: `lfs_dir_get(..., LFS_MKTAG(0x780, 0x3ff, 0), LFS_MKTAG(LFS_TYPE_NAME, id, name_max+1), ...)` | `get_tag_backwards` with gmask `0x780ffc00`, name_gtag `(TYPE_NAME<<20)\|(id<<10)\|(name_max+1)` |
| STRUCT: `LFS_MKTAG(0x700, 0x3ff, 0)` | `0x700f_fc00` |
| Id 0: SUPERBLOCK for root | Same |

Mask 0x780 matches both REG and DIR name tags; 0x7ff would also match, 0x780 is the canonical mask from C.

## get_tag_backwards vs lfs_dir_getslice

| C (719–783) | Rust (metadata.rs) |
|-------------|-------------------|
| Backward iteration from `dir->off`, `ntag = dir->etag` | Same |
| XOR to recover tag: `ntag = (stored ^ tag) & 0x7fffffff` | Same |
| Splice gdiff (lfs.c:750-761): `tag_type1 == SPLICE`, `gdiff += splice` | Same condition; only add for pure `TYPE_SPLICE` (0x400)—CREATE/DELETE use chunk differently and would corrupt id search |
| "Found where we were created" (lfs.c:753-756) | In match block: return Noent when matched tag is CREATE |
| Match `(gmask & tag) == (gmask & gtag)` | Same |
| `lfs_tag_isdelete(tag)` → return NOENT | Same |

- **Synthetic moves** (lfs.c:726-734): Omitted; would require threading gstate/gdisk into `get_tag_backwards`.

## Tests added

- `fetch_after_rename_commit`: Rename (CREATE 2, NAME 2, DIRSTRUCT 2, DELETE 1) → `count >= 3`, `get_entry_info(2)` = "x0", `get_entry_info(1)` = Noent.
- `get_tag_backwards_struct2_after_rename`, `get_tag_backwards_delete1_after_rename`, `get_tag_backwards_create2_noent`: Direct `get_tag_backwards` tests for the rename scenario (struct/delete/create paths).

**Note:** `test_dirs_mkdir_file_open_remount` and `test_dirs_other_errors` currently fail. They require emitting SPLICE when inserting before existing entries (commit path), which is separate from the `get_tag_backwards` read path.

---

# commit.rs vs reference/lfs.c — lfs_dir_commit

Comparison of `commit.rs` with `lfs_dir_commit`, `lfs_dir_relocatingcommit`, `lfs_dir_orphaningcommit`, and related C commit machinery.

## Function Mapping

| C function | Rust counterpart | Notes |
|------------|-------------------|-------|
| `lfs_dir_alloc` (1815) | `dir_alloc` | Same: allocate pair, erase, init empty MdDir |
| `lfs_dir_commitattr` (1621) | `dir_commit_append` loop | Same: write tag XOR ptag, data, update ptag |
| `lfs_dir_commitcrc` (1669) | CRC block in `dir_commit_append` | **Divergence:** C uses FCRC, align, padding; Rust uses single 8-byte CRC |
| `lfs_dir_commit` (2601) | `dir_orphaningcommit` + deorphan | C calls orphaningcommit then `lfs_fs_deorphan`; Rust does not call deorphan |
| `lfs_dir_relocatingcommit` (2234) | `dir_relocatingcommit` | Same flow: hasdelete+drop, inline append vs compact |
| `lfs_dir_orphaningcommit` (2408) | `dir_orphaningcommit` | Same: relocatingcommit, drop handling, relocation chain fix |
| `lfs_dir_compact` (1952) | `dir_compact` | Same: write to pair[1], swap, relocate on Nospc/Corrupt |
| `lfs_dir_split` (1880) | `dir_split` | Same: alloc tail, compact [split,end], set dir.tail/split |
| `lfs_dir_splittingcompact` (2125) | `dir_splittingcompact` | Same: binary-search split, compact |
| `lfs_dir_drop` (1859) | `dir_drop` | Same: steal gstate and tail from orphan into pred |
| `lfs_dir_needsrelocation` (1936) | `dir_needsrelocation` | Same: `(rev+1) % ((block_cycles+1)\|1) == 0` |
| `lfs_dir_commitprog` | inline in `dir_commit_append` | Same: prog + crc update |
| `lfs_dir_commit_commit` (1932) | `dir_traverse_tags` callback | Same: traverse and write tags |
| `lfs_dir_traverse` | `dir_traverse_tags` (metadata) | Same: iterate tags, call callback |

## Known Divergences

### CRC / FCRC layout

**C (lfs_dir_commitcrc, 1669–1813):**

- Aligns to prog_size; uses 5-word CRC blocks with optional FCRC (forward CRC) before padding.
- `LFS_TYPE_FCRC` (0x3ff) written when space allows.
- Multiple CRC blocks to fill remainder of block; padding not CRC'd.
- Post-commit verifies non-padding CRC.

**Rust (dir_commit_append, dir_compact):**

- Single 8-byte CRC (TYPE_CCRC + 4-byte crc) at end.
- No FCRC.
- No alignment/padding loop.
- Simpler layout; may not match C on-disk for blocks with padding.

### C `dir->erased` path

C `lfs_dir_relocatingcommit` (2270) checks `dir->erased && dir->count < 0xff` to attempt inline append without compact. Rust does not use an `erased` flag; it always tries `dir_commit_append` first when not forcing relocation.

### Deorphan after orphaningcommit

C `lfs_dir_commit` (2601) calls `lfs_fs_deorphan` when `orphans` is true. Rust `dir_orphaningcommit` returns `orphans` but the caller does not invoke a deorphan step. Orphan chain fixup is done inside `dir_orphaningcommit` (parent updates, pred soft/hard tail); C additionally runs `lfs_fs_deorphan` to clean remaining orphans.

### Mlist / open-handle fixup

C `lfs_dir_relocatingcommit` (2354–2404) fixes `lfs->mlist` (open dirs/files) when the committed pair is relocated: updates pair, adjusts ids for CREATE/DELETE, refetches tail. Rust has no mlist; open handles are not automatically updated when a dir is relocated.

### SPLICE on insert-before

C uses CREATE (which has type1=SPLICE and splice +1) when inserting; no separate SPLICE tag is emitted in the mkdir/file create path. Rust emits CREATE at the insertion id; with the gdiff fix in `get_tag_backwards` (add gdiff for positive splice only), both mkdir+file and insert-before scenarios read correctly. `test_dirs_mkdir_file_open_remount` and `test_dirs_other_errors` pass.

### Root update in dir_split

C `lfs_dir_split` (1905–1909) updates `lfs->root` when splitting root and `split == 0`. Rust `dir_split` does not; root update is handled elsewhere (e.g. orphaningcommit relocation chain).

### Inline file eviction before orphaningcommit

C `lfs_dir_orphaningcommit` (2413–2426) evicts inline files that share the dir and exceed cache size before committing. Rust does not perform this eviction.

---

# path.rs vs reference/lfs.c — lfs_dir_find / dir_find_for_create

Comparison of `path.rs` with `lfs_dir_find` and the create-path logic (C has no separate `lfs_dir_find_create`).

## Function Mapping

| C | Rust | Notes |
|---|------|-------|
| `lfs_dir_find` (1483) | `dir_find` | Path resolution, returns (dir, id); id=0x3ff for root |
| (create path: `lfs_dir_find` + `lfs_path_islast`) | `dir_find_for_create` | Separate fn in Rust; C uses same lfs_dir_find, caller checks NOENT + islast |
| `lfs_dir_find_match` (1453) | `find_name_in_dir` / `find_name_in_dir_pair` | Name comparison in directory |
| (id from fetchmatch) | `find_insertion_id` | C: `*id = lfs_min(lfs_tag_id(besttag), dir->count)`; Rust: explicit alphabetical scan |
| `lfs_path_namelen` (288) | `segments` + `seg` | Rust splits path and uses segment length |
| `lfs_path_islast` (292) | N/A (separate dir_find_for_create) | C: path has no more components |
| `lfs_path_isdir` (297) | N/A | C: when creating file, reject path with trailing slash or subpath |
| — | `path_is_descendant` | Rust-only: prevents rename-dir-into-itself (littlefs#1162) |
| — | `path_last_component` | Rust-only helper |
| — | `find_dotdot_cancel_count` | Rust: ".." cancellation; C inlines similar logic in lfs_dir_find |

## Flow Comparison

### lfs_dir_find (C) vs dir_find (Rust)

**C (1483–1590):**

- Mutates `*path` to point at current component.
- Uses `lfs_dir_fetchmatch` with `lfs_dir_find_match` callback for name match.
- Handles `.` (skip), `..` (error at root; else follow parent via ".." cancellation in suffix).
- Iterates tail on `dir->split` when no match in current block.
- Returns tag (or LFS_ERR_NOENT, LFS_ERR_NOTDIR).
- When `id` is non-NULL, fetchmatch sets `*id` even on NOENT: `*id = lfs_min(lfs_tag_id(besttag), dir->count)`.

**Rust dir_find:**

- Normalizes path (trim slashes, split), does not mutate path.
- Walks segments with stack for `..`.
- Uses `find_name_in_dir` → `find_name_in_dir_pair` or tail recursion on split.
- Returns `(MdDir, u16)` where u16=0x3ff for root.
- Same semantics: find entry, follow dirs, handle `.` and `..`.

### Create path: C vs dir_find_for_create

**C:**

- Call `lfs_dir_find(lfs, &cwd.m, &path, &id)`.
- If `err == LFS_ERR_NOENT && lfs_path_islast(path)` → can create.
- `id` is set by fetchmatch (insertion slot).
- `lfs_path_namelen(path)` gives name length.

**Rust:**

- Call `dir_find_for_create` which returns `(parent_dir, id, name)` or `Err(Noent)` / `Err(Exist)`.
- `find_insertion_id`: scan `start_id..dir.count`, compare names, return first id where `name >= entry` or `dir.count`.
- Returns `Err(Exist)` if final component exists; `Err(Noent)` if parent missing; `Ok(...)` when creation is valid.

## Known Divergences

### Id for create

**C:** `lfs_dir_fetchmatch` sets `*id = lfs_min(lfs_tag_id(besttag), dir->count)`. On no match, `besttag` can be invalid; `lfs_tag_id(besttag)` and comparison with `dir->count` yield the insertion id. C uses tag-based best-match logic.

**Rust:** `find_insertion_id` scans entries in id order and compares names to preserve alphabetical order. Semantics may differ when entries are not strictly ordered by id.

### Path mutation

C mutates `*path` to advance; Rust works with owned/copied segments. No functional difference for resolution.

### ".." at root

Both return Inval when going above root with `..`.

### Root representation

Both use id 0x3ff for root; root pair from fetch.

### get_dir_struct / lfs_dir_get STRUCT

Rust `get_dir_struct` calls `metadata::get_struct` for TYPE_STRUCT; C uses `lfs_dir_get` with `LFS_MKTAG(LFS_TYPE_STRUCT, id, 8)`. Same semantics.

---

# file.rs vs reference/lfs.c — lfs_file_*

Comparison of `file.rs` with `lfs_file_opencfg_`, `lfs_file_read_`, `lfs_file_write_`, `lfs_file_sync_`, and related C file operations.

## Function Mapping

| C function | Rust counterpart | Notes |
|------------|-------------------|-------|
| `lfs_file_opencfg_` (3065) | `File::open` | Open by path, CREAT/EXCL/TRUNC/APPEND |
| `lfs_file_close_` (6246) | `File::close` | Sync if dirty, drop |
| `lfs_file_read_` (3552) | `File::read` | Read bytes |
| `lfs_file_flushedread` (3492) | logic in `File::read` | Read from inline or CTZ blocks |
| `lfs_file_write_` (3571) | `File::write` | Write bytes |
| `lfs_file_flushedwrite` (3571) | logic in `File::write` | Write to inline buffer or CTZ |
| `lfs_file_sync_` (3430) | `File::sync` | Flush + commit metadata |
| `lfs_file_flush` (3352) | (inline in sync) | Copy tail, flush cache, update ctz |
| `lfs_file_outline` (3336) | `File::outline` | Migrate inline → CTZ when exceeding inline_max |
| `lfs_file_relocate` (3265) | N/A | C: relocate on bad block; Rust does not relocate on prog error |
| `lfs_file_truncate_` (6264) | `File::truncate` | Truncate to size |
| `lfs_file_seek_` | `File::seek` | Seek Set/Cur/End |
| `lfs_file_tell_` | `File::tell` | Current position |
| `lfs_file_size_` | `File::size` | File size |
| `lfs_file_rewind_` (6296) | `File::rewind` | Seek to 0 |

## Flow Comparison

### Open

**C (lfs_file_opencfg_):**

- `lfs_dir_find` for path; if NOENT + `lfs_path_islast` and CREAT → allocate via `lfs_dir_commit` (CREATE, REG name, INLINESTRUCT 0).
- Adds file to mlist.
- Loads inline or CTZ from `lfs_dir_get` (STRUCT).
- Handles LFS_F_INLINE, LFS_F_DIRTY, cache.
- User attrs via `lfs_dir_get` / config.
- TRUNC → force INLINESTRUCT path.
- EXCL → LFS_ERR_EXIST if found.

**Rust File::open:**

- RDONLY: `dir_find`; else `dir_find` or `dir_find_for_create` on Noent.
- CREAT: `dir_find_for_create` then `dir_orphaningcommit` (CREATE, name_reg, inline_struct empty).
- Same checks: IsDir, Exist, Noent, Nametoolong.
- Fetches inline/CTZ via `metadata::get_file_struct`.
- APPEND → pos = size.
- TRUNC → inline, size 0.
- No user attrs; no mlist.

### Read

**C:** Uses `lfs_file_flushedread`; for inline, `lfs_dir_getread`; for CTZ, `lfs_ctz_find` + `lfs_bd_read`. Cache and block management in flushedread.

**Rust:** Inline uses `metadata::get_inline_slice` or `inline_buffer` when dirty. CTZ uses `ctz::ctz_find` + `ctx.read`. No separate cache struct; reads directly.

### Write

**C:** `lfs_file_flushedwrite`; inline overflow → `lfs_file_outline` → `lfs_file_relocate`. CTZ extend via `lfs_ctz_extend`. Buffer in cache.

**Rust:** Inline → `inline_buffer`; overflow → `outline` (alloc, erase, prog). CTZ via `write_one_ctz` → `ctz::ctz_extend`. No block-device cache in file layer.

### Sync

**C:** `lfs_file_flush` (copy tail, flush cache) then `lfs_dir_commit` (INLINESTRUCT or CTZSTRUCT) + user attrs.

**Rust:** `ctx.sync`, then `dir_orphaningcommit` with `inline_struct` or `ctz_struct`. No user attrs.

### Truncate

**C:** Similar: shrink inline or CTZ head/size; extend by writing zeros.

**Rust:** Same idea; shrink to inline when possible; extend via `write` zeros.

## Known Divergences

### User attributes

C supports `lfs_file_config` with custom attributes read/written on open and sync. Rust has no user-attr support.

### Cache

C uses `file->cache` (buffer, block, off, size) for read/write. Rust uses `inline_buffer` for inline only; CTZ writes go directly to block device (via bdcache at a lower layer).

### Bad-block relocation

C `lfs_file_relocate` and `lfs_file_outline` retry on LFS_ERR_CORRUPT. Rust does not retry on prog/erase errors; propagates error.

### Mlist / open-handle updates

C adds file to mlist; `lfs_dir_relocatingcommit` can update open handles when dir is relocated. Rust has no mlist; if a dir is relocated while a file is open, the file keeps its original `mdir.pair` until next metadata fetch (e.g. after sync).

### Inline eviction before commit

C orphaningcommit evicts large inline files from the same dir before committing. Rust does not evict.

### NOSPC → NAMETOOLONG

C (3130): `err = (err == LFS_ERR_NOSPC) ? LFS_ERR_NAMETOOLONG : err` when creating a file whose name does not fit. Rust does not map Nospc to Nametoolong in this case.
