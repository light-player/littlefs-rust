# Phase 3: Function Inventory and Layout

Manually extract all C functions; group by responsibility; map to target .rs modules.

## Tasks

1. **Extract function list from lfs.c (and lfs_util.c if used)**
   - For each function: name, signature (params + types), return type
   - Approximate line range for C source lookup
   - Note `static` vs public; forward declarations

2. **Group functions logically**
   - **bd (block device):** `lfs_bd_read`, `lfs_bd_prog`, `lfs_bd_crc`, `lfs_bd_flush`, `lfs_bd_sync`, `lfs_bd_erase`, `lfs_bd_cmp`, `lfs_cache_drop`, `lfs_cache_zero`
   - **alloc:** `lfs_alloc`, `lfs_alloc_scan`, `lfs_alloc_lookahead`, `lfs_alloc_ckpoint`, `lfs_alloc_drop`
   - **dir (metadata/commit):** `lfs_dir_*` — fetch, getgstate, getinfo, commit, commitattr, commitcrc, alloc, compact, split, drop, etc.
   - **file:** `lfs_file_*`, `lfs_ctz_*`
   - **fs (high-level):** `lfs_format_`, `lfs_mount_`, `lfs_unmount_`, `lfs_fs_*`, `lfs_mkdir_`, `lfs_remove_`, `lfs_rename_`
   - **util:** endianness helpers, `lfs_fcrc_*`, `lfs_ctz_*` (struct)

3. **Use existing docs**
   - `lp-littlefs-old/docs/alignment/metadata-vs-lfs-c.md` — function mappings
   - `lp-littlefs-old/docs/plans-done/` — commit, format, mount structure

4. **Produce module map**
   - Table: C function → target .rs file
   - E.g. `lfs_dir_commit` → `src/dir/commit.rs` or `src/commit.rs`

## Deliverable

- Function inventory (list or table)
- Module assignment for each function
- Basis for Phase 4 stub generation
