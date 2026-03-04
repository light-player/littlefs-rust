# A/B Comparison: Rust vs C test_superblocks_magic

## Summary

**H1 CONFIRMED**: Rust `lfs_dir_traverse` pops immediately after push instead of getting the next tag from disk/attrs. C gets the next tag. This causes SUPERBLOCK to be skipped or wrong data to be committed.

## Rust Log Analysis

From `rust-test_superblocks_magic-*.log`:

```
traverse GetNextTag: push tag=0x0ff00008 type3=255 buffer=0x... attr_i=2   # SUPERBLOCK
traverse GetNextTag: sp=1 phase=GetNextTag
traverse GetNextTag: popped tag=0x0ff00008 type3=255 buffer=0x...          # BUG: pop instead of get next
traverse ProcessTag: sp=0 tag=0x0ff00008 type3=255 buffer=0x...            # Process SUPERBLOCK
commit_commit_raw: tag=0x0ff00008 ... buffer=0x...
bd_prog superblock block=0 off=8 size=8 magic_region[4..8]=[108, 101, 102, 115]  # "lefs" - OK for first commit
```

Then on second compact:
```
traverse GetNextTag: push tag=0x00000000 type3=0 buffer=0x0 attr_i=1        # CREATE -> filter set to NOOP
traverse GetNextTag: popped tag=0x00000000                                 # Pop instead of get SUPERBLOCK
bd_prog magic_region[4..8]=[1, 0, 0, 0]                                    # Wrong data
```

**Key bug**: When `sp > 0` at start of GetNextTag, Rust always pops. C gets the next tag from disk/attrs first. After push, the next action in C is "get next tag", not "pop".

## C Log Analysis

From `c-format-and-dump-*.log`:
- Block 0 bytes 8-15: `6c 69 74 74 6c 65 66 73` ("littlefs") - correct
- Block 1 bytes 8-15: same - correct
- C format produces correct magic

## Conclusion

Fix: Separate "get next" (always from disk/attrs) from "pop" (only when exhausted or callback returned non-zero). Do not pop at the start of GetNextTag when we came from a push.
