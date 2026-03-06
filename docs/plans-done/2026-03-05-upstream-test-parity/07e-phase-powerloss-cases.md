# Phase 7e: Power-Loss Cases

## Scope

Implement 5 cases that use power-loss (reentrant) infrastructure. These run under simulated power-loss; the test runner may format/mount and retry.

**Rules:**

- **Implement tests only.** No bug fixes.
- **Match C exactly.** Read `reference/tests/test_*.toml` for each case.
- **If tests fail, ignore them.** `#[ignore = "…"]` is fine. Fix bugs later.

## Reference

- `reference/tests/test_dirs.toml`
- `reference/tests/test_superblocks.toml`
- `reference/tests/test_orphans.toml`

## Cases

### test_dirs_many_reentrant (test_dirs.rs)

```
defines.N = [5, 11]
defines.POWERLOSS_BEHAVIOR = [NOOP, OOO]
if = 'BLOCK_COUNT >= 4*N'
```

Reentrant mkdir/remove/rename loop under power-loss. Match C exactly.

### test_dirs_file_reentrant (test_dirs.rs)

```
defines.N = [5, 25]
defines.POWERLOSS_BEHAVIOR = [NOOP, OOO]
if = 'N < BLOCK_COUNT/2'
```

Reentrant file create/remove/rename under power-loss. Match C exactly.

### test_superblocks_reentrant_format (test_superblocks.rs)

```
defines.POWERLOSS_BEHAVIOR = [NOOP, OOO]
```

Format under power-loss, then mount and verify. Match C exactly.

### test_superblocks_reentrant_expand (test_superblocks.rs)

```
defines.BLOCK_CYCLES = [2, 1]
defines.N = 24
defines.POWERLOSS_BEHAVIOR = [NOOP, OOO]
```

Reentrant superblock expand with power-loss. Match C exactly.

### test_orphans_reentrant (test_orphans.rs)

```
defines.FILES = [6, 26, 3]
defines.DEPTH = [1, 3]
defines.CYCLES = 20
defines.POWERLOSS_BEHAVIOR = [NOOP, OOO]
```

Random mkdir/remove at varying depths under power-loss. Verify tree consistency after. Match C exactly.
