# Phase 3: Integrate dir_commit_crc into dir_commit_append

## Scope of phase

Replace the single 8-byte CCRC at the end of dir_commit_append with a call to dir_commit_crc. Add disk_version parameter to dir_commit_append.

## Code organization reminders

- Keep dir_commit_append logic clear: attrs loop, movestate, then CRC phase
- Update Nospc check: attrs must leave room for CRC phase (alignment + minimum)

## Implementation details

### 1. Add disk_version parameter

```rust
pub fn dir_commit_append<B: BlockDevice>(
    ctx: &BdContext<'_, B>,
    dir: &mut MdDir,
    attrs: &[CommitAttr<'_>],
    gstate_ctx: &mut Option<&mut GStateCtx<'_>>,
    disk_version: u32,  // NEW
) -> Result<(), Error>
```

### 2. Replace single CCRC block

Remove:
```rust
let crc_tag = mktag(tag::TYPE_CCRC, 0x3ff, 4);
let stored_crc_tag = ...
ctx.prog(...);
off += 4;
ctx.prog(...);
off += 4;
ptag = ...
```

Replace with:
```rust
dir_commit_crc(
    ctx,
    block_idx,
    &mut off,
    &mut ptag,
    &mut crc,
    /* begin: first byte after revision, or 4 if we wrote rev */ 4,
    block_size,
    prog_size,
    disk_version,
)?;
```

Note: `begin` for CRC verification is the start of the commit. For a fresh dir (off==4 initially), we write rev at 0..4, so begin=0. For append, we're continuing from dir.off — but the CRC covers from "begin" of this commit. In C, relocatingcommit sets `commit.begin = dir->off` — the start of the append. So begin = 4 when we wrote rev in this call, or = dir.off when we didn't (appending to existing). Actually: when off==4 we write rev, so the commit starts at 0. When off>4 we're appending, commit starts at... we need to track "begin" — the offset where this commit's data starts. For dir_commit_append, begin is 4 if we wrote rev (fresh), else the initial dir.off from entry. So: `let begin = if initial_off == 4 { 0 } else { initial_off }` — no, when we write rev we start at 0. So begin=0 when we wrote rev. For append without rev, begin = dir.off at entry. Pass `begin` into dir_commit_crc.

### 3. Update CRC_MIN / space reservation

C uses `end = metadata_max - 8` for attrs. Our CRC phase needs at least 20 bytes (5 words) for alignment. The dir_commit_crc will extend from current off to align_up. Ensure we don't allow attrs to run past `block_size - 20` or similar. C's end is metadata_max-8. We use CRC_MIN=20. The constraint: after attrs, we need `off + 20 <= block_size` at minimum for the CRC loop to have room. Actually the CRC loop's end = align_up(min(off+20, block_size), prog_size). If off = block_size - 8, then end = align_up(block_size, prog_size) = block_size. So we'd have noff going to block_size. The loop would run once. So we need off + 8 <= block_size at least (one CCRC). C's -8 leaves 8 bytes. Our CRC_MIN=20 is more conservative. Keep CRC_MIN or adjust to match C (block_size - 8 for metadata_max case).

### 4. Callers of dir_commit_append

Will need to pass disk_version. That happens in phase 5; for now, callers can pass `crate::superblock::DISK_VERSION` or 0x0002_0001 as a stub. Actually phase 5 is "thread disk_version through". So in this phase we add the param and have callers pass a constant (DISK_VERSION) — all current callers use our format. Phase 5 will wire MountState.disk_version.

## Validate

```bash
cd lp-littlefs && cargo test
cargo fmt
```
