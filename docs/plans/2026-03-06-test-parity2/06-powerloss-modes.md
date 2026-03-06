# Phase 6: Power-Loss Modes

## Scope

Add upstream power-loss runner modes (`log`, `exhaustive`) and BD-level out-of-order write behavior (`OOO`). This phase does not unblock new tests — it closes a parity gap in how existing reentrant tests are exercised.

## Background

### Runner modes (WHEN power-loss occurs)

Upstream `test_runner.c` supports these modes:

| Mode | Algorithm | Our status |
|------|-----------|------------|
| `none` | No power-loss, run normally | Implemented (default test run) |
| `linear` | Fail at write N=1, then N=2, N=3, ... | Implemented (`run_powerloss_linear`) |
| `log` | Fail at write N=1, then N=2, N=4, N=8, ... | **Missing** |
| `exhaustive` | Recursively explore all power-loss permutations | **Missing** |

Default upstream is `{none, linear}`. The `log` mode is useful for faster smoke testing (exponential skip). The `exhaustive` mode explores all multi-power-loss combinations (very slow, used for deep validation).

### BD behavior (HOW power-loss affects writes)

The upstream `lfs_emubd` has:

| Behavior | Meaning | Our status |
|----------|---------|------------|
| `NOOP` | Progs are atomic — either fully written or not | Implemented (implicit) |
| `OOO` | Writes between syncs may be reordered; on power-loss, an earlier block write may persist instead of a later one | **Missing** |

`OOO` simulates real flash behavior where a write buffer may commit blocks to flash in a different order than they were programmed. This is important for validating that the FS doesn't assume write ordering between syncs.

## Implementation

### 1. `run_powerloss_log` in `common/powerloss.rs`

Same structure as `run_powerloss_linear` but with exponential step:

```rust
pub fn run_powerloss_log<O, V>(
    env: &mut PowerLossEnv,
    snapshot: &[u8],
    max_iter: u32,
    mut op: O,
    mut verify: V,
) -> Result<(), i32>
where
    O: FnMut(*mut Lfs, *const LfsConfig) -> Result<(), i32>,
    V: FnMut(*mut Lfs, *const LfsConfig) -> Result<(), i32>,
{
    let mut n: u32 = 1;
    while n <= max_iter {
        env.restore(snapshot);
        env.set_fail_after_writes(n);
        env.reset_write_count();

        let mut lfs = core::mem::MaybeUninit::<Lfs>::zeroed();
        match op(lfs.as_mut_ptr(), &env.config as *const _) {
            Ok(()) => return Ok(()),
            Err(LFS_ERR_IO) => verify(lfs.as_mut_ptr(), &env.config as *const _)?,
            Err(e) => return Err(e),
        }
        n *= 2;
    }
    Err(LFS_ERR_IO)
}
```

### 2. `run_powerloss_exhaustive` in `common/powerloss.rs`

This is more complex. The upstream version recursively explores: at each power-loss point, it snapshots the BD, then continues with the next power-loss point from that snapshot. This means for depth D, it explores all D-tuples of power-loss points.

Design sketch:
- Accept a `max_depth` parameter (upstream uses `SIZE_MAX` for full exploration or a specific depth)
- At each depth level: iterate write counts 1..N (until op completes without IO error)
- For each power-loss point: snapshot, verify, then recurse with depth-1

This is a large addition. Implement the simple (depth=1) case first, which is identical to `linear`. Multi-depth can be added later.

### 3. OOO write behavior in `PowerLossCtx`

Add a `PowerLossBehavior` enum and OOO tracking:

```rust
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum PowerLossBehavior {
    Noop, // Progs are atomic
    Ooo,  // Writes between syncs may be reordered
}
```

For OOO mode, `PowerLossCtx` needs:
- Track the first block written since last sync (the "OOO candidate")
- On power-loss trigger: restore the OOO candidate block's data from before the first write, simulating that the first write was the one that persisted
- On sync: clear the OOO tracking

This requires saving a copy of the first block written after each sync, so it can be restored on power-loss.

### 4. Integration with existing tests

No existing tests need changes. The new modes can be used by:
- Replacing `run_powerloss_linear` with `run_powerloss_log` in select tests for faster iteration
- Adding OOO as a parameter to `powerloss_config` (default: `Noop`)
- Future: parameterize reentrant tests over `PowerLossBehavior`

## Validate

```bash
# Existing power-loss tests still pass
cargo test -p lp-littlefs-core --features slow_tests -- test_powerloss

# New modes work with a simple smoke test
cargo test -p lp-littlefs-core test_powerloss_runner_smoke_log
cargo test -p lp-littlefs-core test_powerloss_runner_smoke_exhaustive
```

## Test additions

Add to `test_powerloss.rs`:
- `test_powerloss_runner_smoke_log` — same as `test_powerloss_runner_smoke` but using `run_powerloss_log`
- `test_powerloss_runner_smoke_exhaustive` — same but using `run_powerloss_exhaustive` with depth=2
- `test_powerloss_ooo_smoke` — write two blocks without sync, trigger power-loss, verify one may be out of order
