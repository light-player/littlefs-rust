# Phase 8: Parameterize Existing Tests

## Goal

Exact replication of upstream TOML parameter sets in existing Rust tests. Every upstream define combination must appear as a Rust test parameterization. If new parameter combinations expose bugs, mark them `#[ignore = "bug: <description>"]` and move on — bug fixes come later, after implementation.

## Scope

Upgrade existing Rust tests that map to upstream cases but use a single fixed configuration (or a subset) instead of the full upstream parameter set. Convert to `#[rstest]` with `#[values]` / `#[case]` matching the upstream defines.

## Sub-plans

Split by file, ordered by amount of work:

- **[08a — test_alloc.rs](08a-phase-parameterize-alloc.md)** — Add GC, COMPACT_THRESH, INFER_BC, CYCLES to 7 cases (12 combos each for parallel/serial). 5 cases already match (fixed defines, no parameterization).
- **[08b — test_dirs.rs](08b-phase-parameterize-dirs.md)** — Expand many_creation/removal/rename from N=1 to full upstream ranges. Add missing N values to file_creation/removal/rename. ~8 cases need work.
- **[08c — test_superblocks.rs](08c-phase-parameterize-superblocks.md)** — Expand test_superblocks_grow to 6 combinations. Add metadata_max skeleton. Most cases already match via inner loops.
- **[08d — test_relocations.rs](08d-phase-parameterize-relocations.md)** — Add BLOCK_CYCLES to 2 cases, FILES/DEPTH/CYCLES define sets to 4 cases. All 6 cases need work.

## Dropped from original plan

- **test_attrs.rs** — Upstream TOML has zero per-case defines. Nothing to parameterize.
- **test_entries.rs** — Upstream TOML has only a top-level `CACHE_SIZE = 512` guard, no per-case defines. Nothing to parameterize.

## Strategy

For each test:
1. Find the matching `[cases.NAME]` in the upstream TOML
2. Extract the `defines` block
3. Add `#[rstest]` and parameter attributes matching the upstream values
4. Add `if` guard as early return where applicable
5. Update the upstream comment header to include the full defines
6. Run the test — if it fails, `#[ignore = "bug: <description>"]` and move on
