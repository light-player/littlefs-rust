# Test Parity 2 — Remaining Ignored Tests

Complement to the [2026-03-05 upstream test parity plan](../2026-03-05-upstream-test-parity/00-design.md). That plan covered stubbing, infrastructure, and implementing missing test cases. This plan covers what remains: feature-gating slow tests, implementing stubbed APIs, fixing bugs that block tests, and building the version compat infrastructure.

## Current State

~50 tests are `#[ignore]` across the suite. They break down into 7 categories:

| Category | Tests | Blocking reason |
|----------|-------|-----------------|
| Slow (power-loss / reentrant) | 25 | Runtime only — tests are implemented and would pass |
| `lfs_dir_seek` / `lfs_dir_tell` | 3 | APIs stubbed with `todo!()` |
| Superblock `block_count` validation | 1 | Bug: mount doesn't reject `block_count > superblock.block_count` |
| Space-only path names | 2 | Bug: path resolution fails for `" "`, `"  "`, etc. |
| Orphan internal APIs | 3 | `lfs_dir_alloc` / SOFTTAIL commit not exposed to tests |
| `metadata_max` compaction + shrink | 10 | Test bodies are `todo!()`; `metadata_max` compaction untested |
| Version compat | 17 | No multi-version test infrastructure |

Additionally, the upstream C test runner has power-loss modes (`log`, `exhaustive`) that the Rust suite doesn't implement. The current `run_powerloss_linear` matches upstream `linear`, and tests run under `none` (normal) by default. Missing modes are `log` (exponentially-decreasing) and `exhaustive` (all permutations).

## Inventory of Ignored Tests

### Slow / power-loss (implemented, need feature gate)

| File | Test | Reason |
|------|------|--------|
| test_dirs | `test_dirs_many_reentrant` | power-loss iteration |
| test_dirs | `test_dirs_file_reentrant` | power-loss iteration |
| test_files | `test_files_many_power_loss` | 300 files x power-loss |
| test_interspersed | `test_interspersed_reentrant_files` (6 cases) | power-loss |
| test_orphans | `test_orphans_reentrant` | power-loss iteration |
| test_relocations | `test_relocations_nonreentrant` (3 cases) | CYCLES=2000 |
| test_relocations | `test_relocations_nonreentrant_renames` (3 cases) | slow |
| test_relocations | `test_relocations_reentrant` (3 cases) | power-loss iteration |
| test_relocations | `test_relocations_reentrant_renames` (3 cases) | power-loss iteration |
| test_seek | `test_seek_reentrant_write` (3 cases) | power-loss iteration |
| test_superblocks | `test_superblocks_reentrant_format` | power-loss iteration |
| test_superblocks | `test_superblocks_reentrant_expand` | power-loss iteration |
| test_truncate | `test_truncate_reentrant_write` (3 cases) | power-loss iteration |

### `lfs_dir_seek` / `lfs_dir_tell` (API stubs)

| File | Test | Needs |
|------|------|-------|
| test_dirs | `test_dirs_remove_read` | `lfs_dir_seek` |
| test_dirs | `test_dirs_seek` | `lfs_dir_seek`, `lfs_dir_tell` |
| test_dirs | `test_dirs_toot_seek` | `lfs_dir_seek`, `lfs_dir_tell` |

### Bug fixes

| File | Test | Bug |
|------|------|-----|
| test_superblocks | `test_superblocks_fewer_blocks` | Mount should return `LFS_ERR_INVAL` when config `block_count` > superblock `block_count` |
| test_paths | `test_paths_oopsallspaces` (2 cases) | Path resolution fails for space-only names |

### Orphan internal APIs

| File | Test | Needs |
|------|------|-------|
| test_orphans | `test_orphans_normal` | `write_block_raw` orphan corruption recipe |
| test_orphans | `test_orphans_one_orphan` | `lfs_dir_alloc`, `lfs_dir_commit` with SOFTTAIL |
| test_orphans | `test_orphans_mkconsistent_one_orphan` | same |

### metadata_max compaction + shrink

| File | Test | Needs |
|------|------|-------|
| test_superblocks | `test_superblocks_metadata_max` (9 cases) | Test body + `metadata_max` compaction exercise |
| test_superblocks | `test_superblocks_shrink` | `shrink` feature verification + test body |

### Version compat (17 stubs)

| File | Test | Needs |
|------|------|-------|
| test_compat | `test_compat_forward_mount` | Self-test compat infra |
| test_compat | `test_compat_forward_read_dirs` | same |
| test_compat | `test_compat_forward_read_files` | same |
| test_compat | `test_compat_forward_read_files_in_dirs` | same |
| test_compat | `test_compat_forward_write_dirs` | same |
| test_compat | `test_compat_forward_write_files` | same |
| test_compat | `test_compat_forward_write_files_in_dirs` | same |
| test_compat | `test_compat_backward_mount` | same |
| test_compat | `test_compat_backward_read_dirs` | same |
| test_compat | `test_compat_backward_read_files` | same |
| test_compat | `test_compat_backward_read_files_in_dirs` | same |
| test_compat | `test_compat_backward_write_dirs` | same |
| test_compat | `test_compat_backward_write_files` | same |
| test_compat | `test_compat_backward_write_files_in_dirs` | same |
| test_compat | `test_compat_major_incompat` | Internal superblock APIs |
| test_compat | `test_compat_minor_incompat` | Internal superblock APIs |
| test_compat | `test_compat_minor_bump` | Internal superblock APIs |

## Phases

| Phase | File | Description | Tests unblocked |
|-------|------|-------------|-----------------|
| [01](01-slow-test-feature-flag.md) | Cargo.toml, all test files | `slow_tests` feature flag; replace `#[ignore]` with `#[cfg]` | 25 |
| [02](02-dir-seek-tell.md) | `src/dir/open.rs`, `src/lib.rs` | Implement `lfs_dir_seek_`, `lfs_dir_tell_` | 3 |
| [03](03-bug-fixes.md) | `src/fs/superblock.rs`, path handling | Fix `block_count` validation + space-only paths | 3 |
| [04](04-orphan-internals.md) | `src/dir/commit.rs`, test_orphans.rs | `#[cfg(test)]` pub re-exports; write orphan test bodies | 3 |
| [05](05-metadata-max-shrink.md) | test_superblocks.rs | Write `metadata_max` + shrink test bodies | 10 |
| [06](06-powerloss-modes.md) | `common/powerloss.rs` | Add `log` and `exhaustive` power-loss modes | 0 (parity gap) |
| [07](07-version-compat.md) | test_compat.rs, `src/` | Version compat self-test infra; implement 17 test cases | 17 |

Total: ~61 tests unblocked (some tests have multiple parameterized cases counted once above).
