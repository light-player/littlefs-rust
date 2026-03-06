# Phase 3: Bug Fixes

## Scope

Fix two bugs that cause implemented tests to fail. Unblocks 3 tests.

## Bug 1: Superblock block_count validation on mount

**Test:** `test_superblocks_fewer_blocks`

**Expected behavior:** Format with `block_count = N` (e.g., 64), then mount with `block_count = ERASE_COUNT` (128). Mount should return `LFS_ERR_INVAL` because `cfg.block_count != superblock.block_count`.

**Current behavior:** Mount does not return `LFS_ERR_INVAL`.

**C reference:** `lfs.c:4600-4606`

```c
if (lfs->cfg->block_count
        && superblock.block_count != lfs->cfg->block_count) {
    LFS_ERROR("Invalid block count ...");
    err = LFS_ERR_INVAL;
    goto cleanup;
}
```

**Rust code:** `src/fs/mount.rs:348-351` — the check exists and looks correct:

```rust
if cfg.block_count != 0 && superblock.block_count != cfg.block_count {
    err_inner = LFS_ERR_INVAL;
    break;
}
```

**Investigation steps:**

1. Run `test_superblocks_fewer_blocks` without `#[ignore]` and check the actual error/return value
2. Check whether `clone_config_with_block_count` correctly sets the block_count on the config passed to mount
3. Check whether the BD's erase_count vs config block_count causes an earlier failure (read from non-existent block) that masks the validation
4. If the check is reached but doesn't trigger, debug why `superblock.block_count` or `cfg.block_count` has an unexpected value

## Bug 2: Space-only path handling

**Tests:** `test_paths_oopsallspaces` (2 cases: dirs and files)

**Expected behavior:** Create directories/files with names consisting entirely of spaces (`" "`, `"  "`, `"   "`, etc.) and nested paths like `" / "`. All CRUD operations should work.

**Current behavior:** Path resolution fails for space-only names.

**Investigation steps:**

1. Run `test_paths_oopsallspaces` without `#[ignore]` and capture the failure
2. The most likely issue is in path parsing/splitting — LittleFS treats spaces as valid name characters, but the path splitting might trim or mishandle them
3. Check `lfs_dir_find` / path traversal for how path components are extracted
4. Compare with C reference `lfs.c` path handling for edge cases around spaces and leading/trailing separators

**Possible causes:**
- Path component splitting trims whitespace
- Empty path component after splitting `" / "` (space-slash-space)
- Name comparison fails when name is all spaces

## Tests unblocked

| Test | Status after |
|------|-------------|
| `test_superblocks_fewer_blocks` | Remove `#[ignore]` |
| `test_paths_oopsallspaces::case_1_dirs` | Remove `#[ignore]` |
| `test_paths_oopsallspaces::case_2_files` | Remove `#[ignore]` |

## Validate

```bash
cargo test -p lp-littlefs test_superblocks_fewer_blocks
cargo test -p lp-littlefs test_paths_oopsallspaces
```
