# Phase 7: Mount and unmount

## Scope of phase

Implement `mount()` in `fs/mount.rs`: read metadata pair (blocks 0, 1), find valid superblock, validate. Implement `unmount()` (sync/teardown). Wire into `LittleFs`.

## Code organization reminders

- Mount logic in dedicated module
- Parse helpers at bottom
- Handle revision comparison (both blocks may have commits)

## Implementation details

### 1. Mount flow

Per upstream lfs_mount_:
- Root pair is blocks 0, 1
- Read both blocks, find the one with valid superblock (higher revision wins; handle overflow)
- Parse superblock from commit
- Validate: magic "littlefs", version compatibility, block_size/block_count match config
- Return Ok on success, Error::Corrupt on invalid/empty

### 2. Create src/fs/mount.rs

- `mount<B: BlockDevice>(bd: &B, config: &Config) -> Result<(), Error>`
- Read block 0 and 1 into buffers
- Parse revision from each (first 4 bytes LE)
- Pick block with higher revision (sequence comparison per SPEC to handle overflow)
- Scan backwards through commits to find superblock (or forward—upstream fetches and matches LFS_TYPE_SUPERBLOCK)
- For minimal: assume first commit has superblock at known offsets
- Parse: magic at offset 8, superblock struct after that
- Validate magic == b"littlefs"
- Validate version major matches (0x0002)
- Validate block_size, block_count match config (or config has 0 for block_count = use disk)
- Return Ok(())
- On any parse/validation failure: Err(Error::Corrupt)

### 3. Unmount

For now, unmount is a no-op (sync is empty for RamBlockDevice). Later we'll need to sync caches. Return Ok(()) for now.

### 4. State

LittleFs may need to hold mount state (e.g. loaded superblock). For phase 7, mount can be stateless—we validate and return. Full mount would store state for subsequent operations. We're only testing format+mount+unmount, so minimal state is fine. LittleFs can remain empty for now.

## Validate

```bash
cd lp-littlefs && cargo build
```
