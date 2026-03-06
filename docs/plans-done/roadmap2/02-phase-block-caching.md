# Phase 02: Block Device Caching

## Scope

Add read and program caches to the block device layer. All format/mount/file/dir operations use cached reads and program-through-cache where applicable. Matches upstream `lfs_bd_read`, `lfs_bd_prog`, `lfs_bd_cmp`, `lfs_bd_crc` behavior.

**Translation rules**: [docs/rules.md](../../rules.md). Translate callees first (§0); include C source comments (§3); match logic (§2).

---

## API Targets

No new public API. Internal:

- Read cache: block, offset, size; bypass for large aligned reads (hint-based)
- Program cache: block, offset, size; flush on sync/eviction
- Config: `cache_size`, `read_buffer`, `prog_buffer` (optional static buffers)

Upstream: lfs.c `lfs_bd_*`, `lfs_cache_t`, `lfs_config` cache fields.

---

## Tests to Port

No dedicated cache test in reference. Existing tests must not regress:

| Test | Purpose |
|------|---------|
| `test_superblocks_format` | Format path uses BD |
| `test_superblocks_mount` | Mount path uses BD |
| `test_superblocks_magic` | Raw read for magic check |
| `test_superblocks_stat` | fs_stat after mount |
| `test_traverse_filter_gets_superblock_after_push` | Traverse reads blocks |
| `test_traverse_attrs_callback_order` | Traverse callback order |

All must pass unchanged. Phase validates by non-regression.

---

## SPEC References

- Block reads/programs: DESIGN.md block device semantics; SPEC.md read/prog alignment.

---

## Validation

Before considering this phase complete:

1. **Build**: `cargo build -p littlefs-rust`
2. **Tests**: `cargo test -p littlefs-rust` — all existing tests pass
3. **Format**: `cargo fmt`
4. **Warnings**: Zero warnings
5. **No new public API**: Caching is internal only
