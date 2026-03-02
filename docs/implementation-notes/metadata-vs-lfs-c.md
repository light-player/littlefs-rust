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

| C (719–783) | Rust (225–250) |
|-------------|----------------|
| Backward iteration from `dir->off`, `ntag = dir->etag` | Same |
| XOR to recover tag: `ntag = (stored ^ tag) & 0x7fffffff` | Same |
| Match `(gmask & tag) == (gmask & gtag)` | Same |
| `lfs_tag_isdelete(tag)` → return NOENT | Same |

Rust omits synthetic move / splice gdiff (Phase 01 simplification).

## Tests added

- `fetch_after_rename_commit`: Rename (CREATE 2, NAME 2, DIRSTRUCT 2, DELETE 1) → `count >= 3`, `get_entry_info(2)` = "x0", `get_entry_info(1)` = Noent.
