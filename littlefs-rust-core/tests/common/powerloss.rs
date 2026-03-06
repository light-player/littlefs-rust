//! Power-loss simulation for tests.
//!
//! Per upstream lfs_emubd: trigger on write count (prog + erase). N = fail on Nth write.
//! Used by run_powerloss_linear / run_powerloss_log / run_powerloss_exhaustive.
//!
//! Supports two BD-level behaviours (upstream lfs_emubd powerloss modes):
//!   Noop — progs are atomic (either fully written or not)
//!   Ooo  — writes between syncs may be reordered; on power-loss the first
//!          block written since last sync is reverted to its pre-write state.

use core::cell::Cell;

use littlefs_rust_core::{Lfs, LfsConfig, LFS_ERR_IO};

use super::{RamStorage, BLOCK_SIZE};

/// How power-loss affects in-flight writes.
/// Upstream lfs_emubd_powerloss: NOOP vs OOO.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum PowerLossBehavior {
    Noop,
    Ooo,
}

/// Context for power-loss simulation. Wraps RAM storage and write count.
/// fail_after_writes=0 means never fail; N means fail on Nth prog/erase.
pub struct PowerLossCtx {
    pub ram: RamStorage,
    pub fail_after_writes: Cell<u32>,
    pub write_count: Cell<u32>,
    pub behavior: PowerLossBehavior,
    ooo_first_block: Option<u32>,
    ooo_block_backup: Vec<u8>,
}

impl PowerLossCtx {
    pub fn new(block_size: u32, block_count: u32) -> Self {
        Self::new_with_behavior(block_size, block_count, PowerLossBehavior::Noop)
    }

    pub fn new_with_behavior(
        block_size: u32,
        block_count: u32,
        behavior: PowerLossBehavior,
    ) -> Self {
        Self {
            ram: RamStorage::new(block_size, block_count),
            fail_after_writes: Cell::new(0),
            write_count: Cell::new(0),
            behavior,
            ooo_first_block: None,
            ooo_block_backup: Vec::new(),
        }
    }

    pub fn set_fail_after_writes(&self, n: u32) {
        self.fail_after_writes.set(n);
    }

    pub fn reset_write_count(&self) {
        self.write_count.set(0);
    }

    /// Check if we should fail on this write. Call before performing prog/erase.
    /// fail_after_writes=N means fail on Nth write (e.g. 1 = first write).
    fn check_and_count(&self) -> i32 {
        let count = self.write_count.get() + 1;
        self.write_count.set(count);
        let fail_at = self.fail_after_writes.get();
        if fail_at > 0 && count >= fail_at {
            return littlefs_rust_core::lfs_err!(LFS_ERR_IO);
        }
        0
    }

    /// Save the current contents of `block` before the first write since last sync.
    fn save_ooo_block(&mut self, block: u32) {
        let base = self.ram.block_offset(block);
        let end = base + self.ram.block_size as usize;
        self.ooo_block_backup.clear();
        self.ooo_block_backup
            .extend_from_slice(&self.ram.data[base..end]);
        self.ooo_first_block = Some(block);
    }

    /// On power-loss with OOO: revert the first block written since last sync.
    fn restore_ooo_block(&mut self) {
        if let Some(block) = self.ooo_first_block {
            let base = self.ram.block_offset(block);
            let end = base + self.ooo_block_backup.len();
            self.ram.data[base..end].copy_from_slice(&self.ooo_block_backup);
        }
    }

    fn clear_ooo_tracking(&mut self) {
        self.ooo_first_block = None;
        self.ooo_block_backup.clear();
    }
}

unsafe extern "C" fn powerloss_read(
    cfg: *const LfsConfig,
    block: u32,
    off: u32,
    buffer: *mut u8,
    size: u32,
) -> i32 {
    let ctx = (*cfg).context as *mut PowerLossCtx;
    let ctx = &mut *ctx;
    let size = size as usize;
    let buf = core::slice::from_raw_parts_mut(buffer, size);
    ctx.ram.read(block, off, buf);
    0
}

unsafe extern "C" fn powerloss_prog(
    cfg: *const LfsConfig,
    block: u32,
    off: u32,
    buffer: *const u8,
    size: u32,
) -> i32 {
    let ctx = (*cfg).context as *mut PowerLossCtx;
    let ctx = &mut *ctx;
    let err = ctx.check_and_count();
    if err != 0 {
        if ctx.behavior == PowerLossBehavior::Ooo {
            ctx.restore_ooo_block();
        }
        return littlefs_rust_core::lfs_pass_err!(err);
    }
    if ctx.behavior == PowerLossBehavior::Ooo && ctx.ooo_first_block.is_none() {
        ctx.save_ooo_block(block);
    }
    let size = size as usize;
    let buf = core::slice::from_raw_parts(buffer, size);
    ctx.ram.prog(block, off, buf);
    0
}

unsafe extern "C" fn powerloss_erase(cfg: *const LfsConfig, block: u32) -> i32 {
    let ctx = (*cfg).context as *mut PowerLossCtx;
    let ctx = &mut *ctx;
    let err = ctx.check_and_count();
    if err != 0 {
        if ctx.behavior == PowerLossBehavior::Ooo {
            ctx.restore_ooo_block();
        }
        return littlefs_rust_core::lfs_pass_err!(err);
    }
    if ctx.behavior == PowerLossBehavior::Ooo && ctx.ooo_first_block.is_none() {
        ctx.save_ooo_block(block);
    }
    ctx.ram.erase(block);
    0
}

unsafe extern "C" fn powerloss_sync(cfg: *const LfsConfig) -> i32 {
    let ctx = (*cfg).context as *mut PowerLossCtx;
    let ctx = &mut *ctx;
    ctx.clear_ooo_tracking();
    0
}

/// Test environment with power-loss simulation. Owns PowerLossCtx, config, buffers.
pub struct PowerLossEnv {
    pub ctx: PowerLossCtx,
    pub config: LfsConfig,
    pub _read_buf: Vec<u8>,
    pub _prog_buf: Vec<u8>,
    pub _lookahead_buf: Vec<u8>,
}

/// Build power-loss test environment. block_count defaults to 128.
/// fail_after_writes is set per run via set_fail_after_writes; pass 0 here initially.
pub fn powerloss_config(block_count: u32) -> PowerLossEnv {
    let block_size = BLOCK_SIZE;
    let ctx = PowerLossCtx::new(block_size, block_count);
    let read_buf = vec![0u8; block_size as usize];
    let prog_buf = vec![0u8; block_size as usize];
    let lookahead_buf = vec![0u8; block_size as usize];

    let config = LfsConfig {
        context: core::ptr::null_mut(),
        read: Some(powerloss_read),
        prog: Some(powerloss_prog),
        erase: Some(powerloss_erase),
        sync: Some(powerloss_sync),
        read_size: 16,
        prog_size: 16,
        block_size,
        block_count,
        block_cycles: -1,
        cache_size: block_size,
        lookahead_size: block_size,
        compact_thresh: u32::MAX,
        read_buffer: read_buf.as_ptr() as *mut core::ffi::c_void,
        prog_buffer: prog_buf.as_ptr() as *mut core::ffi::c_void,
        lookahead_buffer: lookahead_buf.as_ptr() as *mut core::ffi::c_void,
        name_max: 255,
        file_max: 2_147_483_647,
        attr_max: 1022,
        metadata_max: 0,
        inline_max: 0,
    };

    let mut env = PowerLossEnv {
        ctx,
        config,
        _read_buf: read_buf,
        _prog_buf: prog_buf,
        _lookahead_buf: lookahead_buf,
    };
    env.config.read_buffer = env._read_buf.as_mut_ptr() as *mut core::ffi::c_void;
    env.config.prog_buffer = env._prog_buf.as_mut_ptr() as *mut core::ffi::c_void;
    env.config.lookahead_buffer = env._lookahead_buf.as_mut_ptr() as *mut core::ffi::c_void;
    env
}

/// Build power-loss test environment with explicit BD behavior.
pub fn powerloss_config_with_behavior(
    block_count: u32,
    behavior: PowerLossBehavior,
) -> PowerLossEnv {
    let block_size = BLOCK_SIZE;
    let ctx = PowerLossCtx::new_with_behavior(block_size, block_count, behavior);
    let read_buf = vec![0u8; block_size as usize];
    let prog_buf = vec![0u8; block_size as usize];
    let lookahead_buf = vec![0u8; block_size as usize];

    let config = LfsConfig {
        context: core::ptr::null_mut(),
        read: Some(powerloss_read),
        prog: Some(powerloss_prog),
        erase: Some(powerloss_erase),
        sync: Some(powerloss_sync),
        read_size: 16,
        prog_size: 16,
        block_size,
        block_count,
        block_cycles: -1,
        cache_size: block_size,
        lookahead_size: block_size,
        compact_thresh: u32::MAX,
        read_buffer: read_buf.as_ptr() as *mut core::ffi::c_void,
        prog_buffer: prog_buf.as_ptr() as *mut core::ffi::c_void,
        lookahead_buffer: lookahead_buf.as_ptr() as *mut core::ffi::c_void,
        name_max: 255,
        file_max: 2_147_483_647,
        attr_max: 1022,
        metadata_max: 0,
        inline_max: 0,
    };

    let mut env = PowerLossEnv {
        ctx,
        config,
        _read_buf: read_buf,
        _prog_buf: prog_buf,
        _lookahead_buf: lookahead_buf,
    };
    env.config.read_buffer = env._read_buf.as_mut_ptr() as *mut core::ffi::c_void;
    env.config.prog_buffer = env._prog_buf.as_mut_ptr() as *mut core::ffi::c_void;
    env.config.lookahead_buffer = env._lookahead_buf.as_mut_ptr() as *mut core::ffi::c_void;
    env
}

/// Call after powerloss_config() to set context. Required for PowerLossEnv.
pub fn init_powerloss_context(env: &mut PowerLossEnv) {
    env.config.context = &mut env.ctx as *mut PowerLossCtx as *mut core::ffi::c_void;
}

impl PowerLossEnv {
    pub fn set_fail_after_writes(&self, n: u32) {
        self.ctx.set_fail_after_writes(n);
    }

    pub fn reset_write_count(&self) {
        self.ctx.reset_write_count();
    }

    /// Snapshot of RAM for later restore. Copy-on-restore for runner iterations.
    pub fn snapshot(&self) -> Vec<u8> {
        self.ctx.ram.data.clone()
    }

    /// Restore RAM from snapshot. Call before each runner iteration.
    pub fn restore(&mut self, snapshot: &[u8]) {
        self.ctx.ram.data.copy_from_slice(snapshot);
    }
}

/// Runs `op` repeatedly, failing at write N=1,2,3,… until `op` completes without power loss.
/// After each power loss: remount, run `verify`, then continue with next N.
///
/// - `snapshot`: BD state before each iteration (caller prepares via format/setup, then env.snapshot())
/// - `op`: mount, perform operations. Returns `Err(LFS_ERR_IO)` when power loss hits.
/// - `verify`: remount on partially-written BD, run consistency checks.
pub fn run_powerloss_linear<O, V>(
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
    let config_ptr = &env.config as *const LfsConfig;
    for n in 1..=max_iter {
        env.restore(snapshot);
        env.set_fail_after_writes(n);
        env.reset_write_count();

        let mut lfs = core::mem::MaybeUninit::<Lfs>::zeroed();
        match op(lfs.as_mut_ptr(), config_ptr) {
            Ok(()) => return Ok(()),
            Err(LFS_ERR_IO) => {
                verify(lfs.as_mut_ptr(), config_ptr)?;
            }
            Err(e) => return Err(e),
        }
    }
    Err(LFS_ERR_IO)
}

/// Like `run_powerloss_linear` but with exponential (log2) stepping:
/// fail at write N=1, 2, 4, 8, 16, … Useful for faster smoke testing.
///
/// Upstream: test_runner.c `log` mode.
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
    let config_ptr = &env.config as *const LfsConfig;
    let mut n: u32 = 1;
    while n <= max_iter {
        env.restore(snapshot);
        env.set_fail_after_writes(n);
        env.reset_write_count();

        let mut lfs = core::mem::MaybeUninit::<Lfs>::zeroed();
        match op(lfs.as_mut_ptr(), config_ptr) {
            Ok(()) => return Ok(()),
            Err(LFS_ERR_IO) => {
                verify(lfs.as_mut_ptr(), config_ptr)?;
            }
            Err(e) => return Err(e),
        }
        n = n.saturating_mul(2);
    }
    Err(LFS_ERR_IO)
}

/// Recursively explore all power-loss permutations up to `max_depth` levels deep.
/// At each depth: iterate write counts 1..max_iter; for each power-loss point
/// snapshot, verify, then recurse with depth-1.
///
/// Upstream: test_runner.c `exhaustive` mode.
pub fn run_powerloss_exhaustive<O, V>(
    env: &mut PowerLossEnv,
    snapshot: &[u8],
    max_iter: u32,
    max_depth: u32,
    mut op: O,
    mut verify: V,
) -> Result<(), i32>
where
    O: FnMut(*mut Lfs, *const LfsConfig) -> Result<(), i32>,
    V: FnMut(*mut Lfs, *const LfsConfig) -> Result<(), i32>,
{
    run_powerloss_exhaustive_inner(env, snapshot, max_iter, max_depth, &mut op, &mut verify)
}

fn run_powerloss_exhaustive_inner<O, V>(
    env: &mut PowerLossEnv,
    snapshot: &[u8],
    max_iter: u32,
    depth: u32,
    op: &mut O,
    verify: &mut V,
) -> Result<(), i32>
where
    O: FnMut(*mut Lfs, *const LfsConfig) -> Result<(), i32>,
    V: FnMut(*mut Lfs, *const LfsConfig) -> Result<(), i32>,
{
    let config_ptr = &env.config as *const LfsConfig;
    for n in 1..=max_iter {
        env.restore(snapshot);
        env.set_fail_after_writes(n);
        env.reset_write_count();

        let mut lfs = core::mem::MaybeUninit::<Lfs>::zeroed();
        match op(lfs.as_mut_ptr(), config_ptr) {
            Ok(()) => return Ok(()),
            Err(LFS_ERR_IO) => {
                verify(lfs.as_mut_ptr(), config_ptr)?;
                if depth > 1 {
                    let inner_snapshot = env.snapshot();
                    let inner = run_powerloss_exhaustive_inner(
                        env,
                        &inner_snapshot,
                        max_iter,
                        depth - 1,
                        op,
                        verify,
                    );
                    // Propagate real errors (verify failures); ignore Err(IO)
                    // which just means max_iter wasn't enough at this depth.
                    if let Err(e) = inner {
                        if e != LFS_ERR_IO {
                            return Err(e);
                        }
                    }
                }
            }
            Err(e) => return Err(e),
        }
    }
    Err(LFS_ERR_IO)
}
