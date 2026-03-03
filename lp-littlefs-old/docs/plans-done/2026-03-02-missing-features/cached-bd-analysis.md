# test_dirs_other_errors: Cached Block Device Analysis

## Summary

The test fails: after `mkdir("potato")` + `file_open("burito", CREAT)` + `file_close` + `bd.sync()` + unmount + remount, the filesystem appears empty. `mkdir("potato")` succeeds instead of returning `Error::Exist`.

**Key finding:** The failure is **not** specific to `CachedBlockDevice`. Running the same sequence with uncached `RamBlockDevice` also fails. The bug is in the FS layer (mkdir + file-create interaction), not in the cache.

## Why Upstream Passes

1. **Cache placement:** Upstream keeps `pcache` and `rcache` inside the `lfs` struct, not in the block device. The block device callbacks (`read`, `prog`, `erase`, `sync`) talk directly to storage. On unmount, `lfs_deinit` frees the struct; caches are discarded. On remount, a fresh `lfs` is created with empty caches.

2. **Test does not call `cfg->sync()` before unmount:**

   ```c
   lfs_file_close(&lfs, &file) => 0;
   lfs_unmount(&lfs) => 0;   // no sync
   lfs_mount(&lfs, cfg) => 0;
   ```

   Upstream relies on `lfs_file_close` flushing via `lfs_file_sync_` → `lfs_bd_sync`. Our test explicitly calls `bd.sync()` before unmount.

3. **Same `lfs` struct across unmount/remount:** Upstream reuses the `lfs_t` instance, so the block device handle is the same. Our `LittleFs` is also reused; the block device persists. The difference is our cache lives in `CachedBlockDevice`, which outlives unmount.

## What Works vs. Fails

| Scenario                              | Result |
|--------------------------------------|--------|
| mkdir only + remount                  | PASS   |
| file create only + remount            | PASS (`test_files_simple`) |
| mkdir + file create + remount         | FAIL   |

So:

- mkdir persistence is fine.
- File-create-only persistence is fine.
- The combination mkdir then file-create breaks persistence.

## Trace Observations

From `RUST_LOG=lp_littlefs=trace`:

- `dir_commit_append pair=[0,1] n_attrs=4 off=140` (mkdir potato) and later commits to root run as expected.
- `cache flush` is invoked correctly when switching blocks.
- Before the failing `mkdir("potato")`, mount reads the root and sees `count=1` (superblock only), so potato and burito are missing.

## CachedBlockDevice Behavior (Reference)

`cache.rs` implements a single-slot program cache:

- On `prog(block, off, data)`: if the block matches the cached block and the range fits, merge into the cache; otherwise flush, then load the new block with a read-before-write (required for RAM-like devices).
- On `sync()`: drop rcache, flush pcache, call `device.sync()`.
- Mount calls `bd.sync()` at the start to ensure a clean view.

Upstream `lfs_bd_prog` does **not** read-before-write; it assumes flash semantics (program only clears bits). Our read-before-write is appropriate for RamBlockDevice.

## Conclusion

The root cause is in the **FS logic**, not in `CachedBlockDevice`:

- The mkdir + file-create sequence leads to a state that looks empty after remount.
- Because the same failure occurs with uncached `RamBlockDevice`, the cache is not the source.

**Next steps:**

1. Add a focused test: mkdir + file_create + remount with uncached bd, and assert root contents.
2. Trace the exact commit and block usage during file_open(CREAT) and file_close when the parent dir was just modified by mkdir.
3. Check whether the file-create path uses stale `root` or dir state, or triggers a relocation that mis-updates the superblock/tail.
4. Compare with upstream `lfs_file_opencfg_` / `lfs_dir_commit` flow for creating a file in a dir that was just created.
