# Phase 06: Power-Loss Resilience (deorphan, mkconsistent, gc)

## Scope

Implement global state (MOVESTATE), deorphan on mount, force-consistency, and fs_gc. Required for safe operation when device can power-cycle mid-write. Includes FCRC in commits for partial-program detection.

**Translation rules**: [docs/rules.md](../../rules.md). Translate callees first (§0); include C source comments (§3); match logic (§2). Preserve control flow (§9).

---

## API Targets

| API | Upstream | Description |
|-----|----------|-------------|
| `lfs_fs_mkconsistent() -> i32` | lfs.h | Deorphan, complete moves, persist gstate |
| `lfs_fs_gc() -> i32` | lfs.h | Compact metadata, populate allocator |
| `lfs_fs_traverse(cb) -> i32` | lfs.h | Callback per used block |
| `lfs_fs_size() -> i64` | lfs.h | Allocated block count |

Internal: Global state XOR deltas (gstate, gdisk, gdelta); MOVESTATE tag; deorphan; move-state completion. Upstream `lfs_fs_deorphan`, `lfs_fs_forceconsistency`, `lfs_fs_preporphans`.

---

## Tests to Port (All Relevant)

From lp-littlefs-old/test_orphans.rs, test_powerloss.rs. Same names per [rules.md §10](../../rules.md).

| Source | Test | Validates |
|--------|------|-----------|
| test_orphans.toml | `test_orphans_mkconsistent_no_orphans` | mkconsistent with no orphans |
| test_orphans.toml | `test_orphans_no_orphans` | preporphans, forceconsistency clears |
| test_orphans.toml | `test_orphans_nonreentrant` | Non-reentrant orphan handling |
| test_orphans.toml | `test_orphans_mkconsistent_one_orphan` | mkconsistent with real orphan (requires orphan creation) |
| test_orphans.toml | `test_orphans_one_orphan` | Low-level orphan (if exposed) |
| test_powerloss.toml | `test_powerloss_only_rev` | Partial write (rev only); mount still works |
| test_powerloss.toml | `test_powerloss_partial_prog` | Partial program at byte offset |

**Deferred** (block corruption / power-loss runner): `test_orphans_normal`, `test_orphans_reentrant` — require block-level corruption simulation or power-loss injection.

---

## SPEC References

- MOVESTATE: 0x7ff LFS_TYPE_MOVESTATE
- Global state: 0x7xx LFS_TYPE_GSTATE
- FCRC: Metadata blocks — forward CRC for next-commit validation
- Deorphan: DESIGN.md; lfs_fs_deorphan — threaded linked-list repair
- Tail / soft-tail / hard-tail: linked-list structure

---

## Validation

Before considering this phase complete:

1. **Build**: `cargo build -p lp-littlefs`
2. **Tests**: `cargo test -p lp-littlefs`
3. **Format**: `cargo fmt`
4. **Warnings**: Zero warnings
5. **Phase tests**: `test_orphans_mkconsistent_no_orphans`, `test_orphans_no_orphans`, `test_powerloss_only_rev` pass
