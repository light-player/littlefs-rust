# Phase 7: Missing Cases in Existing Test Files

## Overview

Phase 7 is split into sub-phases. Each sub-phase implements tests that match the C reference exactly.

**Critical rules for all Phase 7 work:**

1. **Implement tests only.** We are not fixing bugs in the library. We are writing tests.
2. **Match the C reference exactly.** Read `reference/tests/test_*.toml` and translate the code block faithfully.
3. **If tests fail, that's fine.** Use `#[ignore = "…"]` if needed. We fix bugs later.
4. **Do not simplify or "improve" the tests.** Faithful translation from C.

## Sub-phases

| Phase | File | Scope |
|-------|------|-------|
| [07a](07a-phase-test-paths-c-align.md) | test_paths.rs | C-alignment gaps: trailing_dotdots, utf8_ipa, oopsallspaces, oopsalldels, oopsallffs |
| [07b](07b-phase-test-dirs-seek.md) | test_dirs.rs | remove_read, seek, toot_seek (no power-loss) |
| [07c](07c-phase-test-superblocks-simple.md) | test_superblocks.rs | Simple cases: mount_unknown_block_count, stat_tweaked, expand, magic_expand, expand_power_cycle, unknown_blocks, fewer_blocks, more_blocks |
| [07d](07d-phase-test-move.md) | test_move.rs | fix_relocation, fix_relocation_predecessor |
| [07e](07e-phase-powerloss-cases.md) | test_dirs.rs, test_superblocks.rs, test_orphans.rs | Power-loss cases: many_reentrant, file_reentrant, reentrant_format, reentrant_expand, orphans_reentrant |
| [07f](07f-phase-orphans-powerloss-advanced.md) | test_orphans.rs, test_powerloss.rs, test_superblocks.rs | Advanced: orphans_normal, one_orphan, mkconsistent_one_orphan, partial_prog, grow, shrink, metadata_max |

## Code organization (all sub-phases)

- Place upstream cases first, extras at the bottom
- Include upstream defines + summary comment on every test
- Keep related functionality grouped together
