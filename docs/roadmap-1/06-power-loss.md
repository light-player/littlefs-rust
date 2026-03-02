# Phase 06: Power-loss resilience (deorphan, mkconsistent, gc)

## Scope of phase

Implement global state (MOVESTATE), deorphan on mount, force-consistency, and fs_gc. Required for safe operation when device can power-cycle mid-write. Includes FCRC in commits for partial-program detection.

Refer to the C implementation /Users/yona/dev/photomancer/oss/littlefs/lfs.c for the implementation details,
match the C implementation as closely as possible while keeping the Rust code clean and idiomatic.

## API targets

| API                                      | Upstream                          | Description                              |
| ---------------------------------------- | --------------------------------- | ---------------------------------------- |
| `fs_mkconsistent() -> Result<(), Error>` | `lfs_fs_mkconsistent` (lfs.h:529) | Deorphan, complete moves, persist gstate |
| `fs_gc() -> Result<(), Error>`           | `lfs_fs_gc` (lfs.h:546)           | Compact metadata, populate allocator     |
| `fs_traverse(cb) -> Result<(), Error>`   | `lfs_fs_traverse` (lfs.h:519)     | Callback per used block                  |
| `fs_size() -> Result<i64, Error>`        | `lfs_fs_size` (lfs.h:510)         | Allocated block count                    |

**Internal**: Global state XOR deltas (gstate, gdisk, gdelta); MOVESTATE tag; deorphan; move-state completion. Upstream `lfs_fs_deorphan`, `lfs_fs_forceconsistency`, `lfs_fs_preporphans` (lfs.c).

## Upstream tests to port

| Source                      | Case                                   | Validates                                                     |
| --------------------------- | -------------------------------------- | ------------------------------------------------------------- |
| `tests/test_orphans.toml`   | `test_orphans_normal`                  | Corrupt commit → orphan; mount deorphans; stat orphan → NOENT |
| `tests/test_orphans.toml`   | `test_orphans_no_orphans`              | preporphans, forceconsistency clears                          |
| `tests/test_orphans.toml`   | `test_orphans_mkconsistent_no_orphans` | mkconsistent persists gstate; remount no orphans              |
| `tests/test_orphans.toml`   | `test_orphans_mkconsistent_one_orphan` | mkconsistent with real orphan                                 |
| `tests/test_powerloss.toml` | `test_powerloss_only_rev`              | Partial write (rev only); mount still works                   |
| `tests/test_files.toml`     | `test_files_reentrant_write`           | Power-loss during write; recover on remount                   |
| `tests/test_files.toml`     | `test_files_many_power_loss`           | Many files with simulated power-loss                          |

**Minimal set**: `test_orphans_mkconsistent_no_orphans`, `test_orphans_mkconsistent_one_orphan`, `test_powerloss_only_rev`. `test_orphans_normal` requires block-level corruption simulation — may simplify or defer.

## SPEC references

- **MOVESTATE**: SPEC.md "0x7ff LFS_TYPE_MOVESTATE" — sync bit, move type, move id, metadata pair
- **Global state**: SPEC.md "0x7xx LFS_TYPE_GSTATE" — XOR-sum of deltas across commits
- **FCRC**: SPEC.md "Metadata blocks" — forward CRC for next-commit validation
- **Deorphan**: DESIGN.md; lfs.c `lfs_fs_deorphan` — threaded linked-list repair
- **Tail / soft-tail / hard-tail**: SPEC.md — linked-list structure

## Code organization

- Global state (gstate) in fs-level struct
- Deorphan in mount path and mkconsistent
- GC: compact metadata, allocator refill

## Validation

Run `just fci` to format, fix, and validate the code before committing.
