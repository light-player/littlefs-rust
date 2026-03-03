# CRC layout alignment plan — Notes

## Scope of work

Implement full `lfs_dir_commitcrc` behavior in Rust to achieve binary compatibility with upstream C:

1. **Alignment to prog_size** — Commit end must align up to prog_size (not a single 8-byte CRC at arbitrary offset).
2. **5-word CRC loop** — Multiple CCRC blocks to fill remainder; padding not CRC'd.
3. **FCRC (forward CRC)** — When space allows (noff <= block_size - prog_size) and disk_version >= 0x00020001, write FCRC tag before CCRC.
4. **Valid bit from eperturb** — Derive CCRC valid bit from leading byte of next prog unit for power-loss detection.

## Current state of the codebase

### Commit paths

| Location | Current behavior |
|----------|------------------|
| `commit.rs:dir_commit_append` (193–296) | Single CCRC (tag + 4-byte crc) at end. No alignment, no FCRC. |
| `commit.rs:dir_compact` (339–561) | Same: single CCRC at end after traverse + attrs. |
| `format.rs` | Format writes superblock commit; uses simple CRC. |

### BdContext (bdcache.rs)

- `read(block, off, buffer)` — cached read (pcache, rcache, device)
- `prog(block, off, data)` — cached prog
- `sync()` — flush pcache, drop rcache
- **No `bd_crc`** — C's `lfs_bd_crc` reads through cache and computes CRC; we need equivalent.

### Block crate (block/mod.rs)

- `block_crc(bd, block, off, size, init)` — uses raw `bd.read`, bypasses cache. Marked `#[allow(dead_code)]`.

### Metadata fetch (metadata.rs)

- Already handles multiple CCRC blocks (continues on each, updates last_off).
- Already handles FCRC (sets hasfcrc, used for erased check).
- No fetch changes needed.

### Disk version

- `DISK_VERSION = 0x00020001` (superblock.rs)
- MountState has `disk_version` from superblock
- Format always uses 0x00020001

### C reference (reference/lfs.c:1669–1813)

```c
end = alignup(min(commit->off + 20, block_size), prog_size)
while (commit->off < end) {
  noff = min(end - (off+4), 0x3fe) + (off+4)
  if (noff < end) noff = min(noff, end - 20)

  // FCRC when: noff >= end && noff <= block_size - prog_size
  // and disk_version > 0x00020000
  eperturb = read 1 byte at noff
  if (space for fcrc && disk_version >= 0x00020001) {
    fcrc = crc of prog_size bytes at noff
    commitattr(FCRC, fcrc)
  }

  ntag = LFS_MKTAG(TYPE_CCRC + ((~eperturb)>>7), 0x3ff, noff-(off+4))
  write ccrc (tag, crc), update ptag, reset crc
  commit->off = noff
  if (noff >= end or cache full) sync
}
// Verify: crc from begin to off1+4 == crc1, stored crc at off1 != 0
```

---

## Questions

### Q1: Disk version for FCRC gating

**Context:** C gates FCRC on `lfs_fs_disk_version(lfs) > 0x00020000`. We format with 0x00020001. When committing, we need to know disk_version to decide whether to write FCRC.

**Current state:** `MountState.disk_version` exists. Commit functions (`dir_commit_append`, `dir_compact`) receive `BdContext` but not disk_version.

**Suggested answer:** Pass `disk_version: u32` into commit context. For `dir_commit_append` this flows from the caller (e.g. relocatingcommit has dir from fetch, but fetch doesn't store disk_version on MdDir). We need disk_version at commit sites. Options:

- a) Add `disk_version` to `BdContext` (or a `CommitContext` that wraps BdContext + disk_version) — but BdContext is used widely, disk_version is FS-level.
- b) Add `disk_version` parameter to `dir_commit_append` and `dir_compact` — explicit, callers must provide it.
- c) Store disk_version in MdDir when fetching — fetch could accept it and pass through, but MdDir is metadata-pair state, not FS config.
- d) Add a small `CommitConfig` struct `{ disk_version }` passed to commit functions.

**Recommendation:** Add `disk_version: u32` parameter to the commit CRC helper. Callers (`dir_relocatingcommit` → `dir_commit_append`, `dir_compact`, `format`) obtain it from `MountState` or format context. For format, disk_version is the one we're writing (0x00020001).

---

### Q2: Format / first commit CRC layout

**Context:** `format.rs` writes the initial superblock. Does it use the commit machinery or hand-roll the block? If it hand-rolls, it must also use the aligned CRC layout for binary compatibility.

**Current state:** Format builds the block and commits; need to verify it goes through `dir_commit_append` or equivalent. If format uses a different path, it must be updated to use the shared CRC logic.

**Suggested answer:** Format should use the same CRC logic as regular commits. Extract `dir_commit_crc` as a shared helper and have format call it (or go through a commit path that uses it). Format writes to a fresh pair; it has `off`, `ptag`, `crc` after writing attrs—same shape as `dir_commit_append` before CRC.

---

### Q3: bd_crc with cache

**Context:** FCRC requires computing CRC of `prog_size` bytes at `noff`. C uses `lfs_bd_crc` which reads through rcache/pcache. We have `block::block_crc` that uses raw `bd.read` (bypasses cache).

**Suggested answer:** Add `bd_crc` to bdcache that uses `bd_read` in a loop with `crc::crc32`. This ensures we read through cache (pcache may hold just-programmed data; region at noff could be 0xff from erase, or from a prior read). Implementation: loop reading chunks, accumulate CRC.

---

### Q4: C `commitattr` for FCRC vs inline CCRC prog

**Context:** C writes FCRC via `lfs_dir_commitattr` which updates commit->crc, commit->off, commit->ptag. Our Rust `dir_commit_append` writes attrs inline. The CRC loop will need to write FCRC (tag + 8-byte struct) and CCRC (tag + 4-byte crc) itself, updating ptag and crc.

**Suggested answer:** Extract the "write one tag + data" logic into a helper used by both the attr loop and the CRC loop. Or inline the FCRC/CCRC writes in the CRC loop—they're special tags, not generic attrs. Keep the loop self-contained: for each iteration, optionally write FCRC (via prog + update crc/ptag), then write CCRC.

---

## Notes

### Q1 (disk version): Either
- Add `disk_version: u32` as a separate parameter, or use a `CommitConfig { disk_version }` struct. Choose whichever fits the codebase better.

### Q2 (format): Follow C implementation
- **C format** (lfs.c:4391–4449): Uses `lfs_dir_alloc` then `lfs_dir_commit` with superblock attrs — full commit machinery including `lfs_dir_commitcrc`. Second `lfs_dir_commit` with NULL attrs to force compaction.
- **Rust format** (format.rs): Hand-rolls: builds block in buffer, pads to prog_size, single CCRC, progs both blocks 0,1. Does not use dir_alloc or commit.
- **Plan:** Refactor format to follow C: use `dir_alloc` (with lookahead for fresh device) and `dir_commit_append` + shared `dir_commit_crc`. Format will prog to device via commit path. This ensures binary compatibility with C format.

### Q3 (bd_crc): Add cached bd_crc to bdcache
- Add `bd_crc` that uses `bd_read` in a loop with `crc::crc32` — reads through pcache/rcache.

### Q4 (FCRC/CCRC writes): Inline in CRC loop
- Write FCRC and CCRC directly in the loop; prog each, update ptag and crc.

---

## C implementation alignment analysis

### What the design matches

| C element | Our design | Notes |
|-----------|------------|-------|
| `lfs_dir_commitcrc` loop | `dir_commit_crc` helper | Same end formula, noff calc, FCRC/CCRC logic |
| `lfs_bd_crc` for FCRC | `bd_crc` in bdcache | Must use cached read (not raw bd.read) |
| FCRC via commitattr | Inline prog in loop | C uses commitattr→commitprog; we prog directly. Same outcome |
| Post-commit verification | Need to add | C re-reads begin..off1+4, checks crc==crc1; reads stored CRC at off1, must be non-zero |
| Cache sync when noff≥end or cache full | Need to add | C calls `lfs_bd_sync` conditionally. Must sync before padding skip |
| disk_version for FCRC gating | Param to commit_crc | C uses `lfs_fs_disk_version(lfs)` |
| metadata_max / commit end | Already in dir_compact | C: `end = metadata_max ? metadata_max : block_size - 8` |

### C flow we must mirror

1. **relocatingcommit (inline append)**  
   `lfs_commit` { block=dir.pair[0], off=dir.off, begin=dir.off, end=metadata_max-8 } → traverse (commitattr) → movestate if needed → `lfs_dir_commitcrc` → dir.off=commit.off, dir.etag=commit.ptag.

2. **dir_compact**  
   `lfs_commit` { block=dir.pair[1], off=0, begin=0, end=metadata_max-8 } → erase → rev (commitprog) → traverse → tail → movestate → `lfs_dir_commitcrc` → swap pair, dir.off, dir.etag.

3. **format**  
   `lfs_dir_alloc` → `lfs_dir_commit` (superblock attrs) → root.erased=false → `lfs_dir_commit` (NULL attrs, empty compact).

### Gaps to address

1. **Post-commit verification** — C checks non-padding CRC after the loop (lfs.c:1778–1807). Include equivalent logic in `dir_commit_crc`.

2. **Conditional sync** — C syncs when `noff >= end` or `noff >= pcache.off + cache_size`. We need access to pcache state or an equivalent signal. Options: (a) add `should_sync(block, noff)` using pcache, or (b) sync after each iteration when noff≥end, and at end of loop.

3. **commit.end vs attr space** — C uses `end = metadata_max - 8` for the attr phase. Our `CRC_MIN = 20` is stricter. Verify we reserve enough space for the CRC loop (attrs must stop with room for align + 5 words minimum).

4. **FCRC struct layout** — C `lfs_fcrc` is `{ size, crc }` (8 bytes). Our metadata fetch expects 12 bytes (4+4+4 for tag + size + crc). Tag is 4, data is 8 (size LE + crc LE). So dsize=8, tag_dsize=12. Correct.
