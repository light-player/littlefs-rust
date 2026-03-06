# Phase 7: Safe API and Cleanup (Later)

Wrap the translated core in a safe Rust API. Optionally refactor internals toward safe Rust over time.

## Scope (deferred until Phases 1–6 complete)

1. **Safe wrapper API**
   - Trait-based block device (e.g. `BlockDevice`) instead of raw callbacks
   - `Result<(), Error>` instead of negative `lfs_ssize_t`
   - Path handling: `Path`/`PathBuf` or `&str` with validation

2. **Reduce unsafe**
   - Identify invariants; add `SAFETY` comments
   - Replace raw pointers with `NonNull` or references where possible
   - Use `MaybeUninit` instead of uninitialized buffers

3. **Ergonomics**
   - `Dir`, `File`, `LittleFs` handle types
   - `Iterator` for directory listing
   - `Read`/`Write` impls for files

## Prerequisites

- All Phase 6 tests pass
- Core translation is stable and verified

## Note

This phase is explicitly deferred. The initial goal is a working, tested translation. Safe wrapping can follow.
