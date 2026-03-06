# Upstream Test Parity — Design

## Scope

Bring the littlefs-rust Rust test suite to full parity with the upstream C littlefs test suite (`reference/tests/*.toml`). Every upstream case is represented, with full parameterization matching the C runner's Cartesian-product-of-defines model.

## File Structure

```
littlefs-rust/tests/
├── common/
│   ├── mod.rs                    # UPDATE: test_prng, write/verify helpers,
│   │                             #   write_block_raw, config_with_inline_max,
│   │                             #   WearLevelingBd, BadBlockBehavior enum
│   ├── dump.rs                   # UNCHANGED
│   └── powerloss.rs              # UNCHANGED
├── test_alloc.rs                 # UPDATE: parameterize existing cases
├── test_attrs.rs                 # UPDATE: parameterize if needed
├── test_badblocks.rs             # NEW: 4 cases
├── test_compat.rs                # NEW: 17 cases (all stubbed/deferred)
├── test_dirs.rs                  # UPDATE: add ~11 missing, parameterize
├── test_entries.rs               # UPDATE: parameterize if needed
├── test_evil.rs                  # NEW: 8 cases
├── test_exhaustion.rs            # NEW: 5 cases
├── test_files.rs                 # UPDATE: replace weak tests, add 6 missing
├── test_interspersed.rs          # NEW: 4 cases
├── test_move.rs                  # UPDATE: add 2, implement 6 stubs
├── test_orphans.rs               # UPDATE: add 3 missing
├── test_paths.rs                 # UPDATE: add ~7 missing
├── test_powerloss.rs             # UPDATE: add 1 missing
├── test_relocations.rs           # UPDATE: parameterize
├── test_seek.rs                  # NEW: 10 cases
├── test_shrink.rs                # NEW: 2 cases
├── test_superblocks.rs           # UPDATE: add ~14 missing
└── test_truncate.rs              # NEW: 7 cases
```

## Parameterization Conventions

```
Upstream TOML defines          Rust rstest mapping
─────────────────────          ────────────────────
value lists [a, b, c]    →    #[values(a, b, c)]
range(3, 100, 3)          →    inner for loop (3..100).step_by(3)
expressions 'BLOCK_SIZE-8' →  computed in test body
if = 'X < Y'              →    early return at top of test
config defines (INLINE_MAX) →  applied to env.config before mount
```

## Test File Layout

Each test file follows this structure:

```
//! Upstream: tests/test_X.toml

mod common;
use ...;

// ── Upstream Cases ──────────────────────────
// Faithful translations, one function per case.

/// Upstream: [cases.test_X_case1]
/// defines.A = [...], defines.B = [...]
///
/// Summary of what the test does.
#[rstest]
fn test_X_case1(
    #[values(...)] a: u32,
    #[values(...)] b: u32,
) {
    if !filter(a, b) { return; }
    let computed = expr(a, b);
    let mut env = config_with_...();
    // ... test body ...
}

// ── Rust-specific extras ────────────────────
// Bug reproducers, debug helpers, unit tests.
// Not in upstream; kept for regression coverage.

fn test_X_extra_repro() { ... }
```

## Test Comment Format

Each upstream-mapped test includes:
- `/// Upstream: [cases.test_name]`
- All `defines.*` lines from the TOML
- A short summary of the test logic (1-3 sentences)

## Existing Test Handling

- Tests that map to upstream cases are **replaced** with faithful versions.
- Rust-specific extras (bug reproducers, debug helpers, unit tests) are **kept** in a separate section at the bottom of each file.

## Infrastructure Additions (common/mod.rs)

| Component | Purpose |
|-----------|---------|
| `test_prng(state: &mut u32) -> u32` | xorshift32 PRNG, matches C `TEST_PRNG` |
| `write_prng_file(lfs, file, size, chunk, seed)` | Write PRNG data in chunks |
| `verify_prng_file(lfs, file, size, chunk, seed)` | Read + verify PRNG data |
| `advance_prng(state, n)` | Skip n bytes of PRNG sequence |
| `config_with_inline_max(blocks, inline_max)` | Config with INLINE_MAX support |
| `write_block_raw(config, block, off, data)` | Raw block write (for test_evil) |
| `WearLevelingBd` | Per-block erase-cycle tracking BD wrapper |
| `BadBlockBehavior` enum | PROGERROR, ERASEERROR, READERROR, PROGNOOP, ERASENOOP |

## Phases

| Phase | File | Description |
|-------|------|-------------|
| [01](01-phase-stub-all-tests.md) | All | Stub all missing tests with `#[ignore = "stub"]` + `todo!()` |
| [02](02-phase-test-infrastructure.md) | common/mod.rs | test_prng, write/verify helpers, WearLevelingBd, BadBlockBehavior |
| [03](03-phase-test-files.md) | test_files.rs | Large file I/O — highest impact, would have caught CTZ bug |
| [04](04-phase-test-seek-truncate.md) | test_seek.rs, test_truncate.rs | File position and size mutations (10 + 7 cases) |
| [05](05-phase-test-interspersed-badblocks.md) | test_interspersed.rs, test_badblocks.rs | Multi-file I/O + worn-block handling (4 + 4 cases) |
| [06](06-phase-test-evil-exhaustion-shrink.md) | test_evil.rs, test_exhaustion.rs, test_shrink.rs | Corruption detection, wear leveling, resize (8 + 5 + 2 cases) |
| [07](07-phase-existing-missing-cases.md) | Existing files | ~39 missing cases across dirs, superblocks, move, orphans, paths, powerloss |
| [08](08-phase-parameterize-existing.md) | Existing files | Upgrade single-value tests to full upstream parameter sets |
| [09](09-phase-compat-cleanup.md) | test_compat.rs, all | 17 compat cases (deferred if infra missing) + final cleanup |
