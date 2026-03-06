#!/usr/bin/env bash
set -e

# Install Rust (rustup + stable toolchain) if not present
if ! command -v rustc &>/dev/null; then
    echo "Installing Rust..."
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --default-toolchain stable
    source "$HOME/.cargo/env"
else
    echo "Rust already installed: $(rustc --version)"
fi

# Ensure rustfmt is available
rustup component add rustfmt

# Add no_std target for embedded checks
rustup target add thumbv6m-none-eabi

# Install cargo-deny for license and advisory checks
cargo install cargo-deny

# Set up reference/ symlink to upstream littlefs
if [ -x scripts/upstream ]; then
    scripts/upstream sync
fi

echo "Dev environment ready. Run 'just ci' to verify."
