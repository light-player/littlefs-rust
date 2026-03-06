# Phase 07: Remaining Tests (paths, move, entries, attrs, relocations, alloc)

## Scope

Bring over all remaining relevant tests from the reference code that are not covered by Phases 01–06. Implement whatever API or behavior each test requires.

Phase 07 is split into subphases to isolate dependencies and enable incremental progress. Each subphase has its own validation.

**Translation rules**: [docs/rules.md](../../rules.md). Translate callees first (§0); include C source comments (§3); match logic (§2). Keep test names (§10).

---

## Subphases

| Subphase | Focus | Tests |
|----------|-------|-------|
| [07a-paths-move](07a-phase-paths-move.md) | Path resolution, absolute paths, rename (same-dir + cross-dir) | test_paths, test_move |
| [07b-phase-entries](07b-phase-entries.md) | Metadata spill, push_spill, drop, shrink | test_entries |
| [07c-phase-relocations](07c-phase-relocations.md) | dir_compact, dir_split, orphaningcommit | test_relocations |
| [07d-phase-alloc](07d-phase-alloc.md) | Block allocator: parallel, serial, reuse, exhaustion | test_alloc |
| [07e-phase-attrs](07e-phase-attrs.md) | Custom attributes (getattr, setattr, removeattr) | test_attrs |

---

## Reference

- **Upstream tests**: `reference/tests/test_*.toml` (or lp-littlefs-old port)
- **C implementation**: `reference/lfs.c` — lfs_rename_ (3961–4138), lfs_getattr_/setattr_/removeattr_ (4107–4196), lfs_commitattr (4141–4163)
- **Function inventory**: [docs/function-inventory.md](../function-inventory.md)

---

## Phase 07 Validation (All Subphases Complete)

1. **Build**: `cargo build -p lp-littlefs`
2. **Tests**: `cargo test -p lp-littlefs` — all ported tests in scope pass
3. **Format**: `cargo fmt`
4. **Warnings**: Zero warnings
5. **Documentation**: Deferred tests listed with rationale in each subphase
