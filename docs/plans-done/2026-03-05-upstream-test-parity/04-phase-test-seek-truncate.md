# Phase 4: test_seek + test_truncate

## Scope

Implement all cases in the two new files. These exercise file position and size mutations.

## Code Organization Reminders

- Place upstream cases first
- Include upstream defines + summary comment on every test
- Keep related functionality grouped together

## test_seek.rs — 10 cases

### test_seek_read

```
defines = [{COUNT=132, SKIP=4}, {COUNT=132, SKIP=128}, {COUNT=200, SKIP=10},
           {COUNT=200, SKIP=100}, {COUNT=4, SKIP=1}, {COUNT=4, SKIP=2}]
```

Write COUNT copies of "kittycatcat" (11 bytes each). Unmount, remount read-only.
- Skip to SKIP via seek(SKIP * 11, SEEK_SET), read "kittycatcat"
- Rewind, read first "kittycatcat"
- seek(0, SEEK_CUR), read next
- seek(SKIP * 11, SEEK_CUR), read at that offset
- seek(-11, SEEK_END), read last
- lfs_file_size == COUNT * 11

### test_seek_write

Same defines as test_seek_read. Open RDWR, seek to SKIP * 11, overwrite with "doggodogdog". Verify read-back at that position. Rewind, read first entry. Seek to end, verify.

### test_seek_boundary_read

`defines.COUNT = 132`

Build a file of COUNT × 11 bytes. Test seek/read at offsets: 512, 1024-4, 1024-3, 1024-2, 1024-1, 1024, 1024+1, 1024+2, 1024+3, 1024+4, and COUNT*11 - 11. At each offset, seek, read 11 bytes, verify against expected content. Also test read after sync.

### test_seek_boundary_write

`defines.COUNT = 132`

Same offsets as boundary_read. Write "hedgehoghog" at each offset, verify reads at offset 0 and at the written offset, both before and after sync.

### test_seek_out_of_bounds

```
defines = [{COUNT=132, SKIP=4}, {COUNT=132, SKIP=128}, {COUNT=200, SKIP=10},
           {COUNT=200, SKIP=100}, {COUNT=4, SKIP=2}, {COUNT=4, SKIP=3}]
```

Seek past EOF to `(COUNT+SKIP) * 11`, write "porcupineee". Read back: first COUNT*11 is "kittycatcat" data, middle is zeros (hole), last 11 is "porcupineee". Backward seek that would go before 0 returns `LFS_ERR_INVAL`.

### test_seek_inline_write

`defines.SIZE = [2, 4, 128, 132]`

Open RDWR, write SIZE bytes one-by-one, check seek/tell/size on each. Second pass with interleaved writes and seek(-1)/read(3) checks.

### test_seek_reentrant_write

```
defines.COUNT = [4, 64, 128]
defines.POWERLOSS_BEHAVIOR = [NOOP, OOO]
```

Mount-or-format. If "kitty" exists and non-empty, verify content. If empty, fill with COUNT * "kittycatcat". Quadratic-probing seek/write "doggodogdog" pattern. Power-loss interruptions between writes.

### test_seek_filemax

No defines. Seek to LFS_FILE_MAX with SEEK_SET and SEEK_CUR. Then seek +10 from SEEK_END, check position.

### test_seek_underflow

No defines. Backward seeks that underflow: SEEK_CUR(-1) on fresh file, SEEK_END(-size-1). Expect LFS_ERR_INVAL, file pointer unchanged.

### test_seek_overflow

No defines. Seek to LFS_FILE_MAX, then SEEK_CUR(+10), SEEK_CUR(+LFS_FILE_MAX), SEEK_SET(LFS_FILE_MAX+1). Expect LFS_ERR_INVAL, pointer stays at LFS_FILE_MAX.

## test_truncate.rs — 7 cases

### test_truncate_simple

```
defines.MEDIUMSIZE = [31, 32, 33, 511, 512, 513, 2047, 2048, 2049]
defines.LARGESIZE = [32, 33, 512, 513, 2048, 2049, 8192, 8193]
if = 'MEDIUMSIZE < LARGESIZE'
```

Write LARGESIZE bytes of "hair" (`b"hair"` repeating), truncate to MEDIUMSIZE, unmount, remount, verify file_size == MEDIUMSIZE and content.

### test_truncate_read

Same defines + if. Same setup, but also read truncated portion before unmount and again after remount.

### test_truncate_write_read

No defines. Write a sequential buffer, truncate off last quarter, read first 3/4, seek to 1/4, truncate to half, read second quarter, verify.

### test_truncate_write

Same defines + if. Write LARGESIZE, truncate to MEDIUMSIZE, overwrite with "bald" up to MEDIUMSIZE, remount, verify.

### test_truncate_reentrant_write

```
defines.SMALLSIZE = [4, 512]
defines.MEDIUMSIZE = [0, 3, 4, 5, 31, 32, 33, 511, 512, 513, 1023, 1024, 1025]
defines.LARGESIZE = 2048
defines.POWERLOSS_BEHAVIOR = [NOOP, OOO]
```

Reentrant: mount-or-format, read "baldy" (content is "hair"/"bald"/"comb"), write LARGESIZE "hair", truncate to MEDIUMSIZE, write "bald", truncate to SMALLSIZE, write "comb". Power-loss between operations.

### test_truncate_aggressive

```
defines.CONFIG = range(6)
defines.SMALLSIZE = 32
defines.MEDIUMSIZE = 2048
defines.LARGESIZE = 8192
```

6 configs exercising cold/warm shrink/expand and mid-file variants. Create 5 files, truncate with varying seek positions, remount, verify.

Inner loop over `0..6`:
```rust
for config in 0..6 {
    // ...
}
```

### test_truncate_nop

```
defines.MEDIUMSIZE = [32, 33, 512, 513, 2048, 2049, 8192, 8193]
```

Write MEDIUMSIZE bytes while truncating to current write position (no-op), truncate to MEDIUMSIZE, verify. Remount and re-check.

## Validate

```
cargo test -p littlefs-rust test_seek -- --nocapture
cargo test -p littlefs-rust test_truncate -- --nocapture
cargo test -p littlefs-rust 2>&1
cargo fmt -p littlefs-rust
cargo clippy -p littlefs-rust
```
