# Phase 1: Setup and Structure

Establish the new crate and workspace layout; preserve reference material; disable legacy build.

## Tasks

1. **Create lp-littlefs crate** (if replacing or alongside existing)
   - Cargo.toml: `no_std`, `alloc`, features (std, trace, etc.)
   - Basic `src/lib.rs` with `#![no_std]`, `extern crate alloc`

2. **Add to workspace**
   - Update root `Cargo.toml` members as needed
   - Ensure `lp-littlefs-c-align` (format alignment tests) can target new crate

3. **Disable old implementation**
   - Exclude `lp-littlefs-old` from workspace build, or
   - Disable its integration tests via `[[test]]` config or `--exclude`
   - Keep `lp-littlefs-old/` in tree for reference

4. **Reference C sources**
   - Ensure `reference/` contains: `lfs.c`, `lfs.h`, `lfs_util.c`, `lfs_util.h`
   - Add `reference/` to .gitignore if it is a submodule or external copy; otherwise track it

## Success

- `cargo build -p lp-littlefs` succeeds (empty or minimal stub crate)
- Old code and its tests are not built by default
- `reference/` is available for Phase 2–4
