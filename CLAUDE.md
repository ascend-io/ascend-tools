# ascend-ops

SDK and CLI for the Ascend REST API. Rust core with PyO3 Python bindings.

Repo: `ascend-io/ascend-ops`. Internal.

## architecture

Three Rust crates, one PyO3 bridge, thin Python wrapper. Dependency chain is one-directional:

```
src/ascend_ops/
├── __init__.py              # re-exports Client from client.py
├── client.py                # Python SDK: Client class wrapping Rust, returns dicts
├── cli.py                   # CLI entry point: calls core.run(sys.argv)
│
├── ascend-ops/              # Rust SDK crate (core library)
│   └── src/
│       ├── lib.rs           # pub exports
│       ├── auth.rs          # Ed25519 JWT signing, Cloud API token exchange, caching
│       ├── client.rs        # AscendClient — typed HTTP methods for /api/v1
│       ├── config.rs        # env var + CLI flag resolution
│       └── models.rs        # Runtime, Flow, FlowRun, FlowRunTrigger, filter structs
│
├── ascend-ops-cli/          # Rust CLI crate (depends on ascend-ops)
│   └── src/
│       ├── lib.rs           # pub fn run(args) — testable entry point
│       ├── main.rs          # binary entry point
│       └── cli.rs           # clap commands, table/json output, print_table helper
│
└── ascend-ops-py/           # PyO3 binding crate (cdylib, built by maturin)
    └── src/
        └── lib.rs           # exposes Client class + run() to Python as ascend_ops.core
```

The `-py` crate is **not** in a Cargo workspace (cdylib requires maturin). It's built exclusively by `maturin develop`.

PyPI: `ascend-ops`. Crates.io: `ascend-ops` (SDK), `ascend-ops-cli` (binary). Installed binary: `ascend-ops`.

## development

```bash
bin/build       # build Rust + Python (bin/build-rs, bin/build-py)
bin/check       # lint + test (bin/check-rs, bin/check-py)
bin/format      # auto-format (bin/format-rs, bin/format-py)
bin/test        # run tests (bin/test-rs, bin/test-py)
bin/install     # install locally (bin/install-rs, bin/install-py)
```

Rust checks: `cargo fmt --check`, `cargo clippy -- -D warnings`, `cargo test`
Python checks: `ruff check .`, `ruff format --check .`

After code changes, always run `bin/check` before committing.

## authentication

The SDK/CLI authenticates via Ascend service accounts. The flow is handled transparently:

1. User provides service account ID + key (from Ascend UI → Settings → Users → Create Service Account)
2. SDK signs an Ed25519 JWT with the key
3. SDK exchanges the JWT at the Cloud API (`POST /auth/token`) for an instance access token
4. SDK uses the instance token as Bearer auth against the Instance API `/api/v1/*`
5. Token is cached and refreshed automatically before expiry

### env vars

| Variable | Required | Description |
|----------|----------|-------------|
| `ASCEND_SERVICE_ACCOUNT_ID` | yes | Service account ID (`asc-sa-...`) |
| `ASCEND_SERVICE_ACCOUNT_KEY` | yes | Ed25519 private key (base64url, shown once at creation) |
| `ASCEND_INSTANCE_API_URL` | yes | Instance API URL (e.g. `https://api.instance.ascend.io`) |
| `ASCEND_CLOUD_API_URL` | no | Cloud API URL (default: `https://api.ascend.io`) |
| `ASCEND_CLOUD_API_DOMAIN` | no | Override JWT audience domain. Only needed for local dev where the proxy domain differs from the Cloud API's internal `CLOUD_API_DOMAIN`. |

The Python SDK reads these automatically — `ascend_ops.Client()` with no args works if env vars are set.

### local dev

When testing against a local ASE workspace, the Cloud API's internal `CLOUD_API_DOMAIN` defaults to `api.app.local.ascend.dev` but the proxy routes via `<workspace>-api.app.local.ascend.dev`. Set the override:

```bash
export ASCEND_CLOUD_API_URL="https://<workspace>-api.app.local.ascend.dev"
export ASCEND_CLOUD_API_DOMAIN="api.app.local.ascend.dev"
export ASCEND_INSTANCE_API_URL="https://<workspace>-instance.api.local.ascend.dev"
```

## CLI reference

```
ascend-ops [-o text|json] [-V]

  runtime list [--id, --kind, --project-uuid, --environment-uuid]
  runtime get <UUID>

  flow list -r/--runtime <UUID>
  flow run <FLOW_NAME> -r/--runtime <UUID> [--spec '{}']
  flow backfill <FLOW_NAME> -r/--runtime <UUID> [--spec '{}']
  flow list-runs -r/--runtime <UUID> [--status, --flow]
  flow get-run <RUN_NAME> -r/--runtime <UUID>
```

Default output is table format. Use `-o json` for machine-readable output.

No subcommand prints help. Auth params can be passed as `--service-account-id`, `--service-account-key`, etc. or via env vars. Secret values are hidden in `--help` output.

## Python SDK reference

```python
from ascend_ops import Client

# All params optional — resolved from env vars if not provided
client = Client()

# Or explicit
client = Client(
    service_account_id="asc-sa-...",
    service_account_key="...",
    instance_api_url="https://api.instance.ascend.io",
)

# Runtimes
client.list_runtimes()
client.list_runtimes(kind="deployment")
client.get_runtime(uuid="...")

# Flows
client.list_flows(runtime_uuid="...")
client.run_flow(runtime_uuid="...", flow_name="sales")
client.backfill_flow(runtime_uuid="...", flow_name="sales")

# Flow runs
client.list_flow_runs(runtime_uuid="...", status="running")
client.list_flow_runs(runtime_uuid="...", flow="sales", limit=10)
client.get_flow_run(runtime_uuid="...", name="fr-...")
```

All methods return `dict` or `list[dict]`. All parameters are keyword-only.

## backend API

The SDK/CLI calls the Instance API's `/api/v1/` endpoints, defined in `ascend-backend/src/ascend_backend/instance/api/v1/`. These return plain JSON (not JSON:API).

| Endpoint | Method | Description |
|----------|--------|-------------|
| `/api/v1/runtimes` | GET | List runtimes (filters: id, kind, project_uuid, environment_uuid) |
| `/api/v1/runtimes/{uuid}` | GET | Get a runtime |
| `/api/v1/runtimes/{uuid}/flows` | GET | List flows in a runtime |
| `/api/v1/runtimes/{uuid}/flows/{name}:run` | POST | Trigger a flow run |
| `/api/v1/runtimes/{uuid}/flows/{name}:backfill` | POST | Trigger a backfill |
| `/api/v1/flow-runs` | GET | List flow runs (requires runtime_uuid, filters: status, flow, since, until) |
| `/api/v1/flow-runs/{name}` | GET | Get a flow run (requires runtime_uuid query param) |

## conventions

- Rust stable toolchain (edition 2024, requires 1.85+)
- API methods return typed structs in Rust, `serde_json::Value` is not used for responses
- `handle_response()` reads body as text first, then tries JSON parse (robust against non-JSON errors)
- HTTP client uses `ureq` (synchronous) with platform TLS verifier (trusts system CA store)
- `http_status_as_error(false)` — we handle HTTP status codes ourselves, not ureq
- Token caching holds the mutex during refresh to prevent thundering herd
- All clap args with env vars use `hide_env_values = true` for secrets
- Python wrapper deserializes JSON from Rust into plain dicts (no custom Python types)
- CLI prints tables by default, JSON with `-o json`; empty results print "No results." to stderr

## related repos

- **ascend-backend** — Instance API v1 endpoints (`src/ascend_backend/instance/api/v1/`), Auth0 service account fixes (`src/ascend_backend/cloud/authn/manager.py`), cache invalidation on SA create/delete
- **ascend-ui** — Service account creation dialog with env var display (`src/lib/components/forms/CreateServiceAccountDialog.svelte`)
