# Phase 2: Implement dir_commit_crc helper

## Scope of phase

Implement the full `lfs_dir_commitcrc` loop as a shared helper in commit.rs. Per reference/lfs.c:1669–1813. Does not yet integrate into dir_commit_append or dir_compact.

## Code organization reminders

- Place helper utility functions at the bottom of files
- Reference C implementation line-by-line for correctness

## Implementation details

### 1. Add dir_commit_crc function

Signature:
```rust
fn dir_commit_crc<B: BlockDevice>(
    ctx: &BdContext<'_, B>,
    block_idx: u32,
    off: &mut usize,
    ptag: &mut u32,
    crc: &mut u32,
    begin: usize,
    block_size: usize,
    prog_size: usize,
    disk_version: u32,
) -> Result<(), Error>
```

### 2. Logic (per C lfs_dir_commitcrc)

1. **end** = `align_up(min(*off + 20, block_size), prog_size)`
2. **off1, crc1** = track first (non-padding) CCRC for verification
3. **Loop** while `*off < end`:
   - `noff` = `min(end - (off+4), 0x3fe) + (off+4)`; if `noff < end` then `noff = min(noff, end - 20)`
   - **eperturb** = read 1 byte at `noff` via `ctx.read(block_idx, noff, &mut [u8; 1])`
   - **FCRC** (optional): if `noff >= end && noff <= block_size - prog_size` and `disk_version > 0x0002_0000`:
     - `fcrc_crc` = `ctx.crc(block_idx, noff, prog_size, 0xffff_ffff)`
     - Write FCRC tag (TYPE_FCRC, 0x3ff, 8) + data (prog_size LE, fcrc_crc LE) via prog, update crc and ptag
   - **CCRC**: `ntag` = `TYPE_CCRC + ((!eperturb >> 7) as u32)`, size = `noff - (off+4)`
   - Prog CCRC (tag BE xor ptag, crc LE), update crc
   - If off1==0: off1=off+4, crc1=*crc
   - *off = noff, *ptag = ntag ^ ((0x80 & !eperturb) << 24), *crc = 0xffff_ffff
   - **Sync** when `noff >= end` or `noff >= pcache.off + cache_size` (may need to add a way to check cache bounds — or sync when noff >= end, and at end of loop)
4. **Post-commit verify**:
   - Re-read CRC from begin to off1+4 via `ctx.crc(block_idx, begin, off1 - begin + 4, 0xffff_ffff)` — must equal crc1
   - Read stored CRC at off1 (4 bytes) — must be non-zero

### 3. Helpers

- `align_up(a, align)` if not already in bdcache (it has alignup for u32)
- Tag encoding: `mktag(type_, 0x3ff, size)` — reuse existing

### 4. Cache sync

C calls `lfs_bd_sync` when `noff >= end` or `noff >= lfs->pcache.off + lfs->cfg->cache_size`. BdContext does not expose pcache.off. Options:
- Call `ctx.sync()` after every iteration when `noff >= end` (we're done with CRC phase)
- Call `ctx.sync()` at end of loop
- Add `bd_sync` call after writes that might leave padding — C syncs to "manually flush since we don't prog the padding". Simplest: sync at end of loop, and when `noff >= end` before next iteration we break anyway. Sync once when we break out of loop (noff >= end). For cache-full case: our bd_prog already flushes when cache fills; the C sync is to drop rcache and flush. Sync at end of loop is sufficient for correctness.

### 5. Error handling

Return `Error::Corrupt` when verification fails (crc != crc1 or stored CRC is zero).

## Validate

```bash
cd lp-littlefs && cargo build
cargo fmt
```

(Phase 2 does not yet wire dir_commit_crc into callers; build will succeed but the fn is dead code until phase 3.)
