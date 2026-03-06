# Phase 07e: Custom Attributes

## Scope

Port attribute tests. Implement `lfs_getattr_`, `lfs_setattr_`, `lfs_removeattr_`, and `lfs_commitattr` (reference/lfs.c:4107–4196, 4141–4163) if in scope. Per [00-overview](00-overview.md): "Phase 07 brings tests; implementation TBD." Port tests first; implement attr API if needed.

**Translation rules**: [docs/rules.md](../../rules.md). Translate callees first (§0); include C source comments (§3); match logic (§2). Keep test names (§10).

---

## Tests to Port

### test_attrs (from littlefs-rust-old/tests/test_attrs.rs)

| Test | Validates | Notes |
|------|-----------|-------|
| `test_attrs_get_set` | getattr, setattr, removeattr | Core attr API |
| `test_attrs_get_set_root` | Attrs on root dir | |
| `test_attrs_get_set_file` | Attrs on files | |
| `test_attrs_deferred_file` | file_opencfg attrs | |

All require `lfs_getattr`, `lfs_setattr`, `lfs_removeattr`. Per overview: deferred; port tests, implement if needed.

---

## C Reference

- `lfs_getattr_`: reference/lfs.c:4107–4135
- `lfs_commitattr`: reference/lfs.c:4141–4163
- `lfs_setattr_`: reference/lfs.c:4165–4174
- `lfs_removeattr_`: reference/lfs.c:4176–4196

---

## Validation

1. **Build**: `cargo build -p littlefs-rust`
2. **Tests**: All in-scope attr tests pass (or `#[ignore]` with rationale if implementation deferred)
3. **Format**: `cargo fmt`
4. **Warnings**: Zero warnings
