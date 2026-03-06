# Phase 8d: Parameterize test_relocations.rs

## Goal

Exact replication of upstream `reference/tests/test_relocations.toml` parameter sets. Every upstream define combination must appear as a Rust test parameterization. If new combinations fail, mark them `#[ignore = "bug: <description>"]` and move on.

## Reference

- Upstream TOML: `reference/tests/test_relocations.toml`
- Rust file: `littlefs-rust/tests/test_relocations.rs`

## Current State

All 6 upstream cases exist as Rust functions, all using plain `#[test]` with single fixed configurations. None use `#[rstest]`. None iterate over multiple parameter sets.

## Cases to Parameterize

### test_relocations_dangling_split_dir

Upstream:
```
ITERATIONS = 20
COUNT = 10
BLOCK_CYCLES = [8, 1]
```
2 combinations over BLOCK_CYCLES.

Current Rust: uses `default_config(128)` with default `block_cycles: -1`. Does not set BLOCK_CYCLES, ITERATIONS, or COUNT. Test body uses hardcoded values (8 files, no iteration loop matching upstream).

Action: Add `#[rstest]` with `#[values(8, 1)]` for `block_cycles`. Set `env.config.block_cycles = block_cycles` after init. Wire ITERATIONS=20 and COUNT=10 into the loop structure to match upstream C body.

### test_relocations_outdated_head

Upstream:
```
ITERATIONS = 20
COUNT = 10
BLOCK_CYCLES = [8, 1]
```
2 combinations. Same as dangling_split_dir.

Current Rust: same issue — hardcoded values, no block_cycles parameter.

Action: Same as dangling_split_dir.

### test_relocations_reentrant

Upstream:
```
reentrant = true
if = '!(DEPTH == 3 && CACHE_SIZE != 64) && 2*FILES < BLOCK_COUNT'
defines = [
    {FILES=6,  DEPTH=1, CYCLES=20, BLOCK_CYCLES=1},
    {FILES=26, DEPTH=1, CYCLES=20, BLOCK_CYCLES=1},
    {FILES=3,  DEPTH=3, CYCLES=20, BLOCK_CYCLES=1},
]
```
3 define sets.

Current Rust: uses `powerloss_config(128)` with fixed configuration. Does not iterate over the 3 define sets. Test body uses hardcoded FILES/DEPTH/CYCLES.

Action: Add `#[rstest]` with `#[case]` for each of the 3 define sets. Add `if` guard for DEPTH==3 && CACHE_SIZE!=64 as early return. Set `block_cycles = 1`. Use block_count large enough that `2*FILES < BLOCK_COUNT`.

### test_relocations_reentrant_renames

Upstream:
```
reentrant = true
if = '!(DEPTH == 3 && CACHE_SIZE != 64) && 2*FILES < BLOCK_COUNT'
defines = [
    {FILES=6,  DEPTH=1, CYCLES=20, BLOCK_CYCLES=1},
    {FILES=26, DEPTH=1, CYCLES=20, BLOCK_CYCLES=1},
    {FILES=3,  DEPTH=3, CYCLES=20, BLOCK_CYCLES=1},
]
```
3 define sets. Same as reentrant.

Current Rust: same issue.

Action: Same as reentrant.

### test_relocations_nonreentrant

Upstream:
```
if = '!(DEPTH == 3 && CACHE_SIZE != 64) && 2*FILES < BLOCK_COUNT'
defines = [
    {FILES=6,  DEPTH=1, CYCLES=2000, BLOCK_CYCLES=1},
    {FILES=26, DEPTH=1, CYCLES=2000, BLOCK_CYCLES=1},
    {FILES=3,  DEPTH=3, CYCLES=2000, BLOCK_CYCLES=1},
]
```
3 define sets. Same parameters as reentrant but CYCLES=2000 and no power-loss.

Current Rust: uses `default_config(128)` with hardcoded values. Does not iterate.

Action: Add `#[rstest]` with `#[case]` for the 3 define sets. These are long-running (CYCLES=2000), so expect slow execution.

### test_relocations_nonreentrant_renames

Upstream:
```
if = '!(DEPTH == 3 && CACHE_SIZE != 64) && 2*FILES < BLOCK_COUNT'
defines = [
    {FILES=6,  DEPTH=1, CYCLES=2000, BLOCK_CYCLES=1},
    {FILES=26, DEPTH=1, CYCLES=2000, BLOCK_CYCLES=1},
    {FILES=3,  DEPTH=3, CYCLES=2000, BLOCK_CYCLES=1},
]
```
3 define sets.

Current Rust: same issue.

Action: Same as nonreentrant.

## Summary of Actual Work

| Case | Current params | Upstream params | Action |
|------|---------------|----------------|--------|
| dangling_split_dir | fixed | BLOCK_CYCLES=[8,1] | add 2 combos |
| outdated_head | fixed | BLOCK_CYCLES=[8,1] | add 2 combos |
| reentrant | fixed | 3 define sets (FILES/DEPTH/CYCLES/BLOCK_CYCLES) | add 3 cases |
| reentrant_renames | fixed | 3 define sets | add 3 cases |
| nonreentrant | fixed | 3 define sets (CYCLES=2000) | add 3 cases |
| nonreentrant_renames | fixed | 3 define sets | add 3 cases |

## Implementation Notes

- All 6 cases need work. This is a small file but every test body needs the parameterization wired in.
- For dangling_split_dir and outdated_head, the upstream body uses ITERATIONS=20 and COUNT=10 as loop bounds. The current Rust tests may use different hardcoded bounds — align them.
- For reentrant/nonreentrant cases, the upstream body uses a PRNG (`TEST_PRNG`) to generate random paths from `alpha[0..FILES]` at `DEPTH` levels. The current Rust tests likely use the same pattern with hardcoded values — make FILES, DEPTH, CYCLES into parameters.
- The `if = '!(DEPTH == 3 && CACHE_SIZE != 64)'` guard: add as `if depth == 3 && cache_size != 64 { return; }` early return. In practice with default cache_size=512, the DEPTH=3 case will be skipped unless cache_size is adjusted.
- `block_cycles = 1` forces relocations on nearly every write — this is the point of these tests.
- CYCLES=2000 for nonreentrant tests means they're slow. Consider adding `#[ignore = "slow"]` or accepting the runtime.

## Process

```
1. Add rstest to imports
2. For dangling_split_dir and outdated_head:
   a. Replace #[test] with #[rstest]
   b. Add #[values(8, 1)] for block_cycles parameter
   c. Set env.config.block_cycles in body
   d. Wire ITERATIONS=20, COUNT=10 loop bounds
3. For reentrant/reentrant_renames/nonreentrant/nonreentrant_renames:
   a. Replace #[test] with #[rstest]
   b. Add #[case(6, 1, 20)] #[case(26, 1, 20)] #[case(3, 3, 20)] (or CYCLES=2000 for non-reentrant)
   c. Wire files, depth, cycles, block_cycles=1 into body
   d. Add DEPTH==3 && CACHE_SIZE!=64 guard
4. Update upstream comment headers
5. cargo test -p littlefs-rust --test test_relocations
6. Mark any new failures: #[ignore = "bug: <description>"]
7. cargo fmt && cargo clippy
```

## Validate

```
cargo test -p littlefs-rust --test test_relocations 2>&1
cargo fmt -p littlefs-rust
cargo clippy -p littlefs-rust
```
