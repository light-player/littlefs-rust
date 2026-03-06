default:
    just ci

# Run all CI checks locally.
# First-time setup: just install-tools
ci: lint build features no_std deny

# Format code and apply linter fixes (clippy, rustfix)
fix:
    cargo fmt --all
    cargo fix --allow-dirty --allow-staged
    cargo clippy --all-targets --all-features --fix --allow-dirty --allow-staged

# Fix, then run CI
fci: fix ci

# One-time setup: run dev-init.sh to install Rust, cargo-deny, and targets
install-tools:
    ./dev-init.sh

# Lint: check formatting
lint:
    cargo fmt --all -- --check

# Build and test (default members only; excludes lp-littlefs-compat)
build:
    cargo build
    cargo test --all-features --verbose

# Build and test lp-littlefs-compat (C ↔ Rust interop).
# On macOS arm64, sets BINDGEN_EXTRA_CLANG_ARGS for bindgen.
compat:
    #!/usr/bin/env bash
    set -e
    if [[ "$(uname -s)" == "Darwin" ]] && [[ "$(uname -m)" == "arm64" ]]; then
        export BINDGEN_EXTRA_CLANG_ARGS="--target=arm64-apple-darwin"
    fi
    cargo build -p lp-littlefs-compat
    cargo test -p lp-littlefs-compat

# Check with all features enabled
features:
    cargo check --all-features

# Check no_std build (embeddable target; core only — wrapper needs alloc)
no_std:
    rustup target add thumbv6m-none-eabi
    cargo check -p lp-littlefs-core --target thumbv6m-none-eabi --no-default-features

# Cargo deny: license and advisory checks (run dev-init.sh first if needed)
deny:
    cargo deny check
