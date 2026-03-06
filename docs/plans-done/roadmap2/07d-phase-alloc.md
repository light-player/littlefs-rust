# Phase 07d: Allocator Tests

## Scope

Port block allocator tests. Validates `lfs_alloc`, `lfs_alloc_lookahead`, `lfs_alloc_scan` (reference/lfs.c:614–791) under parallel, serial, reuse, and exhaustion scenarios.

**Translation rules**: [docs/rules.md](../../rules.md). Translate callees first (§0); include C source comments (§3); match logic (§2). Keep test names (§10).

---

## Tests to Port

### test_alloc (from lp-littlefs-old/tests/test_alloc.rs)

| Test | Validates | Notes |
|------|-----------|-------|
| `test_alloc_parallel` | Multiple dirs/files created in parallel | |
| `test_alloc_serial` | Serial create in one dir | |
| `test_alloc_parallel_reuse` | Create, remove, create in different dir | |
| `test_alloc_serial_reuse` | Create, remove all, create new | |
| `test_alloc_exhaustion` | Fill FS until NOSPC | |
| `test_alloc_split_dir` | Split dir allocation | |

**Deferred**: `test_alloc_exhaustion_wraparound`, `test_alloc_dir_exhaustion`, `test_alloc_bad_blocks`, `test_alloc_chained_dir_exhaustion`, `test_alloc_outdated_lookahead`, `test_alloc_outdated_lookahead_split_dir`.

---

## C Reference

- `lfs_alloc`: reference/lfs.c:666–791
- `lfs_alloc_scan`, `lfs_alloc_lookahead`: reference/lfs.c:627–663
- Block allocation module: block_alloc/alloc.rs

---

## Validation

1. **Build**: `cargo build -p lp-littlefs`
2. **Tests**: All in-scope alloc tests pass
3. **Format**: `cargo fmt`
4. **Warnings**: Zero warnings
