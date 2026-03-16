# ascend-tools

SDK, CLI, and MCP server for the Ascend REST API. Rust core with PyO3 Python bindings.

Repo: `ascend-io/ascend-tools`. Internal.

> `CLAUDE.md` is a symlink to this file (`AGENTS.md`). Edit `AGENTS.md` only.

## architecture

Four Rust crates, one PyO3 bridge. The core/mcp/cli crates share a Cargo workspace (`Cargo.toml` at repo root). Dependency chain is one-directional:

```
src/ascend_tools/
├── __init__.py              # re-exports Client, CLI entry point (main)
├── core.pyi                 # type stubs for the PyO3 module (IDE autocomplete)
├── py.typed                 # PEP 561 marker (package has inline types)
│
├── ascend-tools-core/         # Rust SDK crate (core library)
│   └── src/
│       ├── lib.rs           # pub exports
│       ├── auth.rs          # Ed25519 JWT signing, Cloud API token exchange, caching
│       ├── client.rs        # AscendClient — typed HTTP methods for /api/v1
│       ├── config.rs        # env var + CLI flag resolution
│       ├── error.rs         # public typed Error enum + Result alias for SDK consumers
│       └── models.rs        # Environment, Project, Runtime, Flow, FlowRun, FlowRunTrigger, filter structs
│
├── ascend-tools-mcp/          # MCP server crate (depends on ascend-tools-core)
│   └── src/
│       ├── lib.rs           # run_stdio() and run_http() entry points
│       ├── server.rs        # AscendMcpServer — 18 tools via rmcp #[tool_router]
│       └── params.rs        # typed parameter structs with JsonSchema for MCP tool schemas
│
├── ascend-tools-cli/          # Rust CLI crate (depends on ascend-tools-core, ascend-tools-mcp)
│   └── src/
│       ├── lib.rs           # pub fn run_cli(args) — testable entry point
│       ├── main.rs          # binary entry point
│       ├── cli.rs           # clap commands, table/json output, print_table helper
│       └── skill-cli.md     # SKILL.md template (embedded via include_str!, installed by `skill install`)
│
└── ascend-tools-py/           # PyO3 binding crate (cdylib, built by maturin)
    └── src/
        └── lib.rs           # exposes Client class + run_cli() to Python via pythonize (direct Rust→Python dict conversion)
```

The `-py` crate is **not** in the Cargo workspace (cdylib requires maturin). It's built exclusively by `maturin develop` and has its own Cargo.lock. The `-mcp` crate uses `rmcp` for the MCP protocol implementation.
Integration tests live under `ascend-tools-core/tests/` and `ascend-tools-cli/tests/`.

PyPI: `ascend-tools`. Crates.io: not yet published. Installed binary: `ascend-tools`.

## development

```bash
bin/build       # build Rust + Python (bin/build-rs, bin/build-py)
bin/check       # lint + test (bin/check-version, bin/check-rs, bin/check-py)
bin/format      # auto-format (bin/format-rs, bin/format-py)
bin/test        # run tests (bin/test-rs)
bin/install     # install locally (bin/install-rs, bin/install-py)
bin/bump-version  # bump version (--patch, --minor (default), --major)
bin/release       # tag + push release (runs check, validates GitHub/PyPI)
```

Cargo workspace is at the repo root:
`cargo fmt --all --check`, `cargo clippy --workspace -- -D warnings`, `cargo test --workspace`
Python checks: `ruff check .`, `ruff format --check .`, `ty check`

After code changes, always run `bin/check` before committing.

## authentication

The SDK/CLI authenticates via Ascend service accounts. The flow is handled transparently:

1. User provides service account ID + key (from Ascend UI → Settings → Users → Create Service Account)
2. SDK signs an Ed25519 JWT with the key
3. SDK exchanges the JWT at the Instance API (`POST /api/v1/auth/token`) for an instance access token
4. SDK uses the instance token as Bearer auth against the Instance API `/api/v1/*`
5. Token is cached and refreshed automatically before expiry

All SDK calls go through `/api/v1/` — no direct Cloud API calls.

### env vars

| Variable | Required | Description |
|----------|----------|-------------|
| `ASCEND_SERVICE_ACCOUNT_ID` | yes | Service account ID (`asc-sa-...`) |
| `ASCEND_SERVICE_ACCOUNT_KEY` | yes | Ed25519 private key (base64url, shown once at creation) |
| `ASCEND_INSTANCE_API_URL` | yes | Instance API URL (e.g. `https://api.instance.ascend.io`) |

That's it — 3 env vars. The SDK automatically discovers the JWT audience domain from the Instance API via `GET /api/v1/auth/config`.

The Python SDK reads these automatically — `ascend_tools.Client()` with no args works if env vars are set.

### local dev

```bash
export ASCEND_INSTANCE_API_URL="https://<workspace>-instance.api.local.ascend.dev"
```

## CLI reference

```
ascend-tools [-o text|json] [-V]

  workspace list [--environment <NAME>] [--project <NAME>]
  workspace get <TITLE>
  workspace create --title <TITLE> --environment <NAME> --project <NAME> --profile <NAME> --git-branch <BRANCH> [--git-branch-base, --size, --storage-size, --auto-snooze-timeout-minutes]
  workspace update <TITLE> [--title, --git-branch, --git-branch-base, --profile, --size, --storage-size, --auto-snooze-timeout-minutes]
  workspace pause <TITLE>
  workspace resume <TITLE>
  workspace delete <TITLE>

  deployment list [--environment <NAME>] [--project <NAME>]
  deployment get <TITLE>
  deployment create --title <TITLE> --environment <NAME> --project <NAME> --profile <NAME> --git-branch <BRANCH> [--git-branch-base, --size, --storage-size, --enable-automations]
  deployment update <TITLE> [--title, --git-branch, --git-branch-base, --profile, --size, --storage-size, --enable-automations]
  deployment pause-automations <TITLE>
  deployment resume-automations <TITLE>
  deployment delete <TITLE>

  flow list --workspace <TITLE> | --deployment <TITLE>
  flow run <FLOW_NAME> --workspace <TITLE> | --deployment <TITLE> [--spec '{}'] [--resume]
  flow list-runs --workspace <TITLE> | --deployment <TITLE> [--status, -f/--flow, --since, --until, --offset, --limit]
  flow get-run <RUN_NAME> --workspace <TITLE> | --deployment <TITLE>

  skill install --target <PATH> [--cli] [--python] [--mcp] [--all]

  mcp [--http] [--bind <ADDR>]
```

Default output is table format. Use `-o json` for machine-readable output.

`--environment` and `--project` accept friendly names (titles), not UUIDs. UUIDs still work for all commands via `--uuid` flag.

No subcommand prints help. Auth params can be passed as `--service-account-id`, `--service-account-key`, etc. or via env vars. Secret values are hidden in `--help` output.

## Python SDK reference

```python
from ascend_tools import Client

# All params optional — resolved from env vars if not provided
client = Client()

# Or explicit — only need the instance API URL
client = Client(
    service_account_id="asc-sa-...",
    service_account_key="...",
    instance_api_url="https://api.instance.ascend.io",
)

# Workspaces
client.list_workspaces()
client.list_workspaces(environment="Production", project="My Project")
client.get_workspace(title="My Workspace")
client.pause_workspace(title="My Workspace")
client.resume_workspace(title="My Workspace")
client.delete_workspace(title="My Workspace")

# Deployments
client.list_deployments()
client.get_deployment(title="My Deployment")
client.delete_deployment(title="My Deployment")

# Flows
client.list_flows(workspace="My Workspace")
client.run_flow(flow="sales", workspace="My Workspace")

# Flow runs
client.list_flow_runs(workspace="My Workspace", status="running")
client.list_flow_runs(deployment="My Deployment", flow="sales", limit=10)
client.get_flow_run(name="fr-...", workspace="My Workspace")
```

All methods return `dict` or `list[dict]`. All parameters are keyword-only.

## MCP server

The `mcp` subcommand starts an MCP (Model Context Protocol) server, exposing AscendClient methods as tools for AI assistants (Claude Code, Claude Desktop, Cursor, etc.).

### transports

- **stdio** (default): `ascend-tools mcp` — communicates over stdin/stdout. Used by Claude Code and most MCP clients.
- **HTTP**: `ascend-tools mcp --http [--bind 127.0.0.1:8000]` — Streamable HTTP on `/mcp`. Used for remote/shared deployments.

### tools

| Tool | Description |
|------|-------------|
| `list_workspaces` | List workspaces with optional filters (environment, project) |
| `get_workspace` | Get a workspace by title |
| `create_workspace` | Create a new workspace |
| `update_workspace` | Update an existing workspace |
| `pause_workspace` | Pause a running workspace |
| `resume_workspace` | Resume a paused workspace |
| `delete_workspace` | Delete a workspace |
| `list_deployments` | List deployments with optional filters (environment, project) |
| `get_deployment` | Get a deployment by title |
| `create_deployment` | Create a new deployment |
| `update_deployment` | Update an existing deployment |
| `pause_deployment_automations` | Pause automations on a deployment |
| `resume_deployment_automations` | Resume automations on a deployment |
| `delete_deployment` | Delete a deployment |
| `list_flows` | List flows in a workspace or deployment |
| `run_flow` | Trigger a flow run with typed spec (resume, full_refresh, components, parameters, etc.) |
| `list_flow_runs` | List flow runs with filters (status, flow, since, until, offset, limit) |
| `get_flow_run` | Get a flow run by name |

### usage with Claude Code

**Remote (recommended)** — copy `ASCEND_MCP_URL` from Settings > Instance > MCP Server:

```bash
claude mcp add --transport http ascend-tools $ASCEND_MCP_URL
```

Auth is handled automatically via OAuth (browser login). No service account or env vars needed.

**Local (alternative)** — for offline or custom setups:

```bash
claude mcp add --transport stdio ascend-tools-dev -- uvx ascend-tools mcp
```

Auth env vars (`ASCEND_SERVICE_ACCOUNT_ID`, `ASCEND_SERVICE_ACCOUNT_KEY`, `ASCEND_INSTANCE_API_URL`) are inherited from the shell.
If Claude is launched without your shell env, set them explicitly:

```bash
claude mcp add --transport stdio ascend-tools-dev \
  -e ASCEND_SERVICE_ACCOUNT_ID="$ASCEND_SERVICE_ACCOUNT_ID" \
  -e ASCEND_SERVICE_ACCOUNT_KEY="$ASCEND_SERVICE_ACCOUNT_KEY" \
  -e ASCEND_INSTANCE_API_URL="$ASCEND_INSTANCE_API_URL" \
  -- uvx ascend-tools mcp
```

```bash
claude mcp remove ascend-tools
claude mcp remove ascend-tools-dev
```

### usage with Codex CLI

```bash
codex mcp add ascend-tools-dev -- uvx ascend-tools mcp
```

If Codex is launched without your shell env, set them explicitly:

```bash
codex mcp add \
  --env "ASCEND_SERVICE_ACCOUNT_ID=$ASCEND_SERVICE_ACCOUNT_ID" \
  --env "ASCEND_SERVICE_ACCOUNT_KEY=$ASCEND_SERVICE_ACCOUNT_KEY" \
  --env "ASCEND_INSTANCE_API_URL=$ASCEND_INSTANCE_API_URL" \
  ascend-tools-dev -- uvx ascend-tools mcp
```

```bash
codex mcp get ascend-tools-dev --json
```

```bash
codex mcp list
```

```bash
codex mcp remove ascend-tools
codex mcp remove ascend-tools-dev
```

Auth env vars (`ASCEND_SERVICE_ACCOUNT_ID`, `ASCEND_SERVICE_ACCOUNT_KEY`, `ASCEND_INSTANCE_API_URL`) are inherited from the shell.
If stale behavior appears after code updates, run one refresh manually:

```bash
uvx --refresh ascend-tools --version
```

If Codex MCP startup fails with `connection closed: initialize response`, refresh once and re-add:

```bash
uvx --refresh ascend-tools --version
```

### architecture notes

- Uses `rmcp` SDK for MCP protocol
- `AscendClient` is sync (ureq); tools use `tokio::task::spawn_blocking` to bridge to async
- `AscendClient` wrapped in `Arc` for the `Clone` requirement (contains `Mutex` in Auth)
- Tracing writes to stderr only (stdout is the MCP protocol channel for stdio transport)
- `reset_sigint()` clears Python's SIGINT handler so Ctrl+C works when running through PyO3
- HTTP mode creates a fresh `AscendClient` per session via `StreamableHttpService` factory

## backend API

The SDK/CLI calls the Instance API's `/api/v1/` endpoints, defined in `ascend-backend/src/ascend_backend/instance/api/v1/`. These return plain JSON (not JSON:API).

| Endpoint | Method | Description |
|----------|--------|-------------|
| `/api/v1/auth/config` | GET | Get JWT audience domain for SA authentication |
| `/api/v1/auth/token` | POST | Exchange SA JWT for instance token (no pre-existing token required) |
| `/api/v1/runtimes` | GET | List runtimes (filters: id, kind, title, project_uuid, environment_uuid) |
| `/api/v1/runtimes` | POST | Create a runtime |
| `/api/v1/runtimes/{uuid}` | GET | Get a runtime |
| `/api/v1/runtimes/{uuid}` | PATCH | Update a runtime |
| `/api/v1/runtimes/{uuid}` | DELETE | Delete a runtime |
| `/api/v1/runtimes/{uuid}/flows` | GET | List flows in a runtime |
| `/api/v1/runtimes/{uuid}/flows/{name}:run` | POST | Trigger a flow run |
| `/api/v1/flow-runs` | GET | List flow runs (requires runtime_uuid, filters: status, flow, since, until) |
| `/api/v1/flow-runs/{name}` | GET | Get a flow run (requires runtime_uuid query param) |

## conventions

- Rust stable toolchain (edition 2024, requires 1.85+)
- Commits and PR titles use Conventional Commits (`type(scope): summary` when scoped, otherwise `type: summary`); use `refactor:` for internal quality improvements without behavior changes
- `ascend-tools-core` exposes typed errors via `ascend_tools::Error` / `ascend_tools::Result` (`thiserror`); `anyhow` is kept at app boundaries (CLI/MCP)
- API methods return typed structs in Rust (`serde_json::Value` used only for dynamic fields like `FlowRun.error`)
- HTTP response handling reads body as text first, then tries JSON parse (robust against non-JSON errors) and maps failures to typed error variants
- HTTP client uses `ureq` (synchronous) with platform TLS verifier (trusts system CA store)
- `http_status_as_error(false)` — we handle HTTP status codes ourselves, not ureq
- Token caching holds the mutex during refresh to prevent thundering herd
- Core library code avoids panic paths in runtime operations (no `expect`/`unwrap` in non-test control flow)
- Clap args for secrets use `hide_env_values = true` (SA ID, SA key)
- PyO3 binding uses `pythonize` to convert Rust structs directly to Python dicts (no JSON string intermediary)
- CLI prints tables by default, JSON with `-o json`; empty results print "No results." to stderr
- MCP tool parameters use `schemars` `JsonSchema` derive for automatic JSON Schema generation; doc comments on fields become schema descriptions
- MCP `FlowRunSpec` uses `#[serde(flatten)]` with a catch-all map for forward compatibility with new backend fields
- PyO3 `run_cli()` uses `py.detach()` to release the GIL during long-running Rust calls (MCP server)
- Test coverage includes integration tests with mock servers (`mockito`) for core HTTP/auth behavior, MCP tool behavior, and CLI output regressions
- When adding or changing CLI commands, update `src/ascend_tools/ascend-tools-cli/src/skill-cli.md` to keep the skill in sync

## related repos

- **ascend-backend** — Instance API v1 endpoints (`src/ascend_backend/instance/api/v1/`), Auth0 service account fixes (`src/ascend_backend/cloud/authn/manager.py`), cache invalidation on SA create/delete
- **ascend-ui** — Service account creation dialog with env var display (`src/lib/components/forms/CreateServiceAccountDialog.svelte`)
