# Contributing to lp-littlefs

## General

- **Commit conventions**: See [AGENTS.md](AGENTS.md) for Conventional Commits format and style.
- **CI**: Run `just fci` before pushing. Full CI: `just ci`.
- **Setup**: Run `./dev-init.sh` once to install Rust, targets, cargo-deny, and set up the `reference/` symlink.

## Syncing upstream littlefs

To bring in bug fixes or changes from [littlefs-project/littlefs](https://github.com/littlefs-project/littlefs):

1. Create a branch: `git checkout -b upstream-sync/$(date +%Y%m%d)`
2. Generate the report: `just upstream-report` — review `.upstream-cache/pending-sync.md` for commits, changed C functions, and the full diff.
3. Generate the agent prompt: `just upstream-prompt` — copy the output.
4. Give the prompt to an agent (Cursor, Claude, etc.) or follow the steps manually. The prompt references `docs/rules.md` and instructs the agent to bump the tracking file and commit when done.
5. Verify: `just fci`
6. Push and open a PR: `git push -u origin HEAD && gh pr create`

The `scripts/upstream` script supports: `sync`, `log`, `diff`, `report`, `prompt`, `bump`. Run `scripts/upstream` with no args for usage.
