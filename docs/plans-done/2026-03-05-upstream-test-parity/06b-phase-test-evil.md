# Phase 6b: test_evil

## Scope

Implement all 8 cases in `test_evil.rs`. These corrupt metadata on disk via raw block writes or `lfs_dir_commit` with forged tags, then verify mount/open/stat return appropriate errors.

## Code Organization Reminders

- Place upstream cases first
- Include upstream defines + summary comment on every test
- Keep related functionality grouped together

## Reference

- `reference/tests/test_evil.toml`

## test_evil.rs — 8 cases

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

## Validate

```
cargo test -p lp-littlefs test_evil -- --nocapture
cargo test -p lp-littlefs 2>&1
cargo fmt -p lp-littlefs
cargo clippy -p lp-littlefs
```
