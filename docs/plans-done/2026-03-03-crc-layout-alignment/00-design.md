# CRC layout alignment — Design

## Scope of work

Implement full `lfs_dir_commitcrc` behavior in Rust for binary compatibility with upstream C:

1. Alignment to prog_size
2. 5-word CRC loop with optional FCRC
3. Valid bit from eperturb (power-loss detection)
4. Post-commit verification
5. Format refactor to use commit path

## File structure

```
lp-littlefs/src/
├── fs/
│   ├── bdcache.rs              # UPDATE: Add bd_crc (cached CRC)
│   ├── commit.rs               # UPDATE: Extract dir_commit_crc, use in dir_commit_append + dir_compact
│   └── format.rs               # UPDATE: Refactor to dir_alloc + dir_commit_append + dir_commit_crc
```

## Conceptual architecture

```
                    ┌─────────────────────────────────────┐
                    │         dir_commit_crc               │
                    │  (shared CRC loop, per lfs.c:1669)   │
                    │  - align end to prog_size             │
                    │  - loop: noff, FCRC?, CCRC           │
                    │  - valid bit from eperturb            │
                    │  - post-commit verify                 │
                    │  - conditional sync                    │
                    └─────────────────┬───────────────────┘
                                      │
         ┌────────────────────────────┼────────────────────────────┐
         │                            │                            │
         ▼                            ▼                            ▼
┌─────────────────┐        ┌─────────────────┐        ┌─────────────────┐
│ dir_commit_append│       │   dir_compact   │        │     format      │
│ (attrs → prog)   │       │ (traverse→prog) │        │ (superblock     │
│ then commit_crc  │       │ then commit_crc  │        │  attrs then     │
└─────────────────┘        └─────────────────┘        │  commit_crc)    │
                                                       └─────────────────┘
         │                            │                            │
         └────────────────────────────┼────────────────────────────┘
                                      │
                                      ▼
                    ┌─────────────────────────────────────┐
                    │         BdContext                   │
                    │  read, prog, sync, bd_crc (NEW)     │
                    └─────────────────────────────────────┘
```

## Main components

### dir_commit_crc
Shared helper taking `(ctx, block_idx, off, ptag, crc, begin, block_size, prog_size, disk_version)`.
- Compute `end = align_up(min(off + 20, block_size), prog_size)`
- Loop while `off < end`: noff, optional FCRC (read eperturb, bd_crc at noff, prog FCRC), CCRC (tag + 4-byte crc), update off/ptag/crc
- Post-commit: re-read begin..off1+4, verify crc; read stored CRC at off1, verify non-zero
- Conditional sync when noff >= end or cache full

### bd_crc
Add to bdcache: read region via bd_read (cached), accumulate crc::crc32. Used for FCRC and post-commit verify.

### Format refactor
Use dir_alloc (with lookahead), dir_commit_append (superblock attrs), dir_commit_crc. Second empty commit per C (root.erased=false, dir_commit with NULL attrs).
