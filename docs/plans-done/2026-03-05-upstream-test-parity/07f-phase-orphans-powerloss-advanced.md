# Phase 7f: Orphans, Partial Prog, Grow/Shrink

## Scope

Implement cases that need internal APIs, raw block corruption, or grow/shrink. These are the most complex Phase 7 items.

**Rules:**

- **Implement tests only.** No bug fixes.
- **Match C exactly.** Read `reference/tests/test_*.toml` for each case.
- **If tests fail, ignore them.** `#[ignore = "…"]` is fine. Fix bugs later.

## Reference

- `reference/tests/test_orphans.toml`
- `reference/tests/test_powerloss.toml`
- `reference/tests/test_superblocks.toml`

## Cases

### test_orphans_normal (test_orphans.rs)

```
if = 'PROG_SIZE <= 0x3fe'
```

Corrupt child's commit to create an orphan directory. Run `lfs_mkdir` (triggers deorphan). Check `lfs_fs_size`. Requires raw block write to corrupt. Match C exactly.

### test_orphans_one_orphan (test_orphans.rs)

Create orphan via internal APIs (`lfs_dir_alloc` + `SOFTTAIL` commit + `lfs_fs_preporphans(+1)`). Run `lfs_fs_forceconsistency`. Verify orphan is cleaned up. These APIs may not be exposed in littlefs-rust; implement if available, else `#[ignore]` with a note.

### test_orphans_mkconsistent_one_orphan (test_orphans.rs)

Same orphan creation. Use `lfs_fs_mkconsistent` + remount. Verify cleanup. Match C exactly.

### test_powerloss_partial_prog (test_powerloss.rs)

```
defines.PROG_SIZE < BLOCK_SIZE
defines.BYTE_OFF = [0, PROG_SIZE-1, PROG_SIZE/2]
defines.BYTE_VALUE = [0x33, 0xcc]
```

Corrupt one byte in a directory block at BYTE_OFF with BYTE_VALUE to simulate partial prog. Verify mount and read/write still work. Requires raw block write. Match C exactly.

### test_superblocks_grow (test_superblocks.rs)

```
defines.BLOCK_COUNT, BLOCK_COUNT_2, KNOWN_BLOCK_COUNT — specific grow scenarios
```

`lfs_fs_grow` from smaller to larger block count. Verify operations work on expanded device. Match C exactly.

### test_superblocks_shrink (test_superblocks.rs)

Requires `LFS_SHRINKNONRELOCATING`. Shrink via `lfs_fs_grow` to smaller size. Match C exactly.

### test_superblocks_metadata_max (test_superblocks.rs)

```
defines.METADATA_MAX
defines.N = [10, 100, 1000]
```

Set `metadata_max` in config during superblock compaction cycles. Match C exactly.
