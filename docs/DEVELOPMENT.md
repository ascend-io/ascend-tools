# Development

Guide for contributors working on ascend-tools.

## Prerequisites

- **Rust** stable toolchain (1.85+, edition 2024)
- **Python** 3.11+ (for PyO3 bindings and linting)
- **Node.js** 24+ (for napi-rs bindings and JS tests)
- **uv** (Python package manager)

## Setup

Clone and run the setup script:

```bash
git clone https://github.com/ascend-io/ascend-tools.git
cd ascend-tools
bin/setup
```

`bin/setup` installs Rust (via rustup) and uv if missing, verifies all tools are on PATH, and installs JS dependencies. If it errors, follow the printed instructions (usually just sourcing a shell env) and re-run.

## Build

```bash
bin/build           # build everything (Rust + Python + JS)
bin/build-rs        # Rust workspace only (cargo build --workspace)
bin/build-py        # Python wheel (maturin develop)
bin/build-js        # JS native module (napi build)
```

`bin/build-rs` accepts extra args (e.g., `bin/build-rs --release`).

## Test

```bash
bin/check           # full CI suite: version check + lint + test (Rust, Python, JS)
bin/test            # Rust tests only (cargo test --workspace)
```

`bin/check` is what CI runs. Always run it before committing.

### What `bin/check` runs

| Step | Command |
|------|---------|
| `bin/check-version` | Verify version is consistent across all Cargo.toml, pyproject.toml, package.json |
| `bin/check-rs` | `cargo fmt --all --check`, `cargo clippy --workspace -- -D warnings`, `cargo test --workspace` |
| `bin/check-py` | `ruff check .`, `ruff format --check .`, `ty check` |
| `bin/check-js` | `npm run build`, `npm test` (ava) |

## Format

```bash
bin/format          # auto-format everything
bin/format-rs       # cargo fmt --all
bin/format-py       # ruff format .
```

## Install locally

```bash
bin/install         # install Rust binary + Python package locally
bin/install-rs      # cargo install --path crates/ascend-tools-cli
bin/install-py      # uv tool install .
```

## Architecture

Six Rust crates, two language bridges:

```
ascend-tools-core  →  ascend-tools-mcp  →  ascend-tools-cli
                   →  ascend-tools-tui  →
                   →  ascend-tools-py  (PyO3, cdylib)
                   →  ascend-tools-js  (napi-rs, cdylib)
```

The four workspace crates (`core`, `mcp`, `tui`, `cli`) share the root `Cargo.toml` workspace. The two binding crates (`py`, `js`) are standalone cdylibs with their own `Cargo.lock` (required by maturin/napi-rs build tooling).

| Crate | Published to | Description |
|-------|-------------|-------------|
| `ascend-tools-core` | [crates.io](https://crates.io/crates/ascend-tools-core) | SDK: typed HTTP client, auth, models, SSE |
| `ascend-tools-mcp` | [crates.io](https://crates.io/crates/ascend-tools-mcp) | MCP server (rmcp) |
| `ascend-tools-tui` | [crates.io](https://crates.io/crates/ascend-tools-tui) | Interactive TUI (ratatui) |
| `ascend-tools-cli` | [crates.io](https://crates.io/crates/ascend-tools-cli) | CLI binary (clap) |
| `ascend-tools-py` | [PyPI](https://pypi.org/project/ascend-tools/) | PyO3 bindings (maturin) |
| `ascend-tools-js` | [npm](https://www.npmjs.com/package/ascend-tools) | napi-rs bindings (@napi-rs/cli) |

## Version management

All six crates and both language packages share one version. To bump:

```bash
bin/bump-version              # minor bump (default)
bin/bump-version --patch      # patch bump
bin/bump-version --major      # major bump
```

This updates all `Cargo.toml` files (including inter-crate dependency versions), `pyproject.toml`, `package.json`, and regenerates all lock files.

## Release process

Releases are triggered by git tags. `bin/release` handles the full flow:

```bash
bin/release
```

Pre-flight checks:
- Working tree is clean
- HEAD matches `origin/main`
- `bin/check` passes
- Tag doesn't exist on GitHub
- Version doesn't exist on PyPI, npm, or crates.io

On tag push, four GitHub Actions workflows run in parallel:
- `release.yml`: builds standalone binaries, creates GitHub Release
- `release-python.yml`: builds wheels (4 platforms), publishes to PyPI (trusted publisher)
- `release-javascript.yml`: builds native modules (4 platforms), publishes to npm (trusted publisher)
- `release-rust.yml`: publishes 4 crates to crates.io (trusted publisher)

All registry publishing uses OIDC trusted publishers — no long-lived tokens.

## Testing against a live instance

Integration tests run via GitHub Actions (see `.github/workflows/integration.yml`). They require `ASCEND_SERVICE_ACCOUNT_ID`, `ASCEND_SERVICE_ACCOUNT_KEY`, and `ASCEND_INSTANCE_API_URL` environment variables pointing to a test instance.

## Adding a new CLI command

1. Add the clap subcommand in `crates/ascend-tools-cli/src/`
2. Wire it into `cli.rs`
3. Update `skill-cli.md` to keep the skill template in sync
4. Run `bin/check`

## Adding a new MCP tool

1. Add the method to `AscendClient` in `crates/ascend-tools-core/src/client.rs`
2. Add the tool to `AscendMcpServer` in `crates/ascend-tools-mcp/src/server.rs`
3. Add parameter structs to `crates/ascend-tools-mcp/src/params.rs`
4. Update `skill-mcp.md`
5. Run `bin/check`

## Conventions

- Commits use [Conventional Commits](https://www.conventionalcommits.org/) (`feat:`, `fix:`, `refactor:`, etc.)
- Core library avoids panics in runtime paths (no `unwrap`/`expect` outside tests)
- HTTP client is sync (`ureq`); async adapters live at boundaries (MCP uses `spawn_blocking`, JS uses napi `AsyncTask`)
- PyO3 uses `pythonize` for direct Rust-to-Python dict conversion (no JSON intermediary)
- napi-rs uses `serde-json` feature for direct Rust-to-JS object conversion
- MCP parameters use `schemars` for automatic JSON Schema generation
- CLI prints tables by default, JSON with `-o json`
