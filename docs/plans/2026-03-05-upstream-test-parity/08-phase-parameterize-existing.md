# Phase 8: Parameterize Existing Tests

## Scope

Upgrade existing Rust tests that map to upstream cases but use a single fixed configuration instead of the full upstream parameter set. Convert to `#[rstest]` with `#[values]` / `#[case]` / inner loops matching the upstream defines.

## Code Organization Reminders

- Place upstream cases first, extras at the bottom
- Include upstream defines + summary comment on every test
- Keep related functionality grouped together
- Changes should be incremental — one file at a time, verify between

## Strategy

For each test:
1. Find the matching `[cases.NAME]` in the upstream TOML
2. Extract the `defines` block
3. Add `#[rstest]` and parameter attributes matching the upstream values
4. Add `if` guard as early return where applicable
5. Update the upstream comment header to include the full defines
6. Run the test to verify all parameter combinations pass

## Files to Update

### test_alloc.rs

Current state: all upstream cases present but some with reduced parameterization.

Check each case against `reference/tests/test_alloc.toml`:
- `test_alloc_parallel_allocation` — verify defines match
- `test_alloc_serial_allocation` — verify defines match
- `test_alloc_exhaustion` — verify defines match
- `test_alloc_parallel_exhaustion` — verify defines match
- `test_alloc_bad_blocks` — verify defines match
- `test_alloc_reentrant` — verify defines match
- `test_alloc_chained_dir_exhaustion` — verify defines match
- `test_alloc_split_dir_exhaustion` — verify defines match
- `test_alloc_outdated_lookahead_exhaustion` — verify defines match
- `test_alloc_outdated_lookahead_reentrant` — verify defines match

### test_attrs.rs

Check against `reference/tests/test_attrs.toml`. Likely needs INLINE_MAX parameterization.

### test_entries.rs

Check against `reference/tests/test_entries.toml`. Add N/SIZE parameterization where missing.

### test_dirs.rs

Cases already present but may need COUNT/N parameterization upgrade for: `test_dirs_many_creation`, `test_dirs_many_removal`, `test_dirs_many_rename`.

### test_relocations.rs

Check against `reference/tests/test_relocations.toml`. May need ERASE_CYCLES, BLOCK_CYCLES, N parameterization.

### test_superblocks.rs

Existing cases may need BLOCK_SIZE, BLOCK_COUNT, ERASE_COUNT parameterization.

## Process

For each file:

```
1. Read upstream TOML defines for each case
2. Compare with current Rust test parameters
3. Add missing #[rstest] + #[values] attributes
4. Update upstream comment header
5. cargo test -p lp-littlefs <test_file_name>
6. cargo fmt && cargo clippy
```

## Validate

```
cargo test -p lp-littlefs 2>&1
# Full suite passes with expanded parameterization

cargo fmt -p lp-littlefs
cargo clippy -p lp-littlefs
```

Note: this phase may reveal new bugs that were masked by limited parameterization. Any test failures should be investigated and fixed before proceeding (or marked `#[ignore = "bug: <description>"]` with a tracking doc).
