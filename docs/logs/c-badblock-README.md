# C Bad-Block Harness: Build, Run, Compare

A/B debugging for the bad-block read bug. The C harness runs the same scenario as `run_badblocks_minimal` and traces BD operations for comparison with Rust.

## Prerequisites

- littlefs C source at `reference/` (symlink to `../../oss/littlefs`) or override `LFS_SRC`
- Commit in [docs/reference.md](../reference.md)

## Build

From repo root:

```bash
make -C docs/c_reference badblock_and_dump
```

Or from `docs/c_reference`:

```bash
make badblock_and_dump
```

Override C source path if needed:

```bash
make -C docs/c_reference badblock_and_dump LFS_SRC=../../reference
```

## Run and Capture Log

```bash
./docs/c_reference/badblock_and_dump 2>&1 | tee docs/logs/c-badblock-$(date +%Y%m%d-%H%M%S).log
```

Expected: `C badblock_and_dump: SUCCESS (read N bytes)` — C completes correctly.

The log contains:
- Step markers: `--- step: pacman_fill ---`, etc.
- BD trace: `bd_read block=X off=Y size=Z` or `-> CORRUPT` when block is bad
- Block hex dumps (first 64 bytes of blocks 0–7)

## Compare with Rust

When Rust reproduces the failure (or to compare passing behavior):

```bash
RUST_LOG=lp_littlefs=trace cargo test -p lp-littlefs test_alloc_bad_blocks_minimal --features log -- --nocapture 2>&1 | tee docs/logs/rust-badblock-$(date +%Y%m%d-%H%M%S).log
```

Compare:
- BD sequence: same blocks read/progged? CORRUPT at same block?
- ctz_find / file_read behavior when reading pacman after GC
- Divergence point = likely bug

## When a Divergence Is Found

Create `docs/logs/ab-comparison-badblock-YYYYMMDD.md` (like [ab-comparison-20260304.md](ab-comparison-20260304.md)) with:
- Hypothesis
- C vs Rust log excerpts
- Proposed fix

## If BD Trace Is Too Coarse

See Phase 3 of the C Trace Bad-Block Debug Plan: add trace calls to a local copy of `lfs.c` (e.g. `lfs_ctz_find`, `lfs_file_read`, `lfs_fs_gc`).
