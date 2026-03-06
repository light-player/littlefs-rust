# Phase 2: Filesystem Lifecycle

## Goal

Implement `Filesystem<S>` with `format`, `mount`, `unmount`, and `Drop`. This is the hardest phase — it wires up the `Storage` trait to `LfsConfig` callbacks via unsafe trampolines and establishes the `RefCell` interior mutability pattern that everything else builds on.

## Internal types

### FsInner

```rust
struct FsInner<S: Storage> {
    lfs: MaybeUninit<lp_littlefs_core::Lfs>,
    config: lp_littlefs_core::LfsConfig,
    storage: S,
    read_buf: Vec<u8>,
    prog_buf: Vec<u8>,
    lookahead_buf: Vec<u8>,
    mounted: bool,
}
```

Owns the core `Lfs` state, the `LfsConfig`, the storage, and the cache buffers. The `mounted` flag tracks whether `lfs_unmount` needs to be called.

### Filesystem

```rust
pub struct Filesystem<S: Storage> {
    pub(crate) inner: RefCell<FsInner<S>>,
}
```

All methods take `&self`. The `RefCell` provides interior mutability. `pub(crate)` so `File` and `ReadDir` (in other modules) can borrow it.

## Trampolines

The core's `LfsConfig` requires `unsafe extern "C" fn` callbacks. The wrapper generates four trampolines that bridge to `Storage` trait methods:

```rust
unsafe extern "C" fn trampoline_read<S: Storage>(
    cfg: *const lp_littlefs_core::LfsConfig,
    block: u32,
    off: u32,
    buffer: *mut u8,
    size: u32,
) -> i32 {
    let storage = &mut *((*cfg).context as *mut S);
    let buf = core::slice::from_raw_parts_mut(buffer, size as usize);
    match storage.read(block, off, buf) {
        Ok(()) => 0,
        Err(_) => lp_littlefs_core::LFS_ERR_IO,
    }
}
```

Similarly for `trampoline_prog`, `trampoline_erase`, `trampoline_sync`.

The `LfsConfig.context` pointer is set to `&mut inner.storage as *mut S as *mut c_void`. This is valid because `FsInner` is pinned behind the `RefCell` and never moves while the filesystem is mounted.

**Safety argument:** The `RefCell<FsInner<S>>` is borrowed mutably during each core call. The core call may invoke storage callbacks via `LfsConfig`. The callbacks cast `context` back to `&mut S` and call trait methods. This is sound because:
1. The `RefCell` borrow is held for the entire core call, so no other code can access `FsInner` concurrently.
2. The core code does not re-enter itself (no core function calls another top-level `lfs_*` function).
3. `FsInner` is not moved while the borrow is held.

## Building LfsConfig

A private function `build_lfs_config<S: Storage>(inner: &mut FsInner<S>) -> ()` populates `inner.config` from the `Config` fields and wires up:
- `context` → `&mut inner.storage`
- `read` → `Some(trampoline_read::<S>)`
- `prog` → `Some(trampoline_prog::<S>)`
- `erase` → `Some(trampoline_erase::<S>)`
- `sync` → `Some(trampoline_sync::<S>)`
- `read_buffer` → `inner.read_buf.as_mut_ptr()`
- `prog_buffer` → `inner.prog_buf.as_mut_ptr()`
- `lookahead_buffer` → `inner.lookahead_buf.as_mut_ptr()`
- All size/count/limit fields from `Config`

The `cache_size` and `lookahead_size` are resolved (0 → `block_size`) before being used to allocate the buffer `Vec`s.

## Public API

### format

```rust
impl<S: Storage> Filesystem<S> {
    pub fn format(storage: &mut S, config: &Config) -> Result<(), Error> {
        // Build a temporary FsInner with the storage
        // Build LfsConfig, wire trampolines
        // Call lfs_format
        // Return Ok/Err
        // FsInner dropped (no mount needed)
    }
}
```

Borrows storage temporarily. Does not mount.

### mount

```rust
    pub fn mount(storage: S, config: Config) -> Result<Self, Error> {
        // Build FsInner, taking ownership of storage
        // Allocate cache buffers
        // Build LfsConfig, wire trampolines
        // Call lfs_mount
        // Set mounted = true
        // Return Filesystem { inner: RefCell::new(inner) }
    }
```

Takes ownership of `S`. The storage lives inside `FsInner` for the lifetime of the filesystem.

### unmount

```rust
    pub fn unmount(self) -> Result<S, Error> {
        // borrow_mut inner
        // Call lfs_unmount
        // Set mounted = false
        // Extract and return storage
    }
}
```

Consumes `self`, returns the storage. If unmount fails, the error is returned but the filesystem is still consumed (no further operations possible).

### Drop

```rust
impl<S: Storage> Drop for Filesystem<S> {
    fn drop(&mut self) {
        if let Ok(mut inner) = self.inner.try_borrow_mut() {
            if inner.mounted {
                let _ = lp_littlefs_core::lfs_unmount(inner.lfs.as_mut_ptr());
                inner.mounted = false;
            }
        }
    }
}
```

Best-effort. Uses `try_borrow_mut` to avoid panicking. Errors ignored.

## Tests

### test_format_mount_unmount

Format with `RamStorage`, mount, unmount, verify storage returned.

### test_mount_unformatted_fails

Mount without formatting → `Error::Corrupt`.

### test_drop_unmounts

Format, mount, drop `Filesystem` without explicit unmount. Verify no panic. Mount again to prove the drop unmounted cleanly.

### test_format_does_not_consume_storage

Format borrows storage; verify the storage is usable after format returns.

## Validate

```bash
cargo test -p lp-littlefs
```
