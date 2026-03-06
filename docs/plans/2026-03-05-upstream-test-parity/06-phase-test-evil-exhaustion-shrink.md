# Phase 6: test_evil + test_exhaustion + test_shrink

## Scope

Implement all cases in these three new files. Evil exercises corruption detection. Exhaustion exercises wear leveling. Shrink exercises block-count reduction.

## Code Organization Reminders

- Place upstream cases first
- Include upstream defines + summary comment on every test
- Keep related functionality grouped together

## test_evil.rs — 8 cases

These tests corrupt metadata on disk via `write_block_raw` or `lfs_dir_commit` with forged tags, then check that mount/open/stat return appropriate errors.

### test_evil_invalid_tail_pointer

```
defines.TAIL_TYPE = [LFS_TYPE_HARDTAIL, LFS_TYPE_SOFTTAIL]
defines.INVALSET = [0x3, 0x1, 0x2]
```

Format, then commit a TAIL_TYPE tag with invalid pair `[INVALSET, INVALSET]` to root metadata. Expect `lfs_mount` to return `LFS_ERR_CORRUPT`.

### test_evil_invalid_dir_pointer

```
defines.INVALSET = [0x3, 0x1, 0x2]
```

Format, create "dir_here", commit a DIRSTRUCT tag with invalid pair to "dir_here". Mount succeeds, `lfs_stat("dir_here")` works, but `lfs_dir_open("dir_here")`, `lfs_stat("dir_here/child")`, and `lfs_file_open("dir_here/child")` fail with `LFS_ERR_CORRUPT`.

### test_evil_invalid_file_pointer

```
defines.SIZE = [10, 1000, 100000]
```

Create "file_here" with SIZE bytes. Corrupt its CTZSTRUCT to point at block 0xcccccccc with faked size. Mount + stat succeed. `lfs_file_open + lfs_file_read` fails with `LFS_ERR_CORRUPT`. If SIZE > 2*BLOCK_SIZE, `lfs_mkdir` also fails (GC triggers corrupt read).

### test_evil_invalid_ctz_pointer

```
defines.SIZE = [2*BLOCK_SIZE, 3*BLOCK_SIZE, 4*BLOCK_SIZE]
```

Create file of SIZE bytes. Corrupt the CTZ skip-list head block by writing invalid block pointers into it. Mount + stat succeed. File read fails with `LFS_ERR_CORRUPT`. If SIZE > 2*BLOCK_SIZE, mkdir also fails.

### test_evil_invalid_gstate_pointer

```
defines.INVALSET = [0x3, 0x1, 0x2]
```

Format, corrupt gstate via `lfs_fs_prepmove` with invalid move pointer. Mount may succeed but first `lfs_mkdir("should_fail")` fails with `LFS_ERR_CORRUPT`.

### test_evil_mdir_loop

No defines. Change root tail to point at `(0, 1)` (itself), forming a 1-length metadata loop. Expect mount to fail with `LFS_ERR_CORRUPT`.

### test_evil_mdir_loop2

No defines. Create "child" dir. Corrupt child's tail to point at root `(0, 1)`, forming a 2-length loop. Expect mount to fail with `LFS_ERR_CORRUPT`.

### test_evil_mdir_loop_child

No defines. Create "child" dir. Corrupt child's tail to point at itself (child's own block pair), forming a 1-length child loop. Expect mount to fail with `LFS_ERR_CORRUPT`.

## test_exhaustion.rs — 5 cases

All use `WearLevelingBd` from Phase 2 with erase-cycle tracking.

### test_exhaustion_normal

```
defines.ERASE_CYCLES = 10
defines.ERASE_COUNT = 256
defines.BLOCK_CYCLES = ERASE_CYCLES / 2  (= 5)
defines.BADBLOCK_BEHAVIOR = [PROGERROR, ERASEERROR, READERROR, PROGNOOP, ERASENOOP]
defines.FILES = 10
```

Loop: write random files under "roadrunner/" until NOSPC. Each cycle: create `roadrunner/test{cycle}_{file}` with random content. After NOSPC, read all files back. After exhaustion, remount, stat and read all surviving files.

### test_exhaustion_superblocks

Same defines. Files in root (no "roadrunner/"), forcing superblock expansion. Same exhaustion/verify logic.

### test_exhaustion_wear_leveling

```
defines.ERASE_CYCLES = 20
defines.ERASE_COUNT = 256
defines.BLOCK_CYCLES = ERASE_CYCLES / 2  (= 10)
defines.FILES = 10
```

Run exhaustion twice: first with BLOCK_COUNT/2 usable blocks, then with full device. Assert that doubling usable blocks yields >= 2× cycles (within 10% tolerance).

### test_exhaustion_wear_leveling_superblocks

Same defines. Root-level files (superblock expansion). Same doubling assertion.

### test_exhaustion_wear_distribution

```
defines.ERASE_CYCLES = 0xffffffff
defines.ERASE_COUNT = 256
defines.BLOCK_CYCLES = [5, 4, 3, 2, 1]
defines.CYCLES = 100
defines.FILES = 10
if = 'BLOCK_CYCLES < CYCLES/10'
```

Run CYCLES write cycles (or until NOSPC). After exhaustion, read per-block wear counts. Compute standard deviation of wear. Assert `stddev² < 8` (even distribution).

## test_shrink.rs — 2 cases

Guard: requires `LFS_SHRINKNONRELOCATING` (feature flag). If not present, all tests get `#[ignore = "requires LFS_SHRINKNONRELOCATING"]`.

### test_shrink_simple

```
defines.BLOCK_COUNT = [10, 15, 20]
defines.AFTER_BLOCK_COUNT = [5, 10, 15, 19]
if = 'AFTER_BLOCK_COUNT <= BLOCK_COUNT'
```

Format on BLOCK_COUNT blocks. Call `lfs_fs_grow(AFTER_BLOCK_COUNT)`. If AFTER_BLOCK_COUNT != BLOCK_COUNT: mount with original config fails, mount with reduced config succeeds.

### test_shrink_full

```
defines.BLOCK_COUNT = [10, 15, 20]
defines.AFTER_BLOCK_COUNT = [5, 7, 10, 12, 15, 17, 20]
defines.FILES_COUNT = [7, 8, 9, 10]
if = 'AFTER_BLOCK_COUNT <= BLOCK_COUNT && FILES_COUNT + 2 < BLOCK_COUNT'
```

Create FILES_COUNT+1 files of BLOCK_SIZE-0x40 bytes. Call `lfs_fs_grow(AFTER_BLOCK_COUNT)`. On success: verify all files and mount with reduced config. On `LFS_ERR_NOTEMPTY`: expect shrink to fail (too many files for smaller device).

## Validate

```
cargo test -p lp-littlefs test_evil -- --nocapture
cargo test -p lp-littlefs test_exhaustion -- --nocapture
cargo test -p lp-littlefs test_shrink -- --nocapture
cargo test -p lp-littlefs 2>&1
cargo fmt -p lp-littlefs
cargo clippy -p lp-littlefs
```
