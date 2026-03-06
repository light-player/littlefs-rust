//! Unit test helpers. Not shipped with the library.
//!
//! Provides TestContext, assertion helpers, and SuperblockSnapshot for tests that need
//! to exercise format/traverse/commit in isolation. Integration tests in tests/ use
//! tests/common which has a similar but separate setup.

mod assert;
mod context;
mod ram;
mod snapshot;
mod tests;

pub use assert::{assert_block_has_magic, assert_blocks_0_and_1_have_magic};
pub use context::TestContext;
pub use ram::{MAGIC, MAGIC_OFFSET};
pub use snapshot::SuperblockSnapshot;
