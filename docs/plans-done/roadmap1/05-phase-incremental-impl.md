# Phase 5: Incremental Implementation

Implement functions one (or a few) at a time, driven by failing tests. First vertical slice: format + mount.

## Tasks

1. **Choose minimal vertical slice**
   - Example: format a RAM block device, then mount
   - Requires: BD layer (`lfs_bd_*`), alloc (`lfs_alloc`), dir alloc/commit (`lfs_dir_alloc`, `lfs_dir_commit`, `lfs_dir_commitcrc`, etc.), format (`lfs_format_`), mount (`lfs_mount_`), init (`lfs_init`), deinit (`lfs_deinit`)

2. **Add or enable one test**
   - e.g. `test_format_then_mount` — format, unmount, mount, succeed
   - Test calls public API; will panic at first `todo!()` in path

3. **Implement in call order**
   - Run test → panic in `lfs_bd_read` (or similar) → implement that function from C comment
   - Re-run → next panic → implement that function
   - Repeat until test passes

4. **Translate style**
   - Preserve C logic and control flow
   - Use `unsafe` for pointer derefs, raw buffer access
   - Match error returns (negative ints → `LFS_ERR_*`)

5. **Validate early**
   - Run format-alignment tests (littlefs-rust-c-align) once format + mount work
   - Ensure on-disk layout matches C

## Success

- At least one end-to-end test passes (e.g. format + mount)
- All functions on that path are implemented, not stubbed
- Format alignment with C confirmed
