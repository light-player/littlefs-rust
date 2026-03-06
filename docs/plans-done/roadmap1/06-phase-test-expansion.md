# Phase 6: Test Coverage Expansion

Port remaining tests incrementally; implement until all pass. Reuse helpers from lp-littlefs-old where useful.

## Tasks

1. **Port test utilities**
   - RAM block device, common setup/teardown
   - Helpers from `lp-littlefs-old/tests/common/mod.rs`
   - Adapt for new crate API (raw `lfs_*` or wrapper)

2. **Port tests incrementally**
   - Add one test (or test module) at a time
   - Run → fix `todo!()` or bugs → pass → next test
   - Order by dependency: format/mount → mkdir → file create → file write → remove → rename → edge cases

3. **Reference lp-littlefs-old tests**
   - `test_superblocks`, `test_dirs`, `test_files`, `test_paths`, etc.
   - Parameterized tests: start with narrow ranges, expand once base passes

4. **Format alignment**
   - Keep lp-littlefs-c-align passing
   - C format ↔ Rust read, Rust format ↔ C read

5. **Edge cases (as needed)**
   - Power loss, exhaustion, bad blocks
   - Can be deferred if not critical for initial goals

## Success

- All ported tests pass
- Format alignment with C maintained
- No `todo!()` on exercised code paths
