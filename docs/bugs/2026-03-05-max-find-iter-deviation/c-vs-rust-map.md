# C vs Rust Line Mapping

## lfs_dir_splittingcompact

| C (lfs.c) | Rust (dir/commit.rs) | Notes |
|-----------|----------------------|-------|
| 2128-2174 while/split loop | 1428-1481 | Split threshold logic |
| 2175-2177 `if (split == begin) break` | 1468-1471 | No split needed |
| (none) | 1472-1479 | **FIX**: `if (end_val <= split) break` — C does not have this guard; Rust needed it to avoid empty-range splits |
| 2180-2195 lfs_dir_split + end=split | 1482-1489 | Split and reduce end |

## Divergence

C `lfs_dir_splittingcompact` can reach `split=1, end=1` after a prior split (e.g. split=1, end=2; then end=split=1). When that happens, the next iteration would call `lfs_dir_split(source, 1, 1)`, compacting the empty range [1,1). C may also have this latent bug but the test geometry or code path avoids it. Rust hit it. The guard `end_val <= split` correctly prevents splitting empty ranges in both implementations.
