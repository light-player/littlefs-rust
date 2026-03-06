# Phase 3: test_files — Large File I/O

## Scope

Implement all missing and replace all weak upstream cases in `test_files.rs`. This is the highest-priority phase — `test_files_large` alone would have caught the `lfs_ctz_find` offset bug.

## Code Organization Reminders

- Place upstream cases first, extras at the bottom
- Include upstream defines + summary comment on every test
- Keep related functionality grouped together
- Any temporary code should have a TODO comment

## Cases to Implement

### test_files_simple — upgrade to parameterized

Current test uses fixed "Hello World!" string. Replace with upstream version:
- `defines.INLINE_MAX = [0, -1, 8]`
- Same logic: write 13-byte string, unmount, mount, read, verify.

### test_files_large — NEW (highest priority)

```
defines.SIZE = [32, 8192, 262144, 0, 7, 8193]
defines.CHUNKSIZE = [31, 16, 33, 1, 1023]
defines.INLINE_MAX = [0, -1, 8]
```

Write SIZE bytes of PRNG(seed=1) data in CHUNKSIZE chunks, close, unmount. Mount, open read-only, verify file_size == SIZE, read back in CHUNKSIZE chunks and verify against PRNG(seed=1). Final read past EOF returns 0.

Use `write_prng_file` and `verify_prng_file` from Phase 2.

### test_files_rewrite — NEW

```
defines.SIZE1 = [32, 8192, 131072, 0, 7, 8193]
defines.SIZE2 = [32, 8192, 131072, 0, 7, 8193]
defines.CHUNKSIZE = [31, 16, 1]
defines.INLINE_MAX = [0, -1, 8]
```

1. Write SIZE1 bytes PRNG(seed=1), close, unmount
2. Mount, read back, verify SIZE1 bytes of PRNG(seed=1), close, unmount
3. Mount, open WRONLY (no TRUNC), write SIZE2 bytes PRNG(seed=2), close, unmount
4. Mount, read back: first SIZE2 bytes match PRNG(seed=2). If SIZE1 > SIZE2, remaining bytes (SIZE2..SIZE1) match PRNG(seed=1) starting at offset SIZE2. Final read returns 0.

### test_files_append — replace existing

```
defines.SIZE1 = [32, 8192, 131072, 0, 7, 8193]
defines.SIZE2 = [32, 8192, 131072, 0, 7, 8193]
defines.CHUNKSIZE = [31, 16, 1]
defines.INLINE_MAX = [0, -1, 8]
```

Write SIZE1 PRNG(1), close, unmount. Mount, open APPEND, write SIZE2 PRNG(2), close, unmount. Mount, read: first SIZE1 matches PRNG(1), next SIZE2 matches PRNG(2). File size == SIZE1 + SIZE2.

### test_files_truncate — replace existing

Same defines as append. Write SIZE1 PRNG(1), close, unmount. Mount, open TRUNC|WRONLY, write SIZE2 PRNG(2), close, unmount. Mount, read: SIZE2 bytes match PRNG(2), file_size == SIZE2. Final read returns 0.

### test_files_reentrant_write — NEW

```
defines.SIZE = [32, 0, 7, 2049]
defines.CHUNKSIZE = [31, 16, 65]
defines.INLINE_MAX = [0, -1, 8]
defines.POWERLOSS_BEHAVIOR = [NOOP, OOO]
```

Uses power-loss infrastructure. Mount-or-format, check existing file (size 0 or SIZE), write SIZE PRNG(1), close, read back, verify. Each power-loss interruption restores BD and retries.

### test_files_reentrant_write_sync — NEW

Complex three-mode test (APPEND, TRUNC, plain write). Stub details from TOML. Power-loss after each sync.

### test_files_many — upgrade parameterization

Current test uses small N. Upgrade to N=300 matching upstream.

### test_files_many_power_cycle — NEW

N=300 files. Unmount/remount after creating each file. Verify all files on final mount.

### test_files_many_power_loss — NEW

```
defines.N = 300
defines.POWERLOSS_BEHAVIOR = [NOOP, OOO]
```

Reentrant creation of 300 files with power-loss simulation.

## Extras to Keep

Move to bottom of file under `// ── Rust-specific extras ──`:
- `test_files_same_session`
- `test_files_simple_read`
- `test_files_seek_tell`
- `test_files_truncate_api`

## Validate

```
cargo test -p littlefs-rust test_files -- --nocapture
# All test_files_* should pass (non-ignored)

cargo test -p littlefs-rust 2>&1
# Full suite still passes

cargo fmt -p littlefs-rust
cargo clippy -p littlefs-rust
```
