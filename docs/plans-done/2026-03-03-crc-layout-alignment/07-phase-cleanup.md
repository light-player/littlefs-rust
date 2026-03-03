# Phase 7: Cleanup and validation

## Scope of phase

Remove temporary code, fix warnings, ensure all tests pass. Finalize the implementation.

## Code organization reminders

- No TODOs or debug prints
- Fix all linter warnings

## Implementation details

### 1. Grep for temporary code

```bash
cd lp-littlefs && grep -r "TODO\|FIXME\|XXX\|dbg!\|println!" --include="*.rs" src/
```

Remove or resolve any found.

### 2. Warnings

```bash
cargo build 2>&1 | grep warning
```

Fix each warning.

### 3. Formatting

```bash
cargo fmt
```

### 4. Tests

```bash
cargo test
```

### 5. Plan cleanup

- Add summary to `docs/plans/2026-03-03-crc-layout-alignment/summary.md`
- Move plan files to `docs/plans-done/`

### 6. Commit

Conventional commit format:
```
feat(littlefs): align CRC layout with upstream C for binary compatibility

- Add bd_crc for cached block CRC
- Implement dir_commit_crc (prog_size align, FCRC, CCRC loop, verify)
- Integrate into dir_commit_append and dir_compact
- Thread disk_version through commit callers
- Refactor format to use commit path
```

## Validate

```bash
cd lp-littlefs && cargo test
cargo fmt
cargo build 2>&1 | grep -E "warning|error" || true
```
