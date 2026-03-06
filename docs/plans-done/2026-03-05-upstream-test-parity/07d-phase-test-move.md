# Phase 7d: test_move Relocation Cases

## Scope

Implement 2 test_move cases. These require `set_wear` or equivalent to force directory relocation during move.

**Rules:**

- **Implement tests only.** No bug fixes.
- **Match C exactly.** Read `reference/tests/test_move.toml` for each case.
- **If tests fail, ignore them.** `#[ignore = "…"]` is fine. Fix bugs later.

## Reference

`reference/tests/test_move.toml`

## Cases

### test_move_fix_relocation

```
defines.RELOCATIONS = range(4)    → for r in 0..4
defines.ERASE_CYCLES = 0xffffffff
```

Move file with `set_wear` to force directory relocation during move. Verify file content after. Requires WearLevelingBd or equivalent with per-block wear control.

### test_move_fix_relocation_predecessor

```
defines.RELOCATIONS = range(8)    → for r in 0..8
```

Move file between sibling and child dirs with forced relocations. Verify tree structure. Match C exactly.
