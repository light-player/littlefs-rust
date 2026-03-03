# lp-littlefs-c-align

C ↔ Rust format alignment tests for lp-littlefs. Uses [littlefs2-sys](https://crates.io/crates/littlefs2-sys) to run the reference C implementation alongside our Rust port.

## Purpose

Isolate whether format/semantics bugs are in our write path or read path by:
- **C writes, Rust reads**: If this passes, C produces valid layout; bug is in our Rust write.
- **Rust writes, C reads**: If this passes, our Rust write produces valid format; bug is in our read.

Targets the known failure: `mkdir("potato")` + `file_open("burito", CREAT)` → after remount, potato disappears.

## Build requirements

- C compiler (for littlefs C sources)
- clang (for bindgen)

`cargo build -p lp-littlefs-c-align` and `cargo test -p lp-littlefs-c-align` compile and link the C littlefs library.

**arm64 macOS workaround**: The repo `.cargo/config.toml` sets `BINDGEN_EXTRA_CLANG_ARGS=--target=arm64-apple-darwin` to work around a bindgen/libclang issue where the wrong target is inferred, causing "Unsupported architecture" and a size_t assertion. If you still see failures, ensure you are using native arm64 clang (not x86 under Rosetta).

## Tests

Run alignment tests:

```
cargo test -p lp-littlefs-c-align
```

| Test | Operation | Verification |
|------|-----------|--------------|
| `c_format_rust_mount_root` | C formats | Rust mounts, reads root |
| `rust_format_c_mount_root` | Rust formats | C mounts, reads root |
| `c_mkdir_file_rust_sees_both` | C: format, mkdir, file_create | Rust sees both potato and burito |
| `rust_mkdir_file_c_sees_both` | Rust: format, mkdir, file_create | C sees both |
| `c_mkdir_remount_exist` | C: format, mkdir, remount | C mkdir same name returns LFS_ERR_EXIST |
| `c_rename_rust_sees_new_name` | C: format, create, rename | Rust sees only new name |
| `rust_rename_c_sees_new_name` | Rust: format, create, rename | C sees only new name (ignored: DELETE visibility bug) |
| `c_remove_rust_sees_gone` | C: format, create, remove | Rust sees entry gone |
| `rust_remove_c_sees_gone` | Rust: format, create, remove | C sees entry gone |
| `c_write_rust_reads_content_*` | C: format, create, write | Rust reads same content (inline/CTZ) |
| `rust_write_c_reads_content_*` | Rust: format, create, write | C reads same content (inline ok; CTZ ignored: bdcache bug) |
| `c_nested_dir_rust_sees` | C: format, mkdir a, mkdir a/b, create a/b/f | Rust sees hierarchy |
| `rust_nested_dir_c_sees` | Rust: format, mkdir a, mkdir a/b, create a/b/f | C sees hierarchy |
| `c_rmdir_rust_sees_gone` | C: format, mkdir, create file, remove file, rmdir | Rust sees dir gone |
| `rust_rmdir_c_sees_gone` | Rust: same | C sees dir gone (ignored: NotEmpty bug) |
| `format_layout_prog_size_*` | Format with prog_size 16/64 | Other impl mounts successfully |
