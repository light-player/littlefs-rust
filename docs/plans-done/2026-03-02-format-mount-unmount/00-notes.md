# Plan: Format + Mount + Unmount

## Scope of work

Implement the minimal littlefs bootstrap path and test infrastructure:

1. **Block device abstraction** — Trait and RAM-backed implementation
2. **Config/geometry** — Parameters for read/prog/block sizes and block count
3. **Format** — Write initial superblock to metadata pair (blocks 0, 1)
4. **Mount** — Read and validate superblock
5. **Unmount** — Sync and teardown
6. **Test infrastructure** — Integration tests in `tests/` with 1:1 mapping to upstream TOML files, comments linking to GitHub source

Target upstream tests (from `test_superblocks.toml`):

- `test_superblocks_format` — Format only
- `test_superblocks_mount` — Format, mount, unmount
- `test_superblocks_magic` — Format, then raw read to assert "littlefs" at offset 8
- `test_superblocks_invalid_mount` — Mount on blank device returns corrupt error

Optional for this plan: `test_bd.toml` — Block device tests (erase/prog/read) to validate RamBlockDevice before littlefs uses it.

## Current state of the codebase

- **lp-littlefs** — Workspace member, `#![no_std]`, placeholder `LittleFs` struct only
- **Cargo.toml** — Has `std` feature, no test deps
- **No** block device, config, format, or mount implementation
- **No** `tests/` directory
- **Upstream reference** — `../oss/littlefs` (SPEC.md, lfs.c, lfs.h, tests/*.toml, bd/lfs_rambd.c)

## Questions

### Q1: no_std and allocation

**Resolved:** Use alloc. Add `alloc` feature now; for the moment only support having it enabled. Structure the code so allocation happens in dedicated functions/modules that can be feature-gated later if we add a no-alloc path. This keeps the door open for a future alloc-optional mode without major refactoring.

---

### Q2: Error representation

**Resolved:** Plain enum with `#[derive(Debug)]`. Must remain no_std compatible (no thiserror).

---

### Q3: Default geometry

**Resolved:** Start with one geometry (upstream "default": read_size=16, prog_size=16, erase_size=512, block_size=512; smaller block_count for fast tests). Structure tests so geometry is easy to change later (e.g. helper fn or const that returns Config/Geometry).

---

### Q4: Block device trait signature

**Resolved:** Caller-provided buffer: `read(&self, block, off, buffer: &mut [u8]) -> Result<()>`. Same for `prog` (with `&[u8]`). Trait includes `read`, `prog`, `erase`, `sync`.

---

### Q5: Test file mapping

**Resolved:** Include both. `tests/test_bd.rs` validates RamBlockDevice first; `tests/test_superblocks.rs` then tests format/mount/unmount. Each file has header comment linking to upstream GitHub source.
