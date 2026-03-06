# Phase 4: Stubbed Implementation

Populate .rs files with function stubs, `todo!()`, and commented C source. Wire call graph so the crate compiles.

## Tasks

1. **Create module structure**
   - Add `mod bd;`, `mod alloc;`, `mod dir;`, `mod file;`, `mod fs;` (or equivalent) to `lib.rs`
   - Create each .rs file according to Phase 3 module map

2. **For each C function**
   - Add Rust function with same signature (types from Phase 2)
   - Body: `todo!("lfs_function_name")`
   - Above or below: block comment with original C source (e.g. `/* C: ... */` or `//! C: lfs.c:1234-1280`)

3. **Wire the call graph**
   - Public API entry points (`lfs_format`, `lfs_mount`, etc.) call internal stubs
   - Internal stubs call each other per C call graph
   - All stubbed functions compile; none run until invoked by a test

4. **Block device bridge**
   - `lfs_config` holds function pointers (read, prog, erase, sync)
   - Rust: closure/trait or raw fn pointers; struct layout compatible with C if needed for alignment tests

## Success

- `cargo build -p littlefs-rust` succeeds
- Every C function has a corresponding Rust stub with `todo!()`
- C source is present as comments; call graph is wired
- Running any test that touches the API will panic at the first `todo!()` in that path
