# ascend-tools

CLI, SDK, and MCP server for the Ascend Instance web API. Rust core with PyO3 Python bindings and napi-rs JavaScript bindings.

Repo: `ascend-io/ascend-tools`. Internal.

> `CLAUDE.md` is a symlink to this file (`AGENTS.md`). Edit `AGENTS.md` only.

For crate structure, design decisions, packaging, and backend API surface, see @ARCHITECTURE.md.

## development

```bash
bin/build       # build Rust + Python + JS (bin/build-rs, bin/build-py, bin/build-js)
bin/check       # lint + test (bin/check-version, bin/check-rs, bin/check-py, bin/check-js)
bin/format      # auto-format (bin/format-rs, bin/format-py)
bin/test        # run tests (bin/test-rs)
bin/install     # install locally (bin/install-rs, bin/install-py)
bin/bump-version  # bump version (--patch, --minor (default), --major)
bin/release       # tag + push release (runs check, validates GitHub/PyPI/npm)
```

Cargo workspace is at the repo root:
`cargo fmt --all --check`, `cargo clippy --workspace -- -D warnings`, `cargo test --workspace`
Python checks: `ruff check .`, `ruff format --check .`, `ty check`

After code changes, always run `bin/check` before committing.

## authentication

### instance config (recommended)

Configure named instances in `~/.ascend-tools/config.toml`:

```bash
ascend-tools instance add default \
  --service-account-id "asc-sa-..." \
  --instance-api-url "https://api.instance.ascend.io" \
  --service-account-key-env ASCEND_SERVICE_ACCOUNT_KEY
export ASCEND_SERVICE_ACCOUNT_KEY="..."
```

Config file format (`~/.ascend-tools/config.toml`):

```toml
default_instance = "production"   # optional, defaults to "default"

[default]
service_account_id = "asc-sa-abc123"
instance_api_url = "https://api.myinstance.ascend.io"
service_account_key_env = "ASCEND_SERVICE_ACCOUNT_KEY"

[staging]
service_account_id = "asc-sa-def456"
instance_api_url = "https://api.staging.ascend.io"
service_account_key_env = "ASCEND_STAGING_KEY"
```

`service_account_key_env` stores the env var **name** (not the secret). The tool reads that env var at runtime.

Switch instances: `--instance <name>` flag or `ASCEND_INSTANCE` env var.

### environment variables

Three env vars work as a fallback (backward compatible):

| Variable | Description |
|----------|-------------|
| `ASCEND_SERVICE_ACCOUNT_ID` | Service account ID (`asc-sa-...`) |
| `ASCEND_SERVICE_ACCOUNT_KEY` | Ed25519 private key (base64url, shown once at creation) |
| `ASCEND_INSTANCE_API_URL` | Instance API URL (e.g. `https://api.instance.ascend.io`) |

All three SDKs read these automatically — `Config::from_env()` (Rust), `ascend_tools.Client()` (Python), `new Client()` (JavaScript).

### resolution order

1. CLI flags (`--service-account-id`, etc.) — highest priority
2. Instance config from TOML (selected by `--instance` or `ASCEND_INSTANCE` env var)
3. Env vars (`ASCEND_SERVICE_ACCOUNT_ID`, etc.) — fallback
4. Error

Auth params can also be passed as CLI flags (`--service-account-id`, `--service-account-key`, etc.). Secret values are hidden in `--help` output.

### local dev

```bash
export ASCEND_INSTANCE_API_URL="https://<workspace>-instance.api.local.ascend.dev"
```

If you accidentally use the matching `https://<workspace>-instance.app.local.ascend.dev` host instead, the shared config path will correct that specific local-dev confusion to the `instance.api.local` host automatically.

## CLI reference

```
ascend-tools [-o text|json] [-V] [--instance <NAME>]

  instance add <NAME> --service-account-id <ID> --instance-api-url <URL> [--service-account-key-env <ENV_VAR>]
  instance list
  instance remove <NAME>
  instance set-default <NAME>

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

  environment list
  environment get <TITLE>

  project list
  project get <TITLE>

  profile list --workspace <TITLE> | --deployment <TITLE> | --project <NAME> --git-branch <BRANCH>

  flow list --workspace <TITLE> | --deployment <TITLE>
  flow run <FLOW_NAME> --workspace <TITLE> | --deployment <TITLE> [--spec '{}'] [--resume]
  flow list-runs --workspace <TITLE> | --deployment <TITLE> [--status, -f/--flow, --since, --until, --offset, --limit]
  flow get-run <RUN_NAME> --workspace <TITLE> | --deployment <TITLE>

  otto run <PROMPT> [--workspace <TITLE> | --deployment <TITLE>] [--provider <ID>] [--model <ID>] [--conversation <TITLE_OR_ID> | --resume] [--jsonl]
  otto provider list
  otto model list [--provider <ID>]
  otto tui [--workspace <TITLE> | --deployment <TITLE>] [--provider <ID>] [--model <ID>] [--conversation <TITLE_OR_ID> | --resume]
  otto conversation list [--limit <N>] [--offset <N>]
  otto conversation get <TITLE>
  otto conversation get <ID> --id
  otto conversation open <TITLE_OR_ID> [--id] [--after <MESSAGE_ID>]
  otto conversation history <TITLE_OR_ID> [--id] --before <MESSAGE_ID> [--limit <N>]

  signup

  skill install --target <PATH> [--cli] [--python] [--javascript] [--rust] [--mcp] [--all]

  mcp [--http] [--bind <ADDR>]
```

Default output is table format. Use `-o json` for machine-readable output.

`--environment` and `--project` accept friendly names (titles), not UUIDs. UUIDs still work for all commands via `--uuid` flag.

No subcommand prints help.

## TUI reference

`ascend-tools otto tui` launches an interactive full-screen chat interface powered by the `ascend-tools-tui` crate.

### features

- **Vi keybindings** (default) — Esc for normal mode, i/a/I/A to insert. `/emacs` to switch.
- **Multi-line input** — Alt+Enter inserts a newline. Input area grows up to 8 lines.
- **Input history** — Up/Down recalls previous prompts. Persisted across sessions (`~/.ascend-tools/history`).
- **Streaming** — Smooth character-by-character output (~200 cps) with spinner while waiting.
- **Markdown rendering** — Code blocks with borders, `**bold**`, `` `inline code` ``.
- **Scrollable chat** — PageUp/Down, mouse wheel, Ctrl+U/D in vi normal. Scrollbar on right edge.
- **Tab completion** — Type `/` and press Tab to cycle through slash commands.
- **Clipboard** — `/copy` copies last Otto response to clipboard.
- **Timestamps** — `/timestamps` toggles message timestamps.
- **Cursor shape** — Block in vi normal, blinking bar in insert/emacs.
- **Context indicator** — Workspace/deployment name shown in status bar.
- **Tool call details** — Ctrl+o toggles expanded view of tool call arguments and output.
- **Notification bell** — Terminal bell when responses take >3 seconds.

### slash commands

| Command | Description |
|---------|-------------|
| `/help` | Show commands and keybindings |
| `/vim`, `/vi` | Switch to Vi keybindings |
| `/emacs` | Switch to Emacs keybindings |
| `/copy` | Copy last Otto response to clipboard |
| `/timestamps` | Toggle message timestamps |
| `/clear` | Clear chat history and start new thread |
| `/quit`, `/exit` | Exit |

## Python SDK reference

```python
from ascend_tools import Client

# All params optional — resolved from instance config or env vars
client = Client()

# Use a specific named instance from ~/.ascend-tools/config.toml
client = Client(instance="staging")

# Or explicit — only need the instance API URL
client = Client(
    service_account_id="asc-sa-...",
    service_account_key="...",
    instance_api_url="https://api.instance.ascend.io",
)

# Environments & Projects
client.list_environments()
client.get_environment(title="Production")
client.list_projects()
client.get_project(title="My Project")
client.list_profiles(workspace="My Workspace")

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

# Otto (AI assistant)
client.list_otto_providers()
client.otto(prompt="What flows are running?", workspace="My Workspace")

# Conversations
client.list_conversations(limit=10)
client.get_conversation(title="My conversation")
client.get_conversation(id="thread-abc123")
```

All methods return `dict` or `list[dict]`. All parameters are keyword-only.

## JavaScript SDK reference

```javascript
import { Client } from "ascend-tools";

// All params optional — resolved from instance config or env vars
const client = new Client();

// Use a specific named instance from ~/.ascend-tools/config.toml
const client = new Client(null, null, null, "staging");

// Or explicit
const client = new Client(
  "asc-sa-...",                      // serviceAccountId
  "...",                              // serviceAccountKey
  "https://api.instance.ascend.io",   // instanceApiUrl
);

// Environments & Projects
await client.listEnvironments();
await client.getEnvironment("Production");
await client.listProjects();
await client.getProject("My Project");
await client.listProfiles("My Workspace");

// Workspaces
await client.listWorkspaces();
await client.getWorkspace("My Workspace");
await client.pauseWorkspace("My Workspace");
await client.resumeWorkspace("My Workspace");
await client.deleteWorkspace("My Workspace");

// Deployments
await client.listDeployments();
await client.getDeployment("My Deployment");
await client.deleteDeployment("My Deployment");

// Flows
await client.listFlows("My Workspace");
await client.runFlow("sales", "My Workspace");

// Flow runs
await client.listFlowRuns("My Workspace", null, null, "running");
await client.getFlowRun("fr-...", "My Workspace");

// Otto (AI assistant)
await client.listOttoProviders();
await client.otto("What flows are running?", "My Workspace");

// Conversations
await client.listConversations(0, 10);
await client.getConversation("My conversation");
await client.getConversation("thread-abc123", true);  // by ID
```

All methods are async (return Promises). All methods return plain objects/arrays. TypeScript type definitions are included (`index.d.cts`).

## MCP server

The `mcp` subcommand starts an MCP server exposing AscendClient methods as tools for AI assistants (Claude Code, Claude Desktop, Cursor, etc.).

### transports

- **stdio** (default): `ascend-tools mcp` — communicates over stdin/stdout.
- **HTTP**: `ascend-tools mcp --http [--bind 127.0.0.1:8000]` — Streamable HTTP on `/mcp`.

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
| `list_environments` | List environments |
| `get_environment` | Get an environment by title |
| `list_projects` | List projects |
| `get_project` | Get a project by title |
| `list_profiles` | List profiles for a workspace, deployment, or project+branch |
| `list_flows` | List flows in a workspace or deployment |
| `run_flow` | Trigger a flow run with typed spec (resume, full_refresh, components, parameters, etc.) |
| `list_flow_runs` | List flow runs with filters (status, flow, since, until, offset, limit) |
| `get_flow_run` | Get a flow run by name |
| `list_conversations` | List recent Otto conversations |
| `get_conversation` | Get an Otto conversation by title or ID |
| `list_otto_providers` | List Otto providers and their enabled models |
| `otto` | Chat with Otto, the Ascend AI assistant |

### usage with Claude Code

```bash
claude mcp add --transport stdio ascend-tools-dev -- uvx ascend-tools mcp    # via uv
claude mcp add --transport stdio ascend-tools-dev -- npx ascend-tools mcp    # via npm
```

Auth env vars are inherited from the shell. If Claude is launched without your shell env, set them explicitly:

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
codex mcp add ascend-tools-dev -- uvx ascend-tools mcp    # via uv
codex mcp add ascend-tools-dev -- npx ascend-tools mcp    # via npm
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
codex mcp list
codex mcp remove ascend-tools
codex mcp remove ascend-tools-dev
```

If stale behavior appears after code updates, or Codex MCP startup fails with `connection closed: initialize response`, refresh once:

```bash
uvx --refresh ascend-tools --version
```

## conventions

- `docs/` naming: `snake_case.md` for public-facing docs, `SCREAMING_SNAKE_CASE.md` for internal-only (e.g. `DEVELOPMENT.md`)
- Rust stable toolchain (edition 2024, requires 1.85+)
- Commits and PR titles use Conventional Commits (`type(scope): summary` when scoped, otherwise `type: summary`); use `refactor:` for internal quality improvements without behavior changes
- API methods return typed structs in Rust (`serde_json::Value` used only for dynamic fields like `FlowRun.error`)
- CLI prints tables by default, JSON with `-o json`; empty results print "No results." to stderr
- Clap args for secrets use `hide_env_values = true` (SA ID, SA key)
- PyO3 `run_cli()` uses `py.detach()` to release the GIL during long-running Rust calls (MCP server)
- Test coverage includes integration tests with mock servers (`mockito`) for core HTTP/auth behavior, MCP tool behavior, and CLI output regressions
- When adding or changing CLI commands, update `crates/ascend-tools-cli/src/skill-cli.md` to keep the skill in sync
