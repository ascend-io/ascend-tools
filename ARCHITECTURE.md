# Architecture

CLI, SDK, and MCP server for the Ascend Instance web API. Rust core with PyO3 Python bindings and napi-rs JavaScript bindings.

## Design decisions

### One Rust core, many interfaces

The central bet: write the SDK logic once in Rust (`ascend-tools-core`), then expose it to every target through thin binding crates. This gives us a single implementation of auth, HTTP, error handling, and models — no drift between languages.

The interfaces are:

| Interface | Crate | Binding | Package |
|-----------|-------|---------|---------|
| Rust SDK | `ascend-tools-core` | (native) | `ascend-tools-core` on crates.io |
| CLI | `ascend-tools-cli` | (native) | `ascend-tools-cli` on crates.io |
| MCP server | `ascend-tools-mcp` | (native) | `ascend-tools-mcp` on crates.io |
| TUI | `ascend-tools-tui` | (native) | `ascend-tools-tui` on crates.io |
| Python SDK | `ascend-tools-py` | PyO3 | `ascend-tools` on PyPI |
| JavaScript SDK | `ascend-tools-js` | napi-rs | `ascend-tools` on npm |

Python and JS users get a `Client` class with identical semantics. Both also expose `run_cli()` so the CLI works via `uvx ascend-tools` / `npx ascend-tools` without a Rust toolchain.

### Synchronous HTTP client

`AscendClient` uses `ureq` (synchronous, blocking I/O). This was deliberate:

- The SDK makes sequential request-response calls — no concurrency benefit from async
- Sync code is simpler to reason about, test, and bind to Python/JS
- The MCP server bridges to async via `tokio::task::spawn_blocking`
- The TUI uses `std::thread::scope` to stream without `Arc`

### Typed errors in the core, `anyhow` at app boundaries

`ascend-tools-core` exposes `ascend_tools::Error` (via `thiserror`) with variants for auth failures, HTTP errors, parse errors, etc. Consumers can match on specific error conditions.

The CLI and MCP crates use `anyhow` for top-level error reporting — they don't need to expose a typed error API, just print useful messages.

Core library code avoids panic paths in runtime operations (no `expect`/`unwrap` in non-test control flow).

### HTTP response handling

Responses are read as text first, then JSON-parsed. This is robust against non-JSON error bodies (e.g., HTML from a proxy). `http_status_as_error(false)` means we handle status codes ourselves, not ureq, so we can extract error details from the response body. Failures map to typed error variants.

### Platform TLS

`ureq` with `rustls-platform-verifier` trusts the system CA store. No bundled root certificates — works correctly behind corporate proxies with custom CAs.

### Data conversion at the boundary

- **Python**: PyO3 + `pythonize` converts Rust structs directly to Python dicts. No JSON string intermediary — one serialization step, not two.
- **JavaScript**: napi-rs converts Rust structs to JS objects. Async methods use the libuv thread pool (blocking Rust calls don't block the Node event loop).

### Token caching

Auth holds the mutex during token refresh to prevent thundering herd. One thread refreshes, others wait and reuse the result.

### Forward-compatible flow specs

`FlowRunSpec` uses `#[serde(flatten)]` with a catch-all `HashMap` so new backend fields pass through without SDK changes.

## Crate structure

Six Rust crates, two language bridges. The core/tui/mcp/cli crates share a Cargo workspace (`Cargo.toml` at repo root). Dependency chain is one-directional:

```
ascend-tools-cli
├── ascend-tools-core
├── ascend-tools-mcp → ascend-tools-core
└── ascend-tools-tui → ascend-tools-core

ascend-tools-py  → ascend-tools-core (cdylib, separate Cargo.lock)
ascend-tools-js  → ascend-tools-core (cdylib, separate Cargo.lock)
```

The `-py` and `-js` crates are **not** in the Cargo workspace — cdylib targets require separate build tooling (maturin and @napi-rs/cli respectively). Each has its own `Cargo.lock`.

### Source layout

```
py/ascend_tools/            # Python package (PyO3 bindings land here)
├── __init__.py             # re-exports Client, CLI entry point (main)
├── core.pyi                # type stubs for the PyO3 module (IDE autocomplete)
└── py.typed                # PEP 561 marker (package has inline types)

crates/
├── ascend-tools-core/        # Rust SDK crate (core library)
│   └── src/
│       ├── lib.rs           # pub exports
│       ├── auth.rs          # Ed25519 JWT signing, Cloud API token exchange, caching
│       ├── client.rs        # AscendClient — typed HTTP methods for /api/v1
│       ├── config.rs        # env var + CLI flag resolution
│       ├── error.rs         # public typed Error enum + Result alias for SDK consumers
│       ├── models.rs        # Environment, Project, Runtime, Flow, FlowRun, FlowRunTrigger, filter structs
│       └── sse.rs           # minimal SSE (Server-Sent Events) line parser for Otto streaming
│
├── ascend-tools-mcp/         # MCP server crate (depends on ascend-tools-core)
│   └── src/
│       ├── lib.rs           # run_stdio() and run_http() entry points
│       ├── server.rs        # AscendMcpServer — 25 tools via rmcp #[tool_router]
│       └── params.rs        # typed parameter structs with JsonSchema for MCP tool schemas
│
├── ascend-tools-tui/         # Interactive TUI crate (depends on ascend-tools-core)
│   └── src/
│       └── lib.rs           # run_tui() — full-screen ratatui chat interface for Otto
│
├── ascend-tools-cli/         # Rust CLI crate (depends on core, mcp, tui)
│   └── src/
│       ├── lib.rs           # pub fn run_cli(args) — testable entry point
│       ├── main.rs          # binary entry point
│       ├── cli.rs           # clap commands, table/json output, print_table helper
│       ├── skill-cli.md     # SKILL.md templates (embedded via include_str!, installed by `skill install`)
│       ├── skill-py.md
│       ├── skill-js.md
│       ├── skill-rs.md
│       └── skill-mcp.md
│
├── ascend-tools-py/          # PyO3 binding crate (cdylib, built by maturin)
│   └── src/
│       └── lib.rs           # exposes Client class + run_cli() to Python via pythonize
│
└── ascend-tools-js/          # napi-rs binding crate (cdylib, built by @napi-rs/cli)
    ├── cli.js               # CLI entry point for `npx ascend-tools`
    └── src/
        └── lib.rs           # exposes Client class + run_cli() to Node.js via napi-rs
```

### Packaging

| Registry | Package | What ships |
|----------|---------|------------|
| PyPI | `ascend-tools` | Python wheel with compiled `.so`/`.dylib` (maturin) — includes CLI via entry point |
| npm | `ascend-tools` | Prebuilt native addon per platform (@napi-rs/cli) — includes CLI via `bin` |
| crates.io | `ascend-tools-core` | Rust SDK library |
| crates.io | `ascend-tools-cli` | Rust binary (`ascend-tools`) |
| crates.io | `ascend-tools-mcp` | MCP server library |
| crates.io | `ascend-tools-tui` | TUI library |

The installed binary is always `ascend-tools` regardless of install method (`cargo install`, `uvx`, `npx`).

## Authentication

The SDK authenticates via Ascend service accounts. The flow is handled transparently:

1. User provides service account ID + key (from Ascend UI → Settings → Users → Create Service Account)
2. SDK signs an Ed25519 JWT with the key
3. SDK exchanges the JWT at the Instance API (`POST /api/v1/auth/token`) for an instance access token
4. SDK uses the instance token as Bearer auth against the Instance API `/api/v1/*`
5. Token is cached and refreshed automatically before expiry

All SDK calls go through `/api/v1/` — no direct Cloud API calls. The SDK automatically discovers the JWT audience domain from the Instance API via `GET /api/v1/auth/config`.

## Backend API surface

The SDK calls the Instance API's `/api/v1/` endpoints, defined in `ascend-backend/src/ascend_backend/instance/api/v1/`. These return plain JSON (not JSON:API).

| Endpoint | Method | Description |
|----------|--------|-------------|
| `/api/v1/auth/config` | GET | Get JWT audience domain for SA authentication |
| `/api/v1/auth/token` | POST | Exchange SA JWT for instance token (no pre-existing token required) |
| `/api/v1/runtimes` | GET | List runtimes (filters: id, kind, title, project_uuid, environment_uuid) |
| `/api/v1/runtimes` | POST | Create a runtime |
| `/api/v1/runtimes/{uuid}` | GET | Get a runtime |
| `/api/v1/runtimes/{uuid}` | PATCH | Update a runtime |
| `/api/v1/runtimes/{uuid}` | DELETE | Delete a runtime |
| `/api/v1/environments` | GET | List environments (filter: title) |
| `/api/v1/projects` | GET | List projects (filter: title) |
| `/api/v1/profiles` | GET | List profiles (filters: runtime_uuid, project, branch) |
| `/api/v1/runtimes/{uuid}/flows` | GET | List flows in a runtime |
| `/api/v1/runtimes/{uuid}/flows/{name}:run` | POST | Trigger a flow run |
| `/api/v1/flow-runs` | GET | List flow runs (requires runtime_uuid, filters: status, flow, since, until) |
| `/api/v1/flow-runs/{name}` | GET | Get a flow run (requires runtime_uuid query param) |

## MCP server architecture

- Uses `rmcp` SDK for the MCP protocol
- `AscendClient` is sync; tools use `tokio::task::spawn_blocking` to bridge to async
- `AscendClient` wrapped in `Arc` for `Clone` (contains `Mutex` in Auth)
- Tracing writes to stderr only (stdout is the MCP protocol channel for stdio transport)
- `reset_sigint()` clears Python's SIGINT handler so Ctrl+C works when running through PyO3
- HTTP mode creates a fresh `AscendClient` per session via `StreamableHttpService` factory
- Tool parameters use `schemars` `JsonSchema` derive for automatic schema generation; doc comments on fields become schema descriptions

## TUI architecture

- `ratatui` + `crossterm`; single public entry point `run_tui()`
- Uses `std::thread::scope` for streaming (borrows `&AscendClient` without `Arc`)
- Vi mode by default; history persisted to `~/.ascend-tools/history`
- Colors are defined as named constants at the top of `lib.rs` — no inline color values

## Related repos

- **ascend-backend** — Instance API v1 endpoints (`src/ascend_backend/instance/api/v1/`), Auth0 service account fixes (`src/ascend_backend/cloud/authn/manager.py`), cache invalidation on SA create/delete
- **ascend-ui** — Service account creation dialog with env var display (`src/lib/components/forms/CreateServiceAccountDialog.svelte`)
