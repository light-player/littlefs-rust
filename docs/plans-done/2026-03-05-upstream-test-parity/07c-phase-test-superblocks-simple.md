# Phase 7c: test_superblocks Simple Cases

## Scope

Implement 8 test_superblocks cases that do not require power-loss. Simple config/mount/expand scenarios.

**Rules:**

- **Implement tests only.** No bug fixes.
- **Match C exactly.** Read `reference/tests/test_superblocks.toml` for each case.
- **If tests fail, ignore them.** `#[ignore = "…"]` is fine. Fix bugs later.

## Reference

`reference/tests/test_superblocks.toml`

## Cases

### test_superblocks_mount_unknown_block_count

No defines. Mount with `block_count = 0`, verify `lfs.block_count` is set correctly after mount.

### test_superblocks_stat_tweaked

```
defines.TWEAKED_NAME_MAX = 63
defines.TWEAKED_FILE_MAX = (1<<16)-1
defines.TWEAKED_ATTR_MAX = 512
if = 'DISK_VERSION == 0'
```

Format with custom limits, mount with default config, verify `lfs_fs_stat` returns tweaked values.

### test_superblocks_expand

```
defines.BLOCK_CYCLES = [32, 33, 1]
defines.N = [10, 100, 1000]
```

Create/remove a dummy file N times, verify superblock survives compaction.

### test_superblocks_magic_expand

Same defines. Same cycle + magic check after.

### test_superblocks_expand_power_cycle

Same defines. Unmount/remount after each iteration.

### test_superblocks_unknown_blocks

No defines. Mount with `block_count = 0`, `lfs_fs_stat`, basic operations.

### test_superblocks_fewer_blocks

```
defines.BLOCK_COUNT = [ERASE_COUNT/2, ERASE_COUNT/4, 2]
```

Format with fewer blocks than physical device. Verify mount fails with `LFS_ERR_INVAL` for wrong count.

### test_superblocks_more_blocks

Format with `2*ERASE_COUNT` blocks. Expect mount fails with `LFS_ERR_INVAL`.
