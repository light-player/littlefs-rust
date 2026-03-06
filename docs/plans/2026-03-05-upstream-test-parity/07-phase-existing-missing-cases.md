# Phase 7: Missing Cases in Existing Test Files

## Scope

Implement all upstream cases that are missing from existing Rust test files but don't require creating new files.

## Code Organization Reminders

- Place upstream cases first, extras at the bottom
- Include upstream defines + summary comment on every test
- Keep related functionality grouped together

## test_dirs.rs — 12 missing cases

### test_dirs_many_rename_append

```
defines.N = range(5, 13, 2)    → for n in (5..13).step_by(2)
if = 'N < BLOCK_COUNT/2'
```

Create N dirs ("a0".."aN"), rename each `a{i}` → `z{i}` (append-style), verify count.

### test_dirs_many_reentrant

```
defines.N = [5, 11]
defines.POWERLOSS_BEHAVIOR = [NOOP, OOO]
```

Reentrant mkdir/remove/rename loop under power-loss.

### test_dirs_file_creation

```
defines.N = range(3, 100, 11)    → for n in (3..100).step_by(11)
if = 'N < BLOCK_COUNT/2'
```

Create N empty files `test{i}`, verify via `dir_read`.

### test_dirs_file_removal

```
defines.N = range(3, 100, 11)
```

Create N files, remove all, verify empty root.

### test_dirs_file_rename

```
defines.N = range(3, 100, 11)
```

Create N files `test{i}`, rename each to `tedd{i}`, verify listing.

### test_dirs_file_reentrant

```
defines.N = [5, 25]
defines.POWERLOSS_BEHAVIOR = [NOOP, OOO]
```

Reentrant file create/remove/rename under power-loss.

### test_dirs_nested

No defines. Create nested dirs `potato/baked/sweet/fried`. Exercise removal of inner dirs, rename across dirs.

### test_dirs_recursive_remove

```
defines.N = [10, 100]
```

Create N subdirs under "prickly-pear/", then recursively remove (inner first, then parent).

### test_dirs_remove_read

```
defines.N = 10
```

Create N dirs. While iterating parent, remove dir at index k, recreate it, repeat.

### test_dirs_other_errors

No defines. Verify: EXIST on duplicate mkdir, NOENT on stat non-existent, NOTDIR on dir_open of file, ISDIR on file_open of dir.

### test_dirs_seek

```
defines.COUNT = [4, 128, 132]
if = 'COUNT < BLOCK_COUNT/2'
```

Create COUNT entries in a child dir. Exercise `lfs_dir_seek`, `lfs_dir_tell`, `lfs_dir_rewind`.

### test_dirs_toot_seek

```
defines.COUNT = [4, 128, 132]
```

Same as test_dirs_seek but on root directory.

## test_superblocks.rs — 13 missing cases

### test_superblocks_mount_unknown_block_count

No defines. Mount with `block_count = 0`, verify `lfs.block_count` is set correctly after mount.

### test_superblocks_reentrant_format

```
defines.POWERLOSS_BEHAVIOR = [NOOP, OOO]
```

Format under power-loss, then mount and verify.

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

### test_superblocks_reentrant_expand

```
defines.BLOCK_CYCLES = [2, 1]
defines.N = 24
defines.POWERLOSS_BEHAVIOR = [NOOP, OOO]
```

Reentrant superblock expand with power-loss.

### test_superblocks_unknown_blocks

No defines. Mount with `block_count = 0`, `lfs_fs_stat`, basic operations.

### test_superblocks_fewer_blocks

```
defines.BLOCK_COUNT = [ERASE_COUNT/2, ERASE_COUNT/4, 2]
```

Format with fewer blocks than physical device. Verify mount fails with `LFS_ERR_INVAL` for wrong count.

### test_superblocks_more_blocks

Format with `2*ERASE_COUNT` blocks. Expect mount fails with `LFS_ERR_INVAL`.

### test_superblocks_grow

```
defines.BLOCK_COUNT, BLOCK_COUNT_2, KNOWN_BLOCK_COUNT — specific grow scenarios
```

`lfs_fs_grow` from smaller to larger block count. Verify operations work on expanded device.

### test_superblocks_shrink

Requires `LFS_SHRINKNONRELOCATING`. Shrink via `lfs_fs_grow` to smaller size.

### test_superblocks_metadata_max

```
defines.METADATA_MAX
defines.N = [10, 100, 1000]
```

Set `metadata_max` in config during superblock compaction cycles.

## test_move.rs — 2 missing cases

### test_move_fix_relocation

```
defines.RELOCATIONS = range(4)    → for r in 0..4
defines.ERASE_CYCLES = 0xffffffff
```

Move file with `set_wear` to force directory relocation during move. Verify file content after.

### test_move_fix_relocation_predecessor

```
defines.RELOCATIONS = range(8)    → for r in 0..8
```

Move file between sibling and child dirs with forced relocations. Verify tree structure.

## test_orphans.rs — 4 missing cases

### test_orphans_normal

```
if = 'PROG_SIZE <= 0x3fe'
```

Corrupt child's commit to create an orphan directory. Run `lfs_mkdir` (triggers deorphan). Check `lfs_fs_size`.

### test_orphans_one_orphan

Create orphan via internal APIs (`lfs_dir_alloc` + `SOFTTAIL` commit + `lfs_fs_preporphans(+1)`). Run `lfs_fs_forceconsistency`. Verify orphan is cleaned up.

### test_orphans_mkconsistent_one_orphan

Same orphan creation. Use `lfs_fs_mkconsistent` + remount. Verify cleanup.

### test_orphans_reentrant

```
defines.FILES = [6, 26, 3]
defines.DEPTH = [1, 3]
defines.CYCLES = 20
defines.POWERLOSS_BEHAVIOR = [NOOP, OOO]
```

Random mkdir/remove at varying depths under power-loss. Verify tree consistency after.

## test_paths.rs — 7 missing cases

### test_paths_noent_trailing_slashes

Stat/open paths with trailing slashes on non-directories. Expect `LFS_ERR_NOENT` or `LFS_ERR_NOTDIR`.

### test_paths_noent_trailing_dots

Paths with trailing dots on non-existent entries. Expect `LFS_ERR_NOENT`.

### test_paths_noent_trailing_dotdots

Paths with trailing `..` components. Expect `LFS_ERR_NOENT`.

### test_paths_utf8_ipa

UTF-8 names using IPA symbols (ɑ, ɒ, ɓ, etc.). Create, stat, read.

### test_paths_oopsallspaces

Path composed entirely of spaces. Create dir/file, stat, read.

### test_paths_oopsalldels

Path of only DEL characters (0x7f). Create, stat, read.

### test_paths_oopsallffs

Path of only 0xff bytes. Create, stat, read.

## test_powerloss.rs — 1 missing case

### test_powerloss_partial_prog

```
defines.PROG_SIZE < BLOCK_SIZE
defines.BYTE_OFF = [0, PROG_SIZE-1, PROG_SIZE/2]
defines.BYTE_VALUE = [0x33, 0xcc]
```

Corrupt one byte in a directory block at BYTE_OFF with BYTE_VALUE to simulate partial prog. Verify mount and read/write still work (FS detects and recovers).

## Validate

```
cargo test -p lp-littlefs test_dirs -- --nocapture
cargo test -p lp-littlefs test_superblocks -- --nocapture
cargo test -p lp-littlefs test_move -- --nocapture
cargo test -p lp-littlefs test_orphans -- --nocapture
cargo test -p lp-littlefs test_paths -- --nocapture
cargo test -p lp-littlefs test_powerloss -- --nocapture
cargo test -p lp-littlefs 2>&1
cargo fmt -p lp-littlefs
cargo clippy -p lp-littlefs
```
