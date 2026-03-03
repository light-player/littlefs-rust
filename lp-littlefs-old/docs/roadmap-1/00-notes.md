# Notes: lp-littlefs Roadmap (LightPlayer-targeted)

## Scope of Work

Create phased roadmap specs for lp-littlefs implementation, targeted at supporting **LightPlayer on ESP32** (flash-backed LpFs). Each phase spec will serve as input to future detailed implementation plans.

**Principal goal**: Define high-level phases with API targets, upstream test references, and SPEC.md citations—so future plans can implement incrementally.

**Reference**: LightPlayer persistent FS plan at `/Users/yona/dev/photomancer/lp2025/docs/plans/2026-03-01-esp32-persistent-filesystem`

**Upstream littlefs**: `/Users/yona/dev/photomancer/oss/littlefs` (lfs.c, lfs.h, SPEC.md, DESIGN.md, tests/*.toml)

## Current State

### lp-littlefs (done)
- BlockDevice trait, RamBlockDevice
- Config (read_size, prog_size, block_size, block_count)
- Format, mount, unmount (stub unmount)
- Tests: test_bd.rs, test_superblocks.rs (format, mount, magic, invalid_mount)

### LightPlayer LpFs requirements
- `read_file`, `write_file`, `file_exists`, `is_dir`, `list_dir` (recursive), `delete_file`, `delete_dir` (recursive), `chroot`
- Paths: `/projects/<name>/...`, `lightplayer.json` at root
- Geometry from lp2025 plan: read=4, prog=4, block=4096, block_count=256 (1MB)
- ESP32, has alloc
- littlefs2-sys blocked on bare-metal → lp-littlefs is the pure-Rust alternative

## Questions

### Q1: LpFs compatibility layer ✅
**Answer**: LpFs is LightPlayer-only, outside scope. Roadmap targets raw littlefs API only. Adapter lives in lp2025/fw-esp32.

---

### Q2: block_count = 0 (read from disk) ✅
**Answer**: Defer. LightPlayer always knows partition size. Document in overview.

---

### Q3: Power-loss resilience depth ✅
**Answer**: Must have. Device can power-cycle mid-upload. Include mkconsistent, gc, deorphan, FCRC, move-state in scope.

---

### Q4: Custom attributes (getattr/setattr/removeattr) ✅
**Answer**: Defer. LpFs has no use. Document in overview.

---

### Q5: fs_grow / fs_shrink ✅
**Answer**: Defer. LightPlayer partition fixed 1MB. Document in overview.

---

### Q6: Test porting scope ✅
**Answer**: Minimal per phase. Each phase spec lists specific test cases that cover the new API. Robustness (exhaustion, evil, stress) deferred; can be picked up later.

---

## Notes

(Answers and additional notes will be recorded here.)
