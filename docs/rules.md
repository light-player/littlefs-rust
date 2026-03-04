# C-to-Rust Translation Rules

Hand-translation of LittleFS from C to Rust. These rules govern how to translate functions from the reference C source (`reference/lfs.c`, `lfs_util.c`, `lfs.h`, `lfs_util.h`).

Refer to the C code in `reference/` for the original source, mostly `reference/lfs.c`.

## 0. Before You Translate

- **Translate callees first**: Before implementing a function, ensure every function it calls is already implemented (or stubbed). Implement leaf functions first, then work up the call graph. Avoids nested `todo!()` panics and lets you test each function in isolation.

## 1. Assertions

- Translate every C `LFS_ASSERT(cond)` to `lfs_assert!(cond)` in Rust (uses `debug_assert!` under the hood)
- Do not remove assertions; they encode invariants from the C implementation
- If the C assertion has a message, preserve it: `lfs_assert!(cond, "message")`
- Keep assertions even when they seem redundant; they aid debugging and catch divergence early

## 2. Code Fidelity

- Match C logic and control flow as closely as possible
- Preserve the order of operations when it affects semantics
- Do not refactor, simplify, or "improve" logic during translation; that belongs in later phases (e.g. Phase 7 safe API)
- Prefer matching C structure over idiomatic Rust until tests pass

## 3. Reference in File

- Include the original C source as a comment above or below each translated function
- Use format like `/// C: lfs.c:1234-1280` or a fenced code block with the C excerpt
- Include line numbers from the reference file for traceability and upstream sync

## 4. Signatures and Types

- **Double-check signatures**: Compare the Rust signature to the C declaration (parameter types, return type, constness). Ensure Phase 2 structs and types are used consistently.
- Prefer concrete pointer types over `void*`: Use `*mut Lfs`, `*const LfsMdir`, `*const lfs_mattr`, etc., instead of `*mut core::ffi::c_void` where the C type is known. For raw byte buffers, prefer `*const u8` / `*mut u8` over `*const c_void`. Do not change semantics; the goal is to avoid casts at every call site. Keep `repr(C)` struct fields matching C (e.g. `buffer: *const c_void` in `lfs_mattr`); function parameters may use more specific types.
- Preserve pointer parameters (`*mut`, `*const`) where the C uses pointers
- Keep the same error model: C returns negative `int` → Rust returns `i32` with `LFS_ERR_*` constants

## 5. Unsafe Usage

- Use `unsafe` for pointer dereferences and raw buffer access when translating C pointer operations
- Do not hide pointer access behind safe abstractions in the core translation
- The goal is a faithful translation first; safe wrappers come later (Phase 7)

## 6. Divergences

- Do not silently change behavior from C
- If intentional divergence is required, document it (in alignment docs, function comment, or `lp-littlefs-old/docs/alignment/`) with rationale
- When tests reveal a divergence, fix the translation before changing the test; never relax a test to accommodate a bug in the translation

## 7. Macros and Helpers

- Map C macros (`LFS_MKTAG`, `LFS_MKATTRS`, etc.) to equivalent Rust macros or functions with identical semantics
- Do not change semantics when translating; e.g. tag encoding, CRC logic, and layout must match

## 8. Call Graph

- Preserve the C call graph: same functions call the same callees in the same order
- Stub with `todo!("lfs_function_name")` until implemented; the first panic guides the next implementation step

## 9. Implementation Details

- **Null checks**: Where C checks `ptr != NULL` before use, preserve the check in Rust (e.g. `!attrs.is_null()` before `from_raw_parts`).
- **Error propagation**: Mirror C control flow: check `err != 0`, return early on non-recoverable errors, handle special cases (e.g. `LFS_ERR_CORRUPT` vs `LFS_ERR_NOSPC`) exactly as in C.
- **cfg access**: Where C assumes `lfs->cfg` is non-null after init, use `unwrap()` or `expect()` only when the C contract guarantees it; otherwise add explicit null checks.
- **goto / control flow**: For C `goto`, preserve structure in Rust using nested blocks, `loop`/`continue`, or helper functions. Do not rewrite into different control-flow idioms if it could change behavior.
- **C casts**: When C uses casts like `(const uint8_t*)buffer`, document the mapping in a comment (e.g. `// C: (const uint8_t*)buffer` → `buffer as *const u8`).

## 10. Copying Tests from Reference

Tests are defined in upstream TOML files (`tests/test_*.toml`) which contain C snippets run by the littlefs test framework. Port them to Rust integration tests, preserving alignment with the reference.

- **Source**: `https://github.com/littlefs-project/littlefs/blob/master/tests/*.toml`. Commit in `docs/reference.md`.

- **Names**: Keep the same test names as upstream. `[cases.test_bd_one_block]` → `test_bd_one_block`. Module mapping: `test_dirs.toml` → `tests/test_dirs.rs`, etc.

- **Permutations**: Use `rstest` with `#[rstest]` and `#[case(...)]`. Map TOML defines to concrete cases:
  - `defines.READ = ['READ_SIZE', 'BLOCK_SIZE']` (and similar) → `#[case(read_size, prog_size)]` with concrete pairs (e.g. `(16, 16)`, `(512, 512)`).
  - `defines.N = 'range(3, 100, 3)'` → start with a subset (e.g. `#[case(5)]#[case(8)]#[case(10)]`); expand once stable.
  - Add a comment above each parameterized test: `// Upstream: defines.X = [...], defines.Y = [...]. Subset: ...`.

- **Refer to C files**: Each test module header: `//! Upstream: tests/test_dirs.toml` plus GitHub URL. Per-test comment: `// Upstream: [cases.test_dirs_many_rename]` (or the C API calls exercised).

- **Strategy**: Start with narrow parameter ranges; broaden after implementation is stable. See `lp-littlefs-old/docs/2026-03-03-parameterized-test-bugs.md` for failures when ranges are expanded too early.
