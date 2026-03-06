# Phase 08: Robustness (exhaustion, evil, bad blocks, interspersed)

## Scope

Bring over robustness and stress tests. Many require block-level corruption simulation, bad-block BD, or power-loss runner. Port all relevant tests; implement infrastructure where feasible; document deferred items.

**Translation rules**: [docs/rules.md](../../rules.md). Translate callees first (§0); include C source comments (§3); match logic (§2). Never relax a test to accommodate a bug (§6).

---

## Test Modules to Port (All Relevant)

### test_exhaustion

| Test | Validates | Notes |
|------|-----------|-------|
| `test_exhaustion_normal` | Normal exhaustion | NOSPC when full |
| `test_exhaustion_superblocks` | Superblock exhaustion | May differ |
| `test_exhaustion_wear_leveling` | block_cycles BD | Requires wear sim |
| `test_exhaustion_wear_leveling_superblocks` | Wear + superblocks | |
| `test_exhaustion_wear_distribution` | Wear distribution | |

### test_evil

| Test | Validates | Notes |
|------|-----------|-------|
| `test_evil_invalid_tail_pointer` | Corrupt tail | Block corruption sim |
| `test_evil_invalid_dir_pointer` | Corrupt dir | |
| `test_evil_invalid_move_pointer` | Corrupt move | |
| `test_evil_powerloss` | Power-loss | Power-loss runner |
| `test_evil_mdir_loop` | mdir loop | Corruption sim |
| `test_evil_multiple_revs` | Multiple revs | Corruption sim |
| `test_evil_split_both_dirs` | Split both | Corruption sim |
| `test_evil_double_compact` | Double compact | Corruption sim |

All require block-level corruption simulation (uncached BD + direct block prog) or power-loss runner.

### test_badblocks

| Test | Validates | Notes |
|------|-----------|-------|
| `test_badblocks_single` | Single bad block | Bad-block BD sim |
| `test_badblocks_double` | Double bad block | |
| `test_badblocks_boundary` | Boundary bad block | |
| `test_badblocks_corrupt` | Corrupt block | |

Requires block device that can simulate bad/prog-failing blocks.

### test_interspersed

| Test | Validates | Notes |
|------|-----------|-------|
| `test_interspersed_files` | Interleaved file ops | |
| `test_interspersed_remove_files` | Interleaved remove | |
| `test_interspersed_remove_inconveniently` | Remove order | |
| `test_interspersed_reentrant_files` | Power-loss | Power-loss runner |

---

## Infrastructure Requirements

- **Block corruption simulation**: BD that can program arbitrary blocks (bypass cache) for evil tests
- **Bad-block BD**: BD that returns error on specific block reads/progs
- **Power-loss runner**: Inject simulated power-loss at configurable points; many tests need this

---

## Implementation Order

1. **Exhaustion (normal)** — `test_exhaustion_normal`; no special BD
2. **Interspersed (non-reentrant)** — `test_interspersed_files`, `test_interspersed_remove_*`
3. **Bad-block BD** — If feasible; then `test_badblocks_*`
4. **Corruption sim** — If feasible; then `test_evil_*` (non-power-loss)
5. **Power-loss runner** — Deferred; enables many remaining tests

---

## Validation

Before considering this phase complete:

1. **Build**: `cargo build -p lp-littlefs`
2. **Tests**: `cargo test -p lp-littlefs` — all implemented robustness tests pass
3. **Format**: `cargo fmt`
4. **Warnings**: Zero warnings
5. **Deferred list**: Document tests requiring infrastructure not yet implemented
