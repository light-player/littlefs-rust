# Phase 7b: test_dirs Seek and Remove-Read

## Scope

Implement 3 test_dirs cases: remove_read, seek, toot_seek. No power-loss.

**Rules:**

- **Implement tests only.** No bug fixes.
- **Match C exactly.** Read `reference/tests/test_dirs.toml` for each case.
- **If tests fail, ignore them.** `#[ignore = "…"]` is fine. Fix bugs later.

## Reference

`reference/tests/test_dirs.toml`

## Cases

### test_dirs_remove_read

```
defines.N = 10
if = 'N < BLOCK_COUNT/2'
```

Create N dirs under `prickly-pear/`. Nested loop over k, j: open dir, iterate to j, remove dir k, iterate rest, close, recreate k, unmount. Match C logic exactly. Requires `lfs_dir_seek` or equivalent for the iteration pattern; if not available, implement the C loop and `#[ignore]` if it fails.

### test_dirs_seek

```
defines.COUNT = [4, 128, 132]
if = 'COUNT < BLOCK_COUNT/2'
```

Create COUNT entries in a child dir. Exercise `lfs_dir_seek`, `lfs_dir_tell`, `lfs_dir_rewind`. Match C exactly. Requires these APIs to be exposed in lp-littlefs.

### test_dirs_toot_seek

```
defines.COUNT = [4, 128, 132]
```

Same as seek but on root directory. Match C exactly.
