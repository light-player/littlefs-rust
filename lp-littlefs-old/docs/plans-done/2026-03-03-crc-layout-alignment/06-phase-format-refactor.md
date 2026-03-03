# Phase 6: Refactor format to use commit path

## Scope of phase

Refactor format.rs to follow C: use dir_alloc, lookahead, dir_commit_append with superblock attrs, dir_commit_crc. Second empty commit (root.erased=false, dir_commit with NULL) per C format.

## Code organization reminders

- Format creates a fresh FS; no prior root. C uses lfs_dir_alloc which allocates a pair from lookahead.
- Our dir_alloc expects (ctx, root, lookahead). For format, root is typically [0,1] but we don't have a root yet — we're creating it. C's lfs_alloc_scan populates lookahead; for format the device is empty so blocks 0,1 are free. Use root [0,1] or similar for the alloc scan. Actually alloc::alloc takes root and lookahead; it finds free blocks. For format, we could pass a dummy root or [0,1] — the alloc will return the first free blocks. On empty device, that's 0, 1.

### C format flow
1. lfs_init (lookahead, etc.)
2. lfs_dir_alloc → root pair (e.g. 0,1)
3. lfs_dir_commit(root, superblock attrs) → writes to root
4. root.erased = false
5. lfs_dir_commit(root, NULL, 0) → empty commit, forces compaction
6. lfs_dir_fetch to verify

### Our format flow (target)
1. Create rcache, pcache, ctx
2. alloc_scan or equivalent to populate lookahead for empty device
3. dir_alloc(ctx, root=[0,1], lookahead) → get root MdDir with pair
4. Build superblock attrs: CREATE(0,0), SUPERBLOCK(0,8)+"littlefs", INLINESTRUCT(0,24)+superblock
5. dir_commit_append(ctx, &mut root, attrs, None, DISK_VERSION)
6. root.erased = false
7. dir_commit_append(ctx, &mut root, &[], None, DISK_VERSION) — empty commit? Or dir_compact? C's second commit is lfs_dir_commit with NULL attrs — that goes through relocatingcommit. With NULL attrs it will try inline append (if erased) or compact. We set erased=false, so it won't try inline. It will compact. So the second commit does a compact with no attrs. That writes a new block with just revision + CRC. We need dir_orphaningcommit or similar with empty attrs. dir_orphaningcommit with &[] will call dir_relocatingcommit. Since erased=false, it won't do inline append. It will dir_splittingcompact with 0..0 or similar. Need to trace C's lfs_dir_commit with NULL,0.

Actually lfs_dir_commit with NULL attrs calls lfs_dir_orphaningcommit. That does relocatingcommit. With attrcount=0, the traverse writes nothing new. The dir has tail=null, count=0 or whatever from the superblock. So we're "committing" an empty set of changes. This effectively forces a compact — we rewrite the dir block. The purpose (per C comment): "force compaction to prevent accidentally mounting any older version of littlefs that may live on disk". So we're ensuring the block layout is clean.

For our refactor: after the first commit, call dir_orphaningcommit(&ctx, &mut root, &[], ..., disk_version) with empty attrs. That will compact since erased=false. We need the full FS context for orphaningcommit — root, lookahead, gstate, etc. Format doesn't have gstate (it's zero). Create minimal state: root=[0,1] or from dir_alloc, gstate=zero, gdisk=zero, gdelta=zero.

dir_orphaningcommit signature: ctx, dir, attrs, root, lookahead, name_max, gstate, gdisk, gdelta, skip_dir_adjust. Format would need to create these. root is the pair we're formatting — dir.pair after alloc. So root = dir.pair. lookahead we have. name_max from config or constant. gstate, gdisk, gdelta = GState::zero(). skip_dir_adjust = true maybe.

### Implementation
1. Create Lookahead, run alloc_scan for empty device (or alloc_ckpoint equivalent)
2. dir_alloc(ctx, [0,1], lookahead) — root for alloc; result gives our root pair
3. Build CommitAttr array: create(0), superblock(0,8,"littlefs"), inline_struct(0, superblock_bytes)
4. dir_commit_append(ctx, &mut root, &attrs, None, DISK_VERSION)
5. root.erased = false
6. dir_orphaningcommit(ctx, &mut root, &[], root.pair, &mut lookahead, name_max, &gstate, &mut gdisk, &mut gdelta, true)?
7. Optional: fetch to verify (C does this)

Format needs to create BdContext with rcache, pcache. And alloc::alloc_scan to populate lookahead. Check how alloc_scan works — it needs a root to traverse. For format we have no root yet. C's format uses lfs_alloc which uses lookahead. The lookahead is populated by lfs_alloc_scan which traverses the tail chain. For format there is no tail chain. So C must have a different path for initial lookahead. Looking at C lfs_alloc — it uses lookahead.buffer. For format, lookahead is created with size = 8*lookahead_size blocks. The lookahead might start empty and get populated on first alloc? Or alloc_scan is called with root [0,1] and traverses — but there's nothing there yet. This is a bootstrap problem. C's format: lfs_dir_alloc calls lfs_alloc twice. lfs_alloc will use lookahead. If lookahead is empty, it might scan. Need to check alloc code. For our purposes: format allocates a pair. Our alloc::alloc returns a free block. For a fresh device, we need to "find" free blocks. The lookahead in littlefs tracks which blocks are in use by scanning the metadata tail chain. On format, that chain doesn't exist. So we need a way to get the first two blocks. Typically 0,1 are the root. We could special-case format: use dir_alloc which will call alloc::alloc. Our alloc probably has logic for "no root" or initial state. Check alloc.rs.

Simpler approach: if the alloc/lookahead bootstrap is complex, we could keep format's current structure (erase 0,1, build block in buffer) but have it use dir_commit_crc logic for the CRC portion. That would mean a buffer-based variant of dir_commit_crc. That might be less disruptive. But the plan said "refactor format to use commit path". So we'll need to sort out the alloc bootstrap. For format, the root pair is often hardcoded as 0,1 in some implementations. We could have format pass root=[0,1] to dir_alloc — but dir_alloc doesn't allocate root, it allocates the NEW pair. The root for alloc is where we look for the free list. On format there's no free list. The lookahead might be populated by scanning — an empty scan would mean all blocks "free" in some default state. Our alloc module: need to read alloc.rs to see how it works for format.

Given complexity, a practical approach: Format uses dir_alloc with root=[0,1]. The alloc might need a "format mode" where we treat the device as empty. Or we run alloc_scan from root [0,1] — the blocks are erased, so fetch would fail. We might need alloc_scan to handle "no valid superblock" and treat whole device as free. This could be phase 6 scope creep. Document: if alloc bootstrap for format is non-trivial, consider a hybrid: format builds attrs in memory, then uses a minimal commit path that progs attrs then dir_commit_crc. That would require a commit path that accepts pre-built attrs and only does the CRC phase. Or we add format_commit that does: erase 0,1, prog revision, prog attrs (create, superblock, inline struct), then dir_commit_crc. That uses the shared CRC logic but keeps format's block-building. The key is dir_commit_crc must run — it needs to read (eperturb) and prog (FCRC, CCRC). So we need ctx with read/prog to the block. Format currently builds a buffer then progs. We could: build buffer up to attrs, prog that, then dir_commit_crc which will prog the rest. So we'd prog in two stages: (1) revision + attrs, (2) dir_commit_crc does FCRC/CCRC/padding. That could work! Format would:
1. Erase 0, 1
2. Prog revision (4 bytes padded to prog_size) to block 0
3. Prog attrs to block 0 (create, superblock, inline struct)
4. dir_commit_crc(ctx, 0, &mut off, &mut ptag, &mut crc, 0, block_size, prog_size, DISK_VERSION)
5. Copy to block 1 (C format writes to one block of pair first? Or both?) — C's dir_commit writes to dir.pair[0] for inline append. So we write to one block. The pair has two blocks; we write to pair[0]. We'd need to either write to both or match C. C's first commit writes to the allocated block (pair[0]). The second commit (empty) does a compact which writes to pair[1] then swaps. So after first commit we have data in pair[0]. After second commit we have compacted data in pair[0] (was pair[1]). For format, we could:
- First commit: write to block 0 (or whichever pair[0] is from alloc)
- Second commit: compact — writes to pair[1], swaps. So we need the full pair.
Actually C's dir_alloc gives us a pair. For format that's typically 0,1. We write first commit to block 0. Then second commit — orphaningcommit with [] — will compact. So we need dir_alloc, first commit (append to pair[0]), then orphaningcommit with []. The orphaningcommit will see erased=false (we set it), so it compacts. It writes to pair[1], then swaps. So we need the full machinery. Let me keep the format refactor as: use dir_alloc, dir_commit_append, dir_orphaningcommit. The alloc bootstrap: on a fresh device, alloc_scan from root [0,1] would try to fetch — and fail (no superblock). We might need a format-specific alloc that marks 0,1 as the root pair and treats the rest as free. Or we have alloc_init_for_format that sets up lookahead as "all free" or "first two blocks allocated for root". I'll add a note in the phase: if alloc bootstrap is complex, we may need alloc::init_for_format or similar.

## Validate

```bash
cd lp-littlefs && cargo test
# Run format/mount tests specifically
cargo fmt
```
