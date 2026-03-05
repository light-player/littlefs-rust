# lp-littlefs Roadmap 2 — Test-Driven Implementation

Phased roadmap for the new lp-littlefs implementation (`lp-littlefs/`). Test-driven: port targeted tests from the reference code, then implement until they pass. Bring over all relevant tests from reference; no test left behind.

**Reference**: [docs/rules.md](../rules.md) — translation rules, test porting, call-graph order.

**Upstream**: littlefs at commit in [docs/reference.md](../reference.md). Tests in `tests/test_*.toml`; reference implementation in lp-littlefs-old and upstream C.

---

## Principles

| Principle | Application |
|-----------|-------------|
| **Test-driven** | Port tests first; implement callees until tests pass ([rules.md §0](../rules.md)) |
| **Reference in file** | Include original C source as comments beside each function ([rules.md §3](../rules.md)) |
| **Call graph order** | Translate callees first; `todo!()` panic guides next step ([rules.md §8](../rules.md)) |
| **All tests** | Each phase targets specific tests; final phases bring over remaining reference tests |
| **Validation** | Every phase has explicit validation steps before sign-off |

---

## Current State (Before Roadmap 2)

- Format, mount, unmount, fs_stat
- Superblock magic check, invalid mount (blank device)
- Traverse infrastructure (filter, push, attrs callback)
- Tests: `test_superblocks_format`, `test_superblocks_mount`, `test_superblocks_invalid_mount`, `test_superblocks_magic`, `test_superblocks_stat`, `test_traverse_filter_gets_superblock_after_push`, `test_traverse_attrs_callback_order`

---

## Phase Summary

| Phase | Focus | Test modules to port |
|-------|--------|----------------------|
| [01-metadata-read](01-phase-metadata-read.md) | stat, dir_open, dir_read, dir_close | test_dirs (root), test_superblocks_stat_tweaked |
| [02-block-caching](02-phase-block-caching.md) | Read/prog caches (internal) | No new tests; regression only |
| [03-directory-mutations](03-phase-directory-mutations.md) | mkdir, remove, rename | test_dirs (full), test_paths (dirs) |
| [04-file-read](04-phase-file-read.md) | Inline + CTZ read | test_files (read), test_entries (read paths) |
| [05-file-write](05-phase-file-write.md) | Create, write, sync, truncate, append | test_files (write), test_entries (grow) |
| [06-power-loss](06-phase-power-loss.md) | deorphan, mkconsistent, gc | test_orphans, test_powerloss |
| [07-remaining-tests](07-phase-remaining-tests.md) | Paths, move, entries, attrs, relocations, alloc | test_paths, test_move, test_entries, test_attrs, test_relocations, test_alloc |
| [08-robustness](08-phase-robustness.md) | Exhaustion, evil, bad blocks | test_exhaustion, test_evil, test_badblocks, test_interspersed |

Phase 07 has subphases: [07a](07a-phase-paths-move.md), [07b](07b-phase-entries.md), [07c](07c-phase-relocations.md), [07d](07d-phase-alloc.md), [07e](07e-phase-attrs.md).

---

## Deferred (Out of Scope for Roadmap 2)

- **block_count = 0** — Read block_count from superblock; LightPlayer uses fixed partition.
- **Custom attributes** (getattr, setattr, removeattr) — Phase 07 brings tests; implementation TBD.
- **fs_grow / fs_shrink** — Fixed partition; test_shrink remains ignored.
- **C↔Rust format compat** — test_compat; requires C binary or fixtures.
- **dir_seek / dir_tell / dir_rewind** — test_dirs_kitty_seek, test_dirs_toot_seek.
- **Power-loss runner** — Tests requiring automated power-loss injection; many marked `#[ignore]`.
- **Block-level corruption simulation** — test_evil_*, test_orphans_normal; needs uncached BD.

---

## Per-Phase Workflow

1. **Select tests** — Minimal set from phase doc; port from lp-littlefs-old or upstream TOML.
2. **Port** — Same names, upstream reference in header ([rules.md §10](../rules.md)).
3. **Run** — Test fails on `todo!()` or wrong behavior.
4. **Implement** — Callees first; C source comments; match logic ([rules.md §0, §2, §3](../rules.md)).
5. **Validate** — Phase-specific checks + `cargo fmt`, `cargo test`, no warnings.
6. **Expand** — Broaden parameter ranges once stable ([rules.md §10](../rules.md)).
