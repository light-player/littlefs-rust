# lp-littlefs Roadmap — LightPlayer-targeted

High-level implementation phases for lp-littlefs. Each phase spec (01-*.md) defines API targets, upstream test references, and SPEC citations for future implementation plans.

**Target**: LightPlayer on ESP32; flash-backed LpFs; alloc available.

**Upstream reference**: `/Users/yona/dev/photomancer/oss/littlefs` (lfs.c, lfs.h, SPEC.md, DESIGN.md, tests/*.toml)

**Power-loss**: Must have. Device can power-cycle during upload. Phases must include mkconsistent, gc, deorphan, FCRC, move-state recovery.

---

## Deferred (out of initial scope)

- **block_count = 0** — Upstream supports reading block_count from superblock when config has block_count=0. Deferred: LightPlayer uses fixed partition size (1MB); known at mount.
- **Custom attributes** (getattr, setattr, removeattr) — LpFs does not use them. Deferred; add only if cross-format tooling needs them.
- **fs_grow / fs_shrink** — LightPlayer partition is fixed 1MB; no resize needed. Deferred.
- **Robustness testing** — Exhaustion, evil inputs, power-loss stress tests. Deferred; minimal test coverage per phase for now. Can be picked up later.

---

## Phases

| Phase | Focus |
|-------|--------|
| 01-metadata-read | Metadata parsing, stat, dir_read, fs_stat (read-only) |
| 02-block-caching | Block device read/prog caching |
| 03-directory-mutations | mkdir, remove, rename, block allocation |
| 04-file-read | File read (inline + CTZ) |
| 05-file-write | File write, sync, truncate, append |
| 06-power-loss | Global state, deorphan, mkconsistent, fs_gc |

Each phase spec (01-*.md) lists API targets, upstream test cases to port, and SPEC.md references.
