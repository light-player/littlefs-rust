# lp-littlefs Hand-Translation Roadmap

Hand-translate the LittleFS C implementation to Rust with minimal logic change. Keep architecture close to the C version; use `unsafe` where needed. Once tests pass, optionally wrap in safe Rust or iteratively refactor.

**Reference:** `reference/` — lfs.c, lfs.h, lfs_util.c, lfs_util.h

## Principles

| Principle | Application |
|-----------|-------------|
| Minimal logic change | Preserve C control flow, pointer patterns; use `unsafe` where needed |
| Reference in-file | Original C source as comments beside each stubbed function |
| `todo!()` as driver | Tests panic at first unimplemented function; guides next implementation |
| Incremental tests | Add tests only for slices being implemented |
| Format alignment | Validate layout against C early |
| Manual extraction | Lightweight or no parsing scripts; rely on docs and grep |
| Use existing docs | `lp-littlefs-old/docs/alignment/metadata-vs-lfs-c.md`, commit mappings |

## Phases

| Phase | Focus |
|-------|-------|
| [01-phase-setup](01-phase-setup.md) | New crate, no_std, disable old build/tests |
| [02-phase-structures](02-phase-structures.md) | Translate C structs, enums, macros; no_std shims |
| [03-phase-function-inventory](03-phase-function-inventory.md) | Extract functions; group into modules |
| [04-phase-stubs](04-phase-stubs.md) | Stubbed functions with `todo!()` and C comments |
| [05-phase-incremental-impl](05-phase-incremental-impl.md) | Vertical slice; implement until test passes |
| [06-phase-test-expansion](06-phase-test-expansion.md) | Port remaining tests; all pass |
| [07-phase-safe-api](07-phase-safe-api.md) | Safe wrapper; optional safe refactor (later) |
