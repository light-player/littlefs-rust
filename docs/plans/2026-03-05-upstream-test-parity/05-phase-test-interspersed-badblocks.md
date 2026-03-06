# Phase 5: test_interspersed + test_badblocks

## Scope

Implement all cases in these two new files. Interspersed exercises multi-file I/O. Badblocks exercises worn-block handling with all 5 behaviors.

## Code Organization Reminders

- Place upstream cases first
- Include upstream defines + summary comment on every test
- Keep related functionality grouped together

## test_interspersed.rs — 4 cases

### test_interspersed_files

```
defines.SIZE = [10, 100]
defines.FILES = [4, 10, 26]
```

Open FILES files ("a","b",...,"z"), write SIZE bytes to each in round-robin (1 byte per iteration), close all. Verify directory listing (FILES + 2 for . and ..). Check each file has SIZE bytes, read back first 10 bytes from each.

Letter naming: `(b'a' + i) as char`.

### test_interspersed_remove_files

```
defines.SIZE = [10, 100]
defines.FILES = [4, 10, 26]
```

Create FILES files with SIZE bytes each. Open "zzz", write one byte and sync, remove one of the FILES-lettered files, repeat. After removing all, verify "zzz" has FILES bytes and directory listing is correct.

### test_interspersed_remove_inconveniently

```
defines.SIZE = [10, 100]
```

Open three files "e","f","g". Write SIZE/2 bytes to each. Remove "f" while all three are still open. Write another SIZE/2 bytes to all three (including removed "f" — write should succeed on the open handle). Close all. Verify directory: "e" and "g" present, "f" absent. Read "e" and "g", verify SIZE bytes.

### test_interspersed_reentrant_files

```
defines.SIZE = [10, 100]
defines.FILES = [4, 10, 26]
defines.POWERLOSS_BEHAVIOR = [NOOP, OOO]
```

Mount-or-format. Open FILES files for append. Write SIZE bytes per file with sync after each byte when size ≤ i. Close. Verify directory and read 10 bytes from each. Power-loss between writes.

## test_badblocks.rs — 4 cases

File-level guard: all cases require `block_cycles == -1` (no wear leveling in the FS config; wear is in the BD).

### Common setup

All four non-superblock cases share:

```
defines.ERASE_COUNT = 256
defines.ERASE_CYCLES = 0xffffffff
defines.ERASE_VALUE = [0x00, 0xff, -1]
defines.BADBLOCK_BEHAVIOR = [PROGERROR, ERASEERROR, READERROR, PROGNOOP, ERASENOOP]
defines.NAMEMULT = 64
defines.FILEMULT = 1
```

Use the `BadBlockBehavior` enum and `WearLevelingBd` from Phase 2.

#### Pattern for bad-block setup

```rust
let mut bd = WearLevelingBd::new(BLOCK_COUNT, BLOCK_SIZE, ERASE_CYCLES);
for i in 0..BAD_BLOCK_COUNT {
    bd.set_wear(block_index(i), 0xffffffff); // worn out
}
// Format, mount, exercise, verify
```

### test_badblocks_single

For each block b in 2..BLOCK_COUNT: mark block b as worn (0xffffffff) and block b-1 as fresh (0). Format, mount. Create 9 dirs ("dir0".."dir8") with files of sizes `NAMEMULT * (i+1)` containing repeated patterns. Unmount. Remount. stat/read all dirs and files.

Inner loop:
```rust
for b in 2..block_count {
    // reset BD, mark block b worn
    // format, create dirs/files, unmount, remount, verify
}
```

### test_badblocks_region_corruption

Mark half the blocks as worn: blocks `i*2 + 2` for `i in 0..(BLOCK_COUNT-2)/2`. Format, create same 9 dirs + files, unmount, remount, verify all.

### test_badblocks_alternating_corruption

Mark every other block: blocks `2*i + 2` for `i in 0..(BLOCK_COUNT-2)/2`. Format, create, remount, verify.

### test_badblocks_superblocks

```
defines.ERASE_CYCLES = 0xffffffff
defines.ERASE_VALUE = [0x00, 0xff, -1]
defines.BADBLOCK_BEHAVIOR = [PROGERROR, ERASEERROR, READERROR, PROGNOOP, ERASENOOP]
```

Mark blocks 0 and 1 (superblocks) as worn. Expect `lfs_format` to fail with `LFS_ERR_NOSPC`. Expect `lfs_mount` to fail with `LFS_ERR_CORRUPT`.

## Validate

```
cargo test -p lp-littlefs test_interspersed -- --nocapture
cargo test -p lp-littlefs test_badblocks -- --nocapture
cargo test -p lp-littlefs 2>&1
cargo fmt -p lp-littlefs
cargo clippy -p lp-littlefs
```
