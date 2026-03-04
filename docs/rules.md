# C-to-Rust Translation Rules

Hand-translation of LittleFS from C to Rust. These rules govern how to translate functions from the reference C source (`reference/lfs.c`, `lfs_util.c`, `lfs.h`, `lfs_util.h`).

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

- Match C function signatures using the Phase 2 struct/enum definitions
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
