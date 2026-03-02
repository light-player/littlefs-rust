default:
    just ci

# Run all CI checks locally.
# First-time setup: just install-tools
ci: lint build features no_std deny

# One-time setup: run dev-init.sh to install Rust, cargo-deny, and targets
install-tools:
    ./dev-init.sh

# Lint: check formatting
lint:
    cargo fmt --all -- --check

# Build and test
build:
    cargo build
    cargo test --all --all-features --verbose

# Check with all features enabled
features:
    cargo check --all-features

# Check no_std build (embeddable target)
no_std:
    rustup target add thumbv6m-none-eabi
    cargo check --target thumbv6m-none-eabi --no-default-features

# Cargo deny: license and advisory checks (run dev-init.sh first if needed)
deny:
    cargo deny check
