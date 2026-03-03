# Phase 4: Integrate dir_commit_crc into dir_compact

## Scope of phase

Replace the single CCRC at the end of dir_compact with a call to dir_commit_crc. Add disk_version parameter to dir_compact (and dir_splittingcompact, dir_split as needed).

## Code organization reminders

- dir_compact writes to dir.pair[1], begin=0 (revision at 0..4)
- Pass begin=0 into dir_commit_crc for verification

## Implementation details

### 1. Add disk_version to dir_compact

```rust
pub fn dir_compact<B: BlockDevice>(
    bdc: &BdContext<'_, B>,
    dir: &mut MdDir,
    source: &MdDir,
    begin: u16,
    end: u16,
    attrs: &[CommitAttr<'_>],
    root: [u32; 2],
    lookahead: &mut Lookahead,
    gstate_ctx: &mut Option<&mut GStateCtx<'_>>,
    disk_version: u32,  // NEW
) -> Result<CompactResult, Error>
```

### 2. Replace single CCRC

Remove the 8-byte CCRC block; call dir_commit_crc with block_idx=dir.pair[1], off, ptag, crc, begin=0 (revision at 0), block_size=meta_max, prog_size.

### 3. Update callers

- dir_splittingcompact
- dir_split (calls dir_compact)
- dir_orphaningcommit path (calls dir_splittingcompact)
- fs_gc (calls dir_orphaningcommit)

Each needs disk_version. For now pass DISK_VERSION; phase 5 threads from MountState.

### 4. meta_max vs block_size

dir_compact uses meta_max (metadata_max or block_size). The CRC loop uses block_size for the end formula. In C, commit.end = metadata_max - 8. The physical block is block_size. For compact we write to one block; the usable region is meta_max. Use block_size for the CRC end formula — we're writing to a full block. Actually C's commit.block is dir->pair[1], and the block size is block_size. The end for attrs is metadata_max-8. The commitcrc uses lfs->cfg->block_size. So for the CRC loop we use block_size (the actual block size), not meta_max. The off can't exceed block_size. Use block_size from config.

## Validate

```bash
cd lp-littlefs && cargo test
cargo fmt
```
