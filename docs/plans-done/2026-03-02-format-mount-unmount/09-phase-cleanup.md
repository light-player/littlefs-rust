# Phase 9: Cleanup and validation

## Scope of phase

Remove temporary code, fix warnings, run full validation, finalize plan.

## Implementation details

### 1. Grep for TODOs and temporary code

```bash
cd lp-littlefs && grep -r "TODO\|FIXME\|XXX\|dbg!\|println!" --include="*.rs" .
```

Remove or resolve any found.

### 2. Warnings and formatting

```bash
cd lp-littlefs && cargo build
cd lp-littlefs && cargo test
cargo fmt
cargo clippy
```

Fix all warnings and apply formatting.

### 3. Plan summary

Add `summary.md` to the plan directory with completed work summary.

Move plan to `docs/plans-done/` (from lp-littlefs root):

```bash
mkdir -p docs/plans-done
mv docs/plans/2026-03-02-format-mount-unmount docs/plans-done/
```

### 4. Commit

```
feat(littlefs): format, mount, unmount with block device abstraction

- Add BlockDevice trait and RamBlockDevice
- Add Config, Error, superblock layout
- Implement format (write initial superblock) and mount (validate)
- Add test_bd and test_superblocks integration tests
- 1:1 mapping to upstream test_bd.toml and test_superblocks.toml
```
