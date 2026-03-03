# C2Rust Translation Approach for LittleFS

**Purpose:** Produce a source-level translation of the reference LittleFS C code (`lfs.c`) into Rust, then wrap it with a safe API. This avoids hand-port bugs and simplifies upstream sync.

**Target:** A crate analogous to `littlefs2-sys` + `littlefs2`, but built from transpiled Rust instead of linked C. Unsafe core is acceptable; safe wrapper follows.

---

## Approach Summary

1. Use **C2Rust** to transpile `reference/lfs.c`, `reference/lfs_util.c`, and headers (`lfs.h`, `lfs_util.h`) into unsafe Rust.
2. Fix any translation artifacts (std→core, intrinsics, C lib stubs).
3. Build a **safe wrapper** (trait-based block device, `Result` types, ergonomic paths) around the generated API.
4. Validate via C↔Rust format alignment tests (e.g. C writes, transpiled Rust reads).

---

## Prerequisites

- **C2Rust:** `cargo install c2rust` (or build from [immunant/c2rust](https://github.com/immunant/c2rust)); requires nightly, LLVM/clang.
- **compile_commands.json:** Use Bear (`bear -- make`) or manually capture the compile command from the reference Makefile.

---

## Reference C Sources (from this repo)

| File | Role |
|------|------|
| `reference/lfs.c` | Core filesystem (~6550 lines) |
| `reference/lfs.h` | API, types, config, block device callbacks |
| `reference/lfs_util.c` | CRC implementation |
| `reference/lfs_util.h` | Macros, `static inline` helpers, malloc/assert overrides |

Build flags from reference Makefile: `-std=c99 -I. -Wall -Wextra`. Add `-DLFS_NO_MALLOC` for no_std; other `LFS_*` defines as needed.

---

## Steps (New Tree)

1. **Create a fresh workspace** (separate from `lp-littlefs`).

2. **Generate compile_commands.json**
   - `cd reference && bear -- make` (or equivalent)
   - Ensure `lfs.c` and `lfs_util.c` are both compiled with correct includes.

3. **Run C2Rust**
   ```bash
   c2rust transpile compile_commands.json --emit-modules
   ```
   Optionally `--emit-build-files` for Cargo scaffolding.

4. **Post-translation fixes**
   - Replace `std` with `core` where possible for `no_std`.
   - Map `memcpy`/`memset` to `core::ptr` or shims.
   - Map `assert` to `debug_assert!` or panic.
   - Map `__builtin_clz`/`ctz`/`popcount`/`bswap32` to Rust equivalents (`leading_zeros`, `trailing_zeros`, `count_ones`, `swap_bytes`).
   - Provide `lfs_crc` or equivalent; keep/port the CRC table from `lfs_util.c`.

5. **Public API boundary**
   - Expose only: `lfs_format`, `lfs_mount`, `lfs_unmount`, `lfs_mkdir`, `lfs_dir_*`, `lfs_file_*`, etc.
   - Keep internal `static` helpers crate-private or `pub(crate)`.

6. **Block device bridge**
   - `lfs_config` uses function pointers (`read`, `prog`, `erase`, `sync`).
   - Safe wrapper accepts a trait (e.g. `BlockDevice`) and bridges to these callbacks via `extern "C"` thunks.

7. **Validation**
   - Port or reference the `lp-littlefs-c-align` pattern: shared storage, C writes↔Rust reads, Rust writes↔C reads.
   - Use `littlefs2-sys` as the C reference for format compatibility tests.

---

## Key C2Rust Behaviors

- **Macros:** Clang preprocesses before translation; `LFS_MKTAG`, `LFS_MKATTRS` etc. are expanded. No special macro handling needed.
- **Known limitations:** LittleFS does not use `longjmp`/`setjmp`, variadic definitions, or non-x86 SIMD. Should translate cleanly.
- **Nightly:** Translator and possibly output may require nightly Rust. Check [c2rust#46](https://github.com/immunant/c2rust/issues/46).

---

## Success Criteria

- Transpiled crate compiles and runs `lfs_format` + `lfs_mount` + basic file/dir ops against an in-memory block device.
- Format alignment tests pass: C and transpiled Rust produce mutually readable on-disk layouts.
- Safe wrapper provides a `littlefs2`-style API without leaking `unsafe` to end users.

---

## Upstream Sync Workflow

1. Update reference C sources from upstream littlefs.
2. Regenerate `compile_commands.json`.
3. Re-run `c2rust transpile`.
4. Reapply minimal patches (no_std, shims).
5. Run tests.

Keep patches small and localized; prefer config/build changes over manual edits to generated logic.
