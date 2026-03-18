# bin

Development scripts. All are bash, executable, and run from any directory.

| Script | Description |
|--------|-------------|
| `build` | Build everything (`build-rs`, `build-py`, `build-js`) |
| `build-rs` | `cargo build --workspace` |
| `build-py` | `maturin develop` |
| `build-js` | `napi build` |
| `check` | Lint + test everything (`check-version`, `check-rs`, `check-py`, `check-js`) |
| `check-rs` | `cargo fmt --check`, `cargo clippy`, `cargo test` |
| `check-py` | `ruff check`, `ruff format --check`, `ty check` |
| `check-js` | `npm install`, `npm run build`, `npm test` |
| `check-version` | Verify all crates/packages have the same version |
| `format` | Auto-format (`format-rs`, `format-py`) |
| `format-rs` | `cargo fmt --all` |
| `format-py` | `ruff format`, `ruff check --fix` |
| `test` | Run tests (`test-rs`) |
| `test-rs` | `cargo test --workspace` |
| `install` | Install locally (`install-rs`, `install-py`) |
| `install-rs` | `cargo install --path crates/ascend-tools-cli` |
| `install-py` | `maturin develop` |
| `setup` | One-time setup (install rustup, uv, JS deps) |
| `bump-version` | Bump version across all crates (`--patch`, `--minor`, `--major`) |
| `release` | Tag + push a release (runs check, validates GitHub/PyPI) |
