# Phase 07c: Relocations

## Scope

Port relocation tests that validate `dir_compact`, `dir_split`, and `orphaningcommit` under specific metadata patterns. Exercises dangling split dirs, outdated head, and non-reentrant mkdir/remove/rename cycles.

**Translation rules**: [docs/rules.md](../rules.md). Translate callees first (§0); include C source comments (§3); match logic (§2). Keep test names (§10).

---

## Tests to Port

### test_relocations (from lp-littlefs-old/tests/test_relocations.rs)

| Test | Validates | Notes |
|------|-----------|-------|
| `test_relocations_dangling_split_dir` | Fill FS, many files in child dir | Triggers split when metadata overflows |
| `test_relocations_outdated_head` | Split dir handling | Multiple dirs, nested sub with many files |
| `test_relocations_nonreentrant` | mkdir/remove cycles | No power-loss |
| `test_relocations_nonreentrant_renames` | Chained renames (x→z, y→x, z→y) | Same-slot name changes |

**Deferred**: `test_relocations_reentrant`, `test_relocations_reentrant_renames` (power-loss runner).

---

## C Reference

- `lfs_dir_compact`, `lfs_dir_splittingcompact`: reference/lfs.c:1952–2232
- `lfs_dir_relocatingcommit`, `lfs_dir_orphaningcommit`: reference/lfs.c:2234–2599
- `lfs_dir_split`: reference/lfs.c:1880–1913

---

## Validation

1. **Build**: `cargo build -p lp-littlefs`
2. **Tests**: All in-scope relocation tests pass
3. **Format**: `cargo fmt`
4. **Warnings**: Zero warnings
