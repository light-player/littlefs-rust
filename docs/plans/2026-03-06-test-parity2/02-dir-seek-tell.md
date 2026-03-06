# Phase 2: Implement lfs_dir_seek / lfs_dir_tell

## Scope

Implement the four stubbed dir-position functions. Unblocks 3 tests.

## Reference

`reference/lfs.c` lines 2817-2857.

## Functions to implement

### `lfs_dir_seek_` (lfs.c:2817-2852)

```c
static int lfs_dir_seek_(lfs_t *lfs, lfs_dir_t *dir, lfs_off_t off) {
    int err = lfs_dir_rewind_(lfs, dir);
    if (err) { return err; }
    dir->pos = lfs_min(2, off);
    off -= dir->pos;
    dir->id = (off > 0 && lfs_pair_cmp(dir->head, lfs->root) == 0);
    while (off > 0) {
        if (dir->id == dir->m.count) {
            if (!dir->m.split) { return LFS_ERR_INVAL; }
            err = lfs_dir_fetch(lfs, &dir->m, dir->m.tail);
            if (err) { return err; }
            dir->id = 0;
        }
        int diff = lfs_min(dir->m.count - dir->id, off);
        dir->id += diff;
        dir->pos += diff;
        off -= diff;
    }
    return 0;
}
```

Translate to `src/dir/open.rs`, replacing the `todo!("lfs_dir_seek_")` stub.

### `lfs_dir_tell_` (lfs.c:2854-2857)

```c
static lfs_soff_t lfs_dir_tell_(lfs_t *lfs, lfs_dir_t *dir) {
    (void)lfs;
    return dir->pos;
}
```

Translate to `src/dir/open.rs`, replacing the `todo!("lfs_dir_tell_")` stub.

### Public wrappers (src/lib.rs)

`lfs_dir_seek` and `lfs_dir_tell` are public wrappers that call the internal `_` variants. They currently have `todo!()`. Update them to delegate, matching the pattern used by `lfs_dir_rewind` and `lfs_dir_read`.

## Tests unblocked

| Test | Status after |
|------|-------------|
| `test_dirs_remove_read` | Remove `#[ignore]` |
| `test_dirs_seek` | Remove `#[ignore]` |
| `test_dirs_toot_seek` | Remove `#[ignore]` |

If any test fails due to a bug (not a missing API), keep `#[ignore = "..."]` with a descriptive reason.

## Validate

```bash
cargo test -p lp-littlefs-core test_dirs_seek
cargo test -p lp-littlefs-core test_dirs_toot_seek
cargo test -p lp-littlefs-core test_dirs_remove_read
```
