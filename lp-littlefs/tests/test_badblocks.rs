//! Upstream: tests/test_badblocks.toml

mod common;

/// Upstream: [cases.test_badblocks_single]
/// defines.ERASE_VALUE = [0x00, 0xff, -1]
/// defines.BADBLOCK_BEHAVIOR = [PROGERROR, ERASEERROR, READERROR, PROGNOOP, ERASENOOP]
///
/// Single bad block; format/mount/ops with BADBLOCK_BEHAVIOR.
#[test]
#[ignore = "stub"]
fn test_badblocks_single() {
    todo!()
}

/// Upstream: [cases.test_badblocks_region_corruption]
/// defines.ERASE_VALUE = [0x00, 0xff, -1]
/// defines.BADBLOCK_BEHAVIOR = [PROGERROR, ERASEERROR, READERROR, PROGNOOP, ERASENOOP]
///
/// Region of bad blocks.
#[test]
#[ignore = "stub"]
fn test_badblocks_region_corruption() {
    todo!()
}

/// Upstream: [cases.test_badblocks_alternating_corruption]
/// defines.ERASE_VALUE = [0x00, 0xff, -1]
/// defines.BADBLOCK_BEHAVIOR = [PROGERROR, ERASEERROR, READERROR, PROGNOOP, ERASENOOP]
///
/// Alternating good/bad blocks.
#[test]
#[ignore = "stub"]
fn test_badblocks_alternating_corruption() {
    todo!()
}

/// Upstream: [cases.test_badblocks_superblocks]
/// defines.ERASE_VALUE = [0x00, 0xff, -1]
/// defines.BADBLOCK_BEHAVIOR = [PROGERROR, ERASEERROR, READERROR, PROGNOOP, ERASENOOP]
///
/// Bad blocks in superblock region.
#[test]
#[ignore = "stub"]
fn test_badblocks_superblocks() {
    todo!()
}
