# Plan: Missing Features (excl. migration, custom attributes)

## Scope of work

Implement the missing LittleFS features identified in the upstream comparison analysis, **excluding**:

- `lfs_migrate` — Migration from older on-disk versions
- `lfs_getattr` / `lfs_setattr` / `lfs_removeattr` — Custom attributes

**In scope:**

1. **Cross-directory rename** — `rename(old_path, new_path)` when old and new are in different directories. Currently returns `Error::Inval` due to `same_pair` check. Upstream supports this via move state (FROM_MOVE tag, prepmove, two-step commit).

2. **dir_seek, dir_tell, dir_rewind** — Directory iteration position. Upstream `lfs_dir_seek` / `lfs_dir_tell` / `lfs_dir_rewind` allow seeking to a position in a directory and rewinding. lp-littlefs has `Dir` with `pos` but no public API.

**Deferred (future plans):**

- **lfs_fs_grow** — Grow the filesystem to a larger block count.
- **lfs_file_opencfg** — Per-file buffer configuration.
- **Bad block handling** — Propagate `Error::Corrupt` from prog/erase.

## Current state of the codebase

### Cross-directory rename

**What's needed to match upstream:**

1. **Commit path (commit.rs):**
   - Add `CommitAttr::FromMove(new_id, old_id, old_pair)`.
   - Emit tag `(0x101 << 20) | (new_id << 10) | old_id` (LFS_FROM_MOVE), payload = 8 bytes (old_pair le).
   - Do not expand; write the actual FROM_MOVE tag. Struct stays in old dir.

2. **Read path (metadata.rs):** When resolving STRUCT for id X (in `get_struct`, `get_file_struct`, `get_entry_info`), if we encounter type3 == 0x101 (FROM_MOVE) with id == X: read 8-byte pair from payload, fetch that MdDir, recurse `get_struct` for `from_id` (tag size). These functions need `bd` and `config` to fetch other dirs. `dir_traverse_tags` and any struct-walking logic must also follow FROM_MOVE.

3. **Rename flow (fs/mod.rs):** Remove `same_pair` early return. For cross-dir: `prepmove(newoldid, old_pair)`; commit to new_cwd (CREATE, NAME, FROM_MOVE, maybe DELETE if overwriting); if cross-dir and hasmove, commit to old_cwd (DELETE old_id). Handle overwrite (prevtag), NOTEMPTY for dirs, rename-to-self.

4. **Tag iteration:** `get_tag_backwards` and similar must recognize type 0x101 and either (a) treat it as a "struct indirection" when we're looking for STRUCT, or (b) have a separate "get_struct_following_from_move" that does the indirection. Option (b) keeps get_tag_backwards simple.

- **fs/mod.rs** `rename()`: Lines 671–674 enforce `same_pair`; returns `Error::Inval` when `old_cwd.pair != new_cwd.pair`.
- Same-dir rename is implemented: create new entry, delete old, in one `dir_orphaningcommit`.
- Upstream uses: `lfs_fs_prepmove(lfs, newoldid, oldcwd.pair)`; commit to new_cwd with `LFS_FROM_MOVE` tag (payload = old_cwd); then commit to old_cwd with `DELETE` if cross-dir.
- **commit.rs**: Has `CommitAttr` variants; no `from_move`. Would need `CommitAttr::from_move(new_id, old_id, old_pair)` and corresponding tag emission.
- **gstate.rs**: Has `prepmove`, `hasmove`, `hasmovehere`; movestate is already supported for persistence.

### dir_seek / dir_tell / dir_rewind

- **fs/dir.rs**: `dir_read` advances `Dir.pos` (1, 2, 3, …) and `Dir.id` per metadata block. No way to rewind or seek to a stored position.
- **Dir** struct: `head`, `mdir`, `id`, `pos`, `is_root`, `name_max`. Position is implicit in (`mdir`, `id`); `pos` is a linear index.
- Upstream: `lfs_dir_seek` rewinds then `dir_read`s until reaching the target offset. `lfs_dir_tell` returns the current offset. `lfs_dir_rewind` resets to start.

### lfs_fs_grow

- **fs/mod.rs**: No `fs_grow`. Mount reads `block_count` from superblock.
- **fs/mount.rs**: `MountState` has `block_count` from disk.
- Upstream: Fetches root, reads superblock from root, updates `block_count`, commits superblock back to root.

### lfs_file_opencfg

- **fs/file.rs**: `File` uses a cache from the config. No per-file buffer override.
- Low impact; defer unless explicitly needed.

### Bad block handling

- **BlockDevice** trait: `prog` and `erase` return `Result<(), Error>`. `Error::Corrupt` exists.
- Commit paths do not propagate or handle `Corrupt` specially. Would require plumbing through many call sites.

## Questions

### Q1: Cross-directory rename — FROM_MOVE and commit path

**Context:** Upstream uses `LFS_FROM_MOVE` (type 0x101) with payload = pointer to source dir. The commit logic recursively traverses the source dir’s struct tags and applies them with an id delta. lp-littlefs `dir_orphaningcommit` takes explicit `CommitAttr`; we don’t have a generic “copy from other dir” path.

**Suggested approach:** Add `CommitAttr::FromMove { new_id, old_id, old_pair }`. In the commit append logic, when emitting this tag: (1) fetch the struct from `old_pair` at `old_id` (dir_struct or ctz_struct); (2) emit the equivalent create+struct attrs for `new_id` with the same data. This avoids implementing full recursive traverse; we only need to copy the one struct. Validate against upstream by checking that test_move_file (a/hello → c/hello) passes.

**Resolved:** User wants to match upstream closely. See "What's needed to match upstream" below.

### Q2: dir_seek / dir_tell — position representation

**Context:** Upstream uses a linear offset (number of entries from start). `dir_seek(off)` rewinds and dir_reads `off` times. `dir_tell` returns that offset.

**Suggested approach:** Store `pos` as the entry index (0 = before ".", 1 = after ".", 2 = after "..", etc.). Add `dir_seek(dir, off)`, `dir_tell(dir)`, `dir_rewind(dir)` to `LittleFs`. `dir_rewind` sets `pos=0`, `id`/`mdir` to initial state. `dir_seek` rewinds then calls `dir_read` until we've skipped `off` entries. `dir_tell` returns `pos`. This matches upstream semantics.

**Resolved:** Use `i64` for API consistency with upstream `lfs_soff_t`.

### Q3: lfs_fs_grow — validation

**Resolved:** Defer fs_grow to a later plan.

### Q4: Phasing of optional items (file_opencfg, bad blocks)

**Context:** `file_opencfg` and bad block handling are lower priority.

**Resolved:** Defer file_opencfg and bad block handling. Plan phases focus on: cross-dir rename, dir_seek/tell/rewind.
