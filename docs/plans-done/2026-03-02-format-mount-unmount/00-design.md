# Design: Format + Mount + Unmount

## Scope of work

Implement the minimal littlefs bootstrap path and test infrastructure:

- Block device abstraction (trait + RamBlockDevice)
- Config/geometry with default for tests
- Format (write initial superblock to metadata pair)
- Mount (read and validate superblock)
- Unmount (sync and teardown)
- Integration tests in `tests/` with 1:1 mapping to upstream TOML, GitHub source links

Target upstream tests: `test_superblocks_format`, `test_superblocks_mount`, `test_superblocks_magic`, `test_superblocks_invalid_mount`, and `test_bd` cases.

## File structure

```
lp-littlefs/
├── Cargo.toml                    # UPDATE: alloc feature, dev-deps for tests
├── lp-littlefs/
│   ├── Cargo.toml                # UPDATE: alloc feature
│   └── src/
│       ├── lib.rs                # UPDATE: module tree, re-exports
│       ├── error.rs              # NEW: Error enum (Corrupt, Io, Inval, ...)
│       ├── config.rs             # NEW: Config, Geometry, default geometry
│       ├── block/
│       │   ├── mod.rs            # NEW: BlockDevice trait
│       │   └── ram.rs            # NEW: RamBlockDevice
│       ├── superblock.rs         # NEW: Superblock struct, tag constants, magic
│       └── fs/
│           ├── mod.rs            # NEW: LittleFs struct, format/mount/unmount API
│           ├── format.rs        # NEW: format implementation
│           └── mount.rs         # NEW: mount implementation
└── tests/
    ├── test_bd.rs               # NEW: block device tests (from test_bd.toml)
    └── test_superblocks.rs      # NEW: format/mount tests (from test_superblocks.toml)
```

## Conceptual architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                         LittleFs                                 │
│  format(config)  mount(config)  unmount()                        │
└────────────────────────┬────────────────────────────────────────┘
                         │
                         │ uses
                         ▼
┌─────────────────────────────────────────────────────────────────┐
│                       Config                                     │
│  read_size, prog_size, block_size, block_count, ...              │
└────────────────────────┬────────────────────────────────────────┘
                         │
                         │ constrains
                         ▼
┌─────────────────────────────────────────────────────────────────┐
│                   BlockDevice (trait)                            │
│  read(block, off, buf)  prog(block, off, data)  erase(block)  sync()  │
└────────────────────────┬────────────────────────────────────────┘
                         │
                         │ impl
                         ▼
┌─────────────────────────────────────────────────────────────────┐
│                  RamBlockDevice                                  │
│  alloc-backed storage                                            │
└─────────────────────────────────────────────────────────────────┘

format.rs / mount.rs use superblock.rs (layout, tags) + BlockDevice
to read/write metadata pairs (blocks 0, 1).
```

## Main components

- **Config** — Geometry (read_size, prog_size, block_size, block_count); passed into format/mount. Helper for test default geometry.
- **BlockDevice** — Trait with read, prog, erase, sync; caller-provided buffers.
- **RamBlockDevice** — In-memory implementation for tests.
- **superblock** — On-disk layout, tag constants, magic string per SPEC.
- **format** — Erase blocks 0, 1; write revision + commit (create, superblock name, inline struct, CRC).
- **mount** — Read metadata pair, find valid commit, parse superblock, validate.
- **LittleFs** — Public API; format/mount/unmount delegate to fs::format, fs::mount.
