# Use the CLI

Manage Ascend workspaces, deployments, flows, and flow runs from the command line.

## Install

```bash
uv tool install ascend-tools      # Python
npm install -g ascend-tools       # Node.js
cargo install ascend-tools-cli    # Rust
```

Upgrade to the latest version:

```bash
uv tool install --upgrade ascend-tools    # Python
npm update -g ascend-tools                # Node.js
cargo install ascend-tools-cli            # Rust (reinstalls latest)
```

See [Installation](installation.md) for other methods (pre-built binaries, `uvx`, `npx`).

## Authenticate

Set three environment variables (see [Quickstart](quickstart.md) for the full service account creation walkthrough):

```bash
export ASCEND_SERVICE_ACCOUNT_ID="<YOUR_SERVICE_ACCOUNT_ID>"
export ASCEND_SERVICE_ACCOUNT_KEY="<YOUR_SERVICE_ACCOUNT_KEY>"
export ASCEND_INSTANCE_API_URL="<YOUR_INSTANCE_API_URL>"
```

You can also pass credentials as CLI flags: `--service-account-id`, `--service-account-key`, `--instance-api-url`. Flags override environment variables.

## Manage workspaces

### List workspaces

```bash
ascend-tools workspace list
```

Filter by environment or project:

```bash
ascend-tools workspace list --environment "Production"
ascend-tools workspace list --project "My Project"
```

### Get a workspace

```bash
ascend-tools workspace get "My Workspace"
```

### Create a workspace

```bash
ascend-tools workspace create --title "My Workspace" --environment "Production" --project "My Project" --profile default --git-branch main
ascend-tools workspace create --title "My Workspace" --environment "Production" --project "My Project" --profile default --git-branch main --size Medium
```

### Update a workspace

```bash
ascend-tools workspace update "My Workspace" --title "Renamed Workspace"
ascend-tools workspace update "My Workspace" --git-branch feature/new
ascend-tools workspace update "My Workspace" --profile production --size Large
```

### Pause and resume a workspace

```bash
ascend-tools workspace pause "My Workspace"
ascend-tools workspace resume "My Workspace"
```

### Delete a workspace

```bash
ascend-tools workspace delete "My Workspace"
```

## Manage deployments

### List deployments

```bash
ascend-tools deployment list
```

Filter by environment or project:

```bash
ascend-tools deployment list --environment "Production"
ascend-tools deployment list --project "My Project"
```

### Get a deployment

```bash
ascend-tools deployment get "My Deployment"
```

### Create a deployment

```bash
ascend-tools deployment create --title "My Deployment" --environment "Production" --project "My Project" --profile default --git-branch main
ascend-tools deployment create --title "My Deployment" --environment "Production" --project "My Project" --profile default --git-branch main --enable-automations true
```

### Update a deployment

```bash
ascend-tools deployment update "My Deployment" --title "Renamed Deployment"
ascend-tools deployment update "My Deployment" --git-branch release/v2
ascend-tools deployment update "My Deployment" --enable-automations false
```

### Pause and resume deployment automations

```bash
ascend-tools deployment pause-automations "My Deployment"
ascend-tools deployment resume-automations "My Deployment"
```

### Delete a deployment

```bash
ascend-tools deployment delete "My Deployment"
```

## Environments, projects, and profiles

```bash
# List environments
ascend-tools environment list
ascend-tools environment get "Production"

# List projects
ascend-tools project list
ascend-tools project get "My Project"

# List profiles (requires a workspace/deployment or project+branch)
ascend-tools profile list --workspace "My Workspace"
ascend-tools profile list --deployment "My Deployment"
ascend-tools profile list --project "My Project" --git-branch main
```

## Manage flows

### List flows

```bash
ascend-tools flow list --workspace "My Workspace"
ascend-tools flow list --deployment "My Deployment"
```

### Run a flow

```bash
ascend-tools flow run "My Flow" --workspace "My Workspace"
ascend-tools flow run "My Flow" --deployment "My Deployment"
```

Resume a paused workspace before running:

```bash
ascend-tools flow run "My Flow" --workspace "My Workspace" --resume
```

Pass a flow run spec for advanced options:

```bash
ascend-tools flow run "My Flow" --workspace "My Workspace" \
  --spec '{"full_refresh": true}'
```

### Flow run spec options

| Field | Type | Description |
|-------|------|-------------|
| `full_refresh` | bool | Drop all internal data and recompute from scratch. **Destructive.** |
| `components` | list | Run only these components (by name). Omit to run all. |
| `component_categories` | list | Run only components in these categories. |
| `parameters` | object | Custom parameters passed to the flow. |
| `run_tests` | bool | Run tests after processing data. Defaults to true. |
| `store_test_results` | bool | Store test results. |
| `halt_flow_on_error` | bool | Stop the flow on error. |
| `disable_optimizers` | bool | Disable optimizers. |
| `update_materialization_type` | bool | Update component materialization types. **May drop data.** |
| `deep_data_pruning` | bool | Full table scan for Smart Table data maintenance. |
| `backfill_missing_statistics` | bool | Backfill statistics for data blocks without them. |
| `disable_incremental_metadata_collection` | bool | Disable incremental read/transform metadata collection. |
| `runner_overrides` | object | Runner config overrides (e.g., `{"size": "Medium"}`). |

## Monitor flow runs

### List flow runs

```bash
ascend-tools flow list-runs --workspace "My Workspace"
ascend-tools flow list-runs --deployment "My Deployment"
```

Filter by status, flow name, time range, or paginate:

```bash
ascend-tools flow list-runs --workspace "My Workspace" --status running
ascend-tools flow list-runs --workspace "My Workspace" --flow sales
ascend-tools flow list-runs --workspace "My Workspace" --since 2025-01-01T00:00:00Z
ascend-tools flow list-runs --workspace "My Workspace" --limit 10 --offset 20
```

### Get a flow run

```bash
ascend-tools flow get-run <RUN_NAME> --workspace "My Workspace"
ascend-tools flow get-run <RUN_NAME> --deployment "My Deployment"
```

## Output formats

Default output is a human-readable table. Use `-o json` for machine-readable output:

```bash
ascend-tools -o json workspace list
ascend-tools -o json flow list-runs --workspace "My Workspace"
```

Empty results print "No results." to stderr.

## Otto (AI assistant)

```bash
# One-shot message
ascend-tools otto run "What flows are running?"
ascend-tools otto run "Describe the sales flow" --workspace "My Workspace"
ascend-tools otto run "What flows are running?" --deployment "My Deployment"
ascend-tools otto run "Help me debug this pipeline" --provider "OpenAI" --model gpt-4o
ascend-tools otto run "Capture the raw event trace" --jsonl

# List providers and models
ascend-tools otto provider list
ascend-tools otto model list
ascend-tools otto model list --provider "OpenAI"

# Inspect a conversation through the progressive tools surface
ascend-tools -o json otto conversation open <THREAD_ID> --id
ascend-tools -o json otto conversation open <THREAD_ID> --id --after <LATEST_MESSAGE_ID>
ascend-tools -o json otto conversation history <THREAD_ID> --id --before <OLDEST_MESSAGE_ID> --limit 5

# Interactive chat (Ctrl+C to exit)
ascend-tools otto tui --workspace "My Workspace"
ascend-tools otto tui --deployment "My Deployment"
```

## Install AI assistant skills

Install reference skills for AI coding assistants (Claude Code, Codex, etc.):

```bash
ascend-tools skill install --target .claude/skills --all
```

Available flags: `--cli` (default), `--python`, `--javascript`, `--rust`, `--mcp`, `--all`.
