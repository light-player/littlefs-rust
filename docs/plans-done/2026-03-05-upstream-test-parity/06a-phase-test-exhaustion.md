# Phase 6a: test_exhaustion

## Scope

Implement all 5 cases in `test_exhaustion.rs`. These exercise wear leveling and block exhaustion using `WearLevelingBd` from Phase 2/5.

## Code Organization Reminders

- Place upstream cases first
- Include upstream defines + summary comment on every test
- Keep related functionality grouped together

## Reference

- `reference/tests/test_exhaustion.toml`

## test_exhaustion.rs — 5 cases

All use `WearLevelingBd` with erase-cycle tracking.

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

Run exhaustion twice: first with BLOCK_COUNT/2 usable blocks, then with full device. Assert that doubling usable blocks yields >= 2x cycles (within 10% tolerance).

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

Run CYCLES write cycles (or until NOSPC). After exhaustion, read per-block wear counts. Compute standard deviation of wear. Assert `stddev^2 < 8` (even distribution).

## Validate

```
cargo test -p littlefs-rust test_exhaustion -- --nocapture
cargo test -p littlefs-rust 2>&1
cargo fmt -p littlefs-rust
cargo clippy -p littlefs-rust
```
