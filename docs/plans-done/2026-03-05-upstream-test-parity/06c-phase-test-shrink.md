# Phase 6c: test_shrink

## Scope

Implement 2 cases in `test_shrink.rs`. These exercise block-count reduction via `lfs_fs_grow` (shrink path).

Guard: requires `LFS_SHRINKNONRELOCATING` feature flag. If not present, all tests get `#[ignore = "requires LFS_SHRINKNONRELOCATING"]`.

## Code Organization Reminders

- Place upstream cases first
- Include upstream defines + summary comment on every test
- Keep related functionality grouped together

## Reference

- `reference/tests/test_shrink.toml`

## test_shrink.rs — 2 cases

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
cargo test -p littlefs-rust test_shrink -- --nocapture
cargo test -p littlefs-rust 2>&1
cargo fmt -p littlefs-rust
cargo clippy -p littlefs-rust
```
