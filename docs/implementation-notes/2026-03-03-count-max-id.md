# Count logic: max_id + 1 divergence from C

**Date:** 2026-03-03

## What differs

In `fetch_metadata_pair` (metadata.rs), at each CRC block we apply:

```rust
if seen_name_or_splice {
    tempcount = tempcount.max(0).max(max_id as i32 + 1);
} else {
    tempcount = tempcount.max(0);
}
```

**C does not.** C uses `tempcount` as-is from the NAME and SPLICE updates only (lfs.c:1247–1254).

## Why we keep it

C’s logic can yield a count that is too low for append renames. Example: CREATE 2, NAME 2, DELETE 1 (single-entry rename to a name that sorts after):

- CREATE 2: tempcount += 1 → 3
- NAME 2: id 2 < 3, so no bump
- DELETE 1: tempcount += -1 → 2

So C gets count = 2. Iteration is `id < count` (0..2), so id 2 is never visited. The new entry at id 2 is missed.

With `max_id + 1`, we force `count >= 3`, so iteration reaches id 2.

## Why C’s tests pass without it

Upstream `test_dirs_many_rename` renames `test%03d` → `tedd%03d`. Because `"tedd" < "test"`, every rename inserts before existing entries; append renames are not exercised. An append-rename test was added (`test_dirs_many_rename_append`, a→z) and C passes it with N=5,7,9,11.

## Caveat: empty child dirs

`max_id + 1` is only applied when `seen_name_or_splice` is true. Child dirs with no NAME/SPLICE (e.g. only SOFTTAIL) must stay at count = 0 so `remove` does not incorrectly return NotEmpty.

## Status

We keep `max_id + 1` because tests (e.g. `test_dirs_many_rename`, `fetch_after_rename_commit`) depend on it. C may have a latent bug in the append-rename case that its tests do not hit, or our logic compensates for a remaining difference elsewhere. A future cleanup would be to remove it and fix any regressions by aligning more closely with C’s block/tag handling.
