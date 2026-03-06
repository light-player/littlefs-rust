# lp-littlefs — Agent instructions

## Commits

Use Conventional Commits format: `<type>(<scope>): <description>`

- Types: feat, fix, chore, docs, refactor, etc.
- Scope: crate or component name (e.g. `littlefs`, `allocator`)
- Description: short description of the commit
- Body: optional; bulleted list of changes when not obvious from description

Keep code compiling and tests passing between commits when possible.

## Formatting

Run `cargo fmt` on all changes before committing.

## Warnings

Fix warnings before committing. Do not ignore them.

## Code organization

- Place helper utility functions at the bottom of files
- Place more abstract things, entry points, and tests first
- Keep related functionality grouped together

## Tests

- Tests in `mod tests` at the top of the module
- Short and concise; use utility functions to avoid duplication
- Test helpers at the bottom of the test module
- Prefer clear test names over inline comments
- Avoid debug `println!` in tests unless debugging a specific failure
- Use `unwrap_or_else` / `expect` with helpful error messages
- Each test should test one thing clearly
- Never change a test to make it pass when it fails due to a bug—fix the bug, not the test

## Style

- Use `#![no_std]` where appropriate
- Gate std-dependent code behind features
- Follow existing code style

## C-to-Rust Translation

When translating functions from the reference C code (`reference/lfs.c`, `lfs_util.c`) to Rust:

- **Use the local reference only**: Read from `reference/lfs.c` etc. in this repo. Do NOT fetch lfs.c or other C source from the web. If `reference/` is missing, run `scripts/upstream sync`.

- Translate callees first; ensure each function called is already implemented (or stubbed) before implementing
- Double-check signatures against C; prefer concrete types (`*mut Lfs`, `*const u8`) over `*mut c_void` where known
- Preserve all `LFS_ASSERT` → translate to `lfs_assert!` (or `debug_assert!` where appropriate)
- Match C logic and control flow as closely as possible; do not refactor or simplify during translation
- Include original C source as comments above/below each function with line references (e.g. `//! C: lfs.c:1234-1280`)
- Use `unsafe` for pointer dereferences and raw buffer access; keep the same error model (negative int → `LFS_ERR_*`)
- Document any intentional divergence from C; do not silently change behavior
- See [docs/rules.md](docs/rules.md) for the full translation rules

## Language

- Keep language professional and restrained
- Avoid overly optimistic language ("comprehensive", "fully production ready")
- No emoticons
- Use measured, factual descriptions
