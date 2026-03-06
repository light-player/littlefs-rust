# Upstream Test Parity — Notes

## Scope of Work

Bring the lp-littlefs Rust test suite to full parity with the upstream C littlefs test suite (`reference/tests/*.toml`). This includes:

1. Stubbing all missing tests with `todo!()` + `#[ignore]` so the gap is visible
2. Adding shared test infrastructure (PRNG, chunked I/O helpers)
3. Implementing all missing test cases with full upstream parameterization via `rstest`
4. Upgrading existing single-value tests to full upstream parameter coverage

## Current State

### C reference: ~140 test cases across 21 .toml files

Each case uses a `defines` mechanism for parameterization (e.g. `defines.SIZE = [32, 8192, 262144]`). The C test runner generates a test for every combination of define values.

### Rust port: ~95 test functions across 11 test files

**8 entire test files missing** (~40 cases):
- `test_seek` (10 cases)
- `test_truncate` (7 cases)
- `test_interspersed` (4 cases)
- `test_shrink` (2 cases)
- `test_badblocks` (4 cases)
- `test_evil` (8 cases)
- `test_exhaustion` (5 cases)
- `test_compat` (17 cases)

**~35 cases missing from existing files:**
- `test_files`: 6 missing (test_files_large, test_files_rewrite, reentrant variants, power-loss variants)
- `test_dirs`: ~11 missing (file_creation/removal/rename, nested, recursive_remove, seek, etc.)
- `test_superblocks`: ~14 missing (expand, grow, shrink, metadata_max, etc.)
- `test_move`: 2 missing + 6 ignored stubs needing implementation
- `test_orphans`: 3 missing
- `test_paths`: ~7 missing
- `test_powerloss`: 1 missing

**Parameterization incomplete:** existing tests use single fixed values where C uses arrays.

### Infrastructure present
- `rstest = "0.26"` in dev-deps (used in test_paths.rs)
- `common/mod.rs`: config builders, assert macros, `run_with_timeout`, `BadBlockRamStorage`
- `common/powerloss.rs`: power-loss simulation, snapshot/restore, `run_powerloss_linear`

### Infrastructure missing
- No PRNG (C uses xorshift32 `TEST_PRNG`)
- No chunked write/read/verify helpers
- No `INLINE_MAX` config support
- No `write_block_raw` for corruption injection (test_evil)
- `BadBlockRamStorage` only supports READERROR; C tests use 5 behaviors

## Questions

### Q1: Parameter explosion — how to handle 324-combo tests?

**Context:** `test_files_rewrite` has 6×6×3×3 = 324 parameter combinations. With `rstest`, each combo becomes a separate test function. This is faithful but makes `cargo test` output very long and potentially slow.

**Suggestion:** Use full combos as requested. `rstest` handles this well — each combo runs as a separate test, making failures easy to pinpoint. If compile time becomes an issue, we can split into separate test binaries later.

**Answer:** Use full upstream parameter sets. Parameterization conventions:
- **Simple value lists** → `#[rstest]` with `#[values(...)]` literals
- **Ranges** (`range(start, stop, step)`) → inner loop in test body using `(start..stop).step_by(step)`
- **Cross-product with filter** (`if = 'X < Y'`) → `#[values]` for all axes, early return for the `if` condition
- **Expression defines** (`SIZE = '(BLOCK_SIZE-8)*...'`) → compute in test body using same formula
- **Config-affecting defines** (`INLINE_MAX`, `COMPACT_THRESH`) → apply to config struct before format/mount

### Q2: test_compat — requires previous-version LFS support?

**Context:** `test_compat.toml` has 17 cases testing forward/backward compatibility between disk format versions. These require `LFSP_*` (previous version) types and a way to format a disk with an older version. Our codebase only implements the current version.

**Suggestion:** Stub all 17 cases with `todo!()` + `#[ignore = "requires version compat infrastructure"]`. Defer implementation to a later plan since it requires significant new infrastructure.

**Answer:** Stub all 17 with `todo!()` + `#[ignore = "requires version compat infrastructure"]`. Defer implementation.

### Q3: test_exhaustion — requires erase-cycle tracking?

**Context:** These tests need the BD layer to track erase cycles per block and fail after N cycles. Our `RamStorage` doesn't track this. The C test framework uses `lfs_emubd` with `erase_cycles` and `wear_count` arrays.

**Suggestion:** Stub with `todo!()` + `#[ignore]`. Implementation requires adding a `WearLevelingBd` wrapper to common/ that tracks per-block erase counts and fails after the configured limit.

**Answer:** Implement `WearLevelingBd` wrapper as part of the infrastructure phase.

### Q4: test_evil — requires raw block writes for corruption injection?

**Context:** These tests write invalid pointers/data directly to disk blocks, then verify the FS detects the corruption. This requires bypassing the normal FS API to write raw bytes to specific blocks.

**Suggestion:** Add a `write_block_raw(config, block, off, data)` helper to common/. The existing `read_block_raw` already exists; the write variant is straightforward.

**Answer:** Yes — add `write_block_raw` to `common/mod.rs`.

### Q5: BadBlock behaviors — extend BadBlockRamStorage?

**Context:** C tests use 5 bad-block behaviors: PROGERROR, ERASEERROR, READERROR, PROGNOOP, ERASENOOP. Our `BadBlockRamStorage` only supports READERROR.

**Suggestion:** Extend `BadBlockRamStorage` with a `BadBlockBehavior` enum and implement all 5 variants.

**Answer:** Yes — extend `BadBlockRamStorage` with all 5 behaviors.

### Q6: Stub-first approach — structure?

**Context:** User wants to stub all missing tests first (Phase 1), then implement in subsequent phases. This means Phase 1 creates all 8 new test files and adds stubs to existing files.

**Suggestion:** Phase 1 creates every missing test as `#[test] #[ignore = "stub"] fn test_name() { todo!() }`. For parameterized tests, create the `#[rstest]` skeleton with `#[values]` attributes but `todo!()` body. This makes the full gap visible via `cargo test -- --ignored`.

**Answer:** Yes — stub-first approach with `#[ignore = "stub"]` + `todo!()`. For parameterized tests, include `#[rstest]` + `#[values]` skeleton.

### Q7: What to do with existing non-upstream tests?

**Context:** Some existing tests don't correspond to any upstream C case:
- `test_files_same_session` — write+read in same mount (no unmount between)
- `test_files_simple_read` — read a pre-created file
- `test_files_seek_tell` — seek/tell exercise
- `test_files_truncate_api` — truncate via `lfs_file_truncate`
- `test_alloc_two_files_ctz` — reproducer for dir corruption bug
- `test_bad_blocks_ctz_repro` — reproducer for CTZSTRUCT corruption bug
- `test_alloc_bad_blocks_minimal` / `_narrow` — minimal bad-block reproducers
- Various `test_powerloss_*` debug helpers
- `test_traverse_*` unit tests in test_superblocks.rs
- `test_dirs_one_mkdir` — single mkdir exercise

Some existing tests map to upstream cases but use different parameter values (e.g. `test_files_append` uses fixed strings instead of PRNG+parameterized sizes).

**Options:**
1. Keep all existing tests as-is, add upstream-faithful tests alongside (may have near-duplicates)
2. Replace existing upstream-mapped tests with faithful versions, keep Rust-specific extras
3. Refactor existing tests to match upstream exactly, keep extras in a separate section

**Suggestion:** Option 2 — replace tests that map to upstream cases with faithful versions. Keep Rust-specific extras (bug reproducers, debug helpers, unit tests) in a clearly marked section at the bottom of each file. This avoids near-duplicates while preserving useful regression tests.

**Answer:** Replace upstream-mapped tests with faithful versions. Keep Rust-specific extras (bug reproducers, debug helpers, unit tests) in a clearly marked section at the bottom of each file.

### Q8: Inline C source in test comments?

**Context:** User wants the C test code from the TOML inlined as comments in the Rust tests, similar to how function implementations include the C source. This makes divergences easy to spot.

**Suggestion:** Include the C code block from the TOML as a doc comment above each test function, like:
```rust
/// Upstream: [cases.test_files_large]
/// defines.SIZE = [32, 8192, 262144, 0, 7, 8193]
/// defines.CHUNKSIZE = [31, 16, 33, 1, 1023]
/// defines.INLINE_MAX = [0, -1, 8]
///
/// ```c
/// lfs_t lfs;
/// lfs_format(&lfs, cfg) => 0;
/// // ... full C code ...
/// ```
```

**Answer:** Include defines + a short summary of the test logic above each test function. Not the full C code, but enough to trace back. Format:
```rust
/// Upstream: [cases.test_files_large]
/// defines.SIZE = [32, 8192, 262144, 0, 7, 8193]
/// defines.CHUNKSIZE = [31, 16, 33, 1, 1023]
/// defines.INLINE_MAX = [0, -1, 8]
///
/// Write SIZE bytes of PRNG data in CHUNKSIZE chunks, unmount, remount, read back and verify.
/// Final read past EOF returns 0.
```

## Notes

- The `lfs_ctz_find` offset bug was only caught because `test_alloc_bad_blocks` (128 blocks) exercises multi-block file reads. All existing file tests use tiny inline files. `test_files_large` would have caught this immediately.
- `docs/rules.md` section 11 covers test porting conventions: use rstest, match C names, comment upstream references.
- C PRNG is xorshift32: `x ^= x << 13; x ^= x >> 17; x ^= x << 5;` with `uint32_t` state, seeded with 1 typically.
