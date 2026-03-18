# crates

Rust crates for ascend-tools. The four workspace crates (`core`, `mcp`, `tui`, `cli`) share the root `Cargo.toml` workspace. The two binding crates (`py`, `js`) are standalone cdylibs with their own `Cargo.lock`.

| Crate | Type | Description |
|-------|------|-------------|
| `ascend-tools-core` | lib | SDK — typed HTTP client, auth, models, SSE parser |
| `ascend-tools-mcp` | lib | MCP server (rmcp) — exposes SDK methods as MCP tools |
| `ascend-tools-tui` | lib | Interactive TUI (ratatui) — full-screen Otto chat |
| `ascend-tools-cli` | bin | CLI (clap) — `ascend-tools` binary, depends on all above |
| `ascend-tools-py` | cdylib | PyO3 bindings — built by maturin, exposes `ascend_tools.core` |
| `ascend-tools-js` | cdylib | napi-rs bindings — built by `@napi-rs/cli`, exposes `@ascend-io/ascend-tools` |
