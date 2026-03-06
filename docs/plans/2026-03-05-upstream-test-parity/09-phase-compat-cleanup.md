# Phase 9: test_compat + Cleanup & Validation

## Scope

1. Implement test_compat cases (or defer if version infrastructure is missing)
2. Final cleanup and validation of the entire test suite

## test_compat.rs — 17 cases

### Prerequisites

test_compat requires:
- A "previous" littlefs implementation (`lfsp_*` functions) that can format/mount on the same BD
- `LFSP_DISK_VERSION_MAJOR`, `LFSP_DISK_VERSION_MINOR` constants
- Ability to modify superblock version fields via internal APIs

If this infrastructure doesn't exist yet, all 17 cases remain `#[ignore = "stub: requires version compat infrastructure"]`.

### Forward compatibility (7 cases)

All guarded by `LFS_DISK_VERSION_MAJOR == LFSP_DISK_VERSION_MAJOR && DISK_VERSION == 0`.

| Case | Summary |
|------|---------|
| `test_compat_forward_mount` | Format with lfsp, mount with lfs, check fsinfo.disk_version |
| `test_compat_forward_read_dirs` | lfsp creates 5 dirs, lfs reads them |
| `test_compat_forward_read_files` | lfsp creates 5 files (SIZE=[4,32,512,8192], CHUNK=4), lfs reads |
| `test_compat_forward_read_files_in_dirs` | lfsp creates dirs+files, lfs reads |
| `test_compat_forward_write_dirs` | lfsp creates 5 dirs, lfs creates 5 more, lists all |
| `test_compat_forward_write_files` | lfsp writes first half, lfs writes second half (CHUNK=2) |
| `test_compat_forward_write_files_in_dirs` | Same in nested dirs |

### Backward compatibility (7 cases)

All guarded by `LFS_DISK_VERSION == LFSP_DISK_VERSION && DISK_VERSION == 0`.

| Case | Summary |
|------|---------|
| `test_compat_backward_mount` | Format with lfs, mount with lfsp |
| `test_compat_backward_read_dirs` | lfs creates 5 dirs, lfsp reads |
| `test_compat_backward_read_files` | lfs creates 5 files, lfsp reads |
| `test_compat_backward_read_files_in_dirs` | lfs creates dirs+files, lfsp reads |
| `test_compat_backward_write_dirs` | lfs creates 5, lfsp creates 5 more |
| `test_compat_backward_write_files` | lfs writes first half, lfsp writes second half |
| `test_compat_backward_write_files_in_dirs` | Same in nested dirs |

### Version edge cases (3 cases)

| Case | Summary |
|------|---------|
| `test_compat_major_incompat` | Bump major version +1, expect mount → LFS_ERR_INVAL |
| `test_compat_minor_incompat` | Bump minor version +1, expect mount → LFS_ERR_INVAL |
| `test_compat_minor_bump` | Downgrade minor, mount works, write triggers minor bump to current |

## Cleanup & Validation

### Code cleanup

- Remove any remaining `println!` debug output from tests
- Remove any `#[allow(unused)]` that are no longer needed
- Verify all `#[ignore]` attributes have descriptive messages
- Ensure all test files have the `//! Upstream: tests/test_*.toml` header
- Verify Rust-specific extras are in clearly marked sections

### Final validation

```bash
# Full suite (all non-ignored tests pass)
cargo test -p lp-littlefs 2>&1

# List all tests
cargo test -p lp-littlefs -- --list 2>&1 | wc -l

# Count stubs remaining
cargo test -p lp-littlefs -- --list --ignored 2>&1 | grep -c "stub"

# Formatting and lints
cargo fmt -p lp-littlefs --check
cargo clippy -p lp-littlefs -- -D warnings
```

### Coverage report

Write a summary to `docs/plans/2026-03-05-upstream-test-parity/COMPLETION.md`:

```
# Upstream Test Parity — Completion Report

## Coverage

| File | Upstream cases | Implemented | Stubbed | Extras |
|------|---------------|-------------|---------|--------|
| test_alloc | N | N | 0 | K |
| ... | ... | ... | ... | ... |

## Known gaps
- test_compat: 17 cases deferred (version infra needed)
- test_shrink: 2 cases deferred (LFS_SHRINKNONRELOCATING needed)
- ...

## Bugs found during parameterization
- ...
```
