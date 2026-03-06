# Contributing to littlefs-rust

Contributions are welcome — bug reports, fixes, documentation improvements, and feature ideas all
help.

## Reporting bugs

Open a [GitHub issue](https://github.com/light-player/littlefs-rust/issues). Include:

- What you expected vs. what happened
- Minimal reproduction steps or a failing test case
- Block device geometry (block size, block count) if relevant

## Submitting a pull request

1. Fork the repo and create a branch from `main`.
2. Make your changes. Run `just fci` (format + full CI) before pushing.
3. Open a pull request. Describe what the change does and why.

### Commit conventions

Use [Conventional Commits](https://www.conventionalcommits.org/): `<type>(<scope>): <description>`

- Types: `feat`, `fix`, `chore`, `docs`, `refactor`, `test`
- Scope: crate or component name (e.g., `littlefs`, `allocator`)

Examples: `fix(core): off-by-one in CTZ index calculation`, `feat(littlefs): add attr get/set to safe API`

### Setup

```bash
./dev-init.sh
```

Installs Rust (stable + rustfmt), the `thumbv6m-none-eabi` target for `no_std` checks, and
cargo-deny. Sets up `reference/` (symlink to the upstream littlefs C source).

Requires [just](https://github.com/casey/just) to run CI recipes locally.

### Running tests

```bash
cargo test -p littlefs-rust          # safe wrapper
cargo test -p littlefs-rust-core     # core
just compat                        # C compatibility (requires C toolchain)
```

## Syncing upstream littlefs

To bring in bug fixes or changes from [littlefs-project/littlefs](https://github.com/littlefs-project/littlefs):

1. Create a branch: `git checkout -b upstream-sync/$(date +%Y%m%d)`
2. Generate the report: `just upstream-report` — review `.upstream-cache/pending-sync.md`.
3. Generate the agent prompt: `just upstream-prompt` — follow the steps manually or give the prompt to an AI assistant.
4. Verify: `just fci`
5. Push and open a PR.

The `scripts/upstream` script supports: `sync`, `log`, `diff`, `report`, `prompt`, `bump`.
Run `scripts/upstream` with no args for usage.
