# Summary: Format + Mount + Unmount

Plan completed. All phases implemented.

## Delivered

- **BlockDevice** trait with `read`, `prog`, `erase`, `sync`
- **RamBlockDevice** for tests (alloc-backed)
- **Config** with `default_for_tests(block_count)`
- **Error** enum (Corrupt, Io, Inval, Nospc)
- **Superblock** layout, tag constants, MAGIC, MAGIC_OFFSET
- **format** — writes revision + CREATE, SUPERBLOCK, INLINESTRUCT, CRC to blocks 0 and 1
- **mount** — reads metadata pair, picks block by revision, validates magic and superblock
- **unmount** — sync + teardown (no-op for now)

## Tests

- `test_bd.rs` — `test_bd_one_block` (erase, prog, read verification)
- `test_superblocks.rs` — format, mount, magic, invalid_mount

## Notes

- MAGIC at offset 12 (layout: [rev:4][create_tag:4][sb_tag:4][magic:8])
- Mount allows `block_count == 0` for unknown block count (read from disk)
