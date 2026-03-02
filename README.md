# lp-littlefs

Pure Rust implementation of the LittleFS embedded filesystem. No C dependencies—avoids C compiler,
bindgen, and cross-compilation headaches on embedded targets (ESP32, RISC-V, etc.).

On-disk format compatible with upstream LittleFS for interoperability.

Created for use in [LightPlayer](https://github.com/light-player/lightplayer), an LED lighting
control system, for use on esp32 and other embedded targets.

## Development

One-time setup:

```bash
./dev-init.sh
```

Installs Rust (stable + rustfmt), the `thumbv6m-none-eabi` target for no_std checks, and cargo-deny.
Requires [just](https://github.com/casey/just) to run the CI recipe locally.

```bash
just ci
```

## Upstream and reference

- [littlefs-project/littlefs](https://github.com/littlefs-project/littlefs) — original C implementation
- [DESIGN.md](https://github.com/littlefs-project/littlefs/blob/master/DESIGN.md) — design and rationale
- [SPEC.md](https://github.com/littlefs-project/littlefs/blob/master/SPEC.md) — on-disk format specification

## Prior art

- [littlefs2](https://crates.io/crates/littlefs2) — idiomatic Rust API wrapping the C library via FFI (requires C toolchain)
- [littlefs2-sys](https://crates.io/crates/littlefs2-sys) — low-level C bindings, compiles littlefs C source
- [chamelon](https://github.com/yomimono/chamelon) — pure OCaml implementation, interoperable with littlefs; inspiration for clean-room ports

## Why lp-littlefs

Existing Rust options (`littlefs2`, `littlefs2-sys`) depend on the C library and thus require a C
toolchain for every target. lp-littlefs is a from-spec Rust port—no C, no bindgen,
no cross-compilation toolchain headaches.

## Versioning

Releases are automated via [release-plz](https://release-plz.dev/) using conventional commits:

- `fix:` → patch (1.0.0 → 1.0.1)
- `feat:` → minor (1.0.0 → 1.1.0)
- `feat!:` or `BREAKING CHANGE` → major (1.0.0 → 2.0.0)

## Usage

Add as a git dependency:

```toml
lp-littlefs = { git = "https://github.com/light-player/lp-littlefs", branch = "main" }
# or pin to a release:
lp-littlefs = { git = "https://github.com/light-player/lp-littlefs", rev = "v0.1.0" }
```

## Status

Early-stage; API unstable.
