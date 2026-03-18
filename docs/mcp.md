# Set up the MCP server

Connect AI assistants to Ascend using the [MCP](https://modelcontextprotocol.io/) (Model Context Protocol) server. Exposes 25 tools for managing workspaces, deployments, environments, projects, profiles, flows, and Otto. Works with Claude Code, Claude Desktop, Codex CLI, Cursor, and other MCP-compatible clients.

## Remote MCP server (recommended)

Every Ascend instance hosts a remote MCP server — no local installation required. Find the URL at **Settings > Instance > MCP Server** in the Ascend UI.

### Claude Code

```bash
# Copy ASCEND_MCP_URL from Settings > Instance > MCP Server
claude mcp add --transport http ascend-tools $ASCEND_MCP_URL
```

Authentication is handled automatically via OAuth (browser login). No service account or env vars needed.

### Codex CLI

```bash
codex mcp add ascend-tools --url $ASCEND_MCP_URL
```

### Other MCP clients

Point your client at the MCP URL from Settings using the **Streamable HTTP** transport. The server supports OAuth 2.1 with PKCE for authentication.

## Local MCP server (alternative)

For offline development, custom configurations, or when you prefer running tools locally.

### Claude Code

```bash
claude mcp add --transport stdio ascend-tools-dev -- uvx ascend-tools mcp    # via uv
claude mcp add --transport stdio ascend-tools-dev -- npx ascend-tools mcp    # via npm
```

Auth environment variables (`ASCEND_SERVICE_ACCOUNT_ID`, `ASCEND_SERVICE_ACCOUNT_KEY`, `ASCEND_INSTANCE_API_URL`) are inherited from your shell. See [Quickstart](QUICKSTART.md) for the full service account creation walkthrough.

If Claude is launched without your shell env, pass them explicitly:

```bash
claude mcp add --transport stdio ascend-tools-dev \
  -e ASCEND_SERVICE_ACCOUNT_ID="$ASCEND_SERVICE_ACCOUNT_ID" \
  -e ASCEND_SERVICE_ACCOUNT_KEY="$ASCEND_SERVICE_ACCOUNT_KEY" \
  -e ASCEND_INSTANCE_API_URL="$ASCEND_INSTANCE_API_URL" \
  -- uvx ascend-tools mcp
```

### Codex CLI

```bash
codex mcp add ascend-tools-dev -- uvx ascend-tools mcp    # via uv
codex mcp add ascend-tools-dev -- npx ascend-tools mcp    # via npm
```

If Codex is launched without your shell env, pass them explicitly:

```bash
codex mcp add \
  --env "ASCEND_SERVICE_ACCOUNT_ID=$ASCEND_SERVICE_ACCOUNT_ID" \
  --env "ASCEND_SERVICE_ACCOUNT_KEY=$ASCEND_SERVICE_ACCOUNT_KEY" \
  --env "ASCEND_INSTANCE_API_URL=$ASCEND_INSTANCE_API_URL" \
  ascend-tools-dev -- uvx ascend-tools mcp
```

### Other transports

```bash
# Stdio (default) -- communicates over stdin/stdout
ascend-tools mcp

# HTTP -- Streamable HTTP on /mcp endpoint
ascend-tools mcp --http --bind 127.0.0.1:8000
```

### Verify

- Claude Code: run `/mcp` and confirm you see the server name you configured (`ascend-tools` for remote, `ascend-tools-dev` for local setup via uv/uvx).
- Codex CLI: run `codex mcp list` and `codex mcp get <name> --json`, then start a session and confirm MCP tools are available.

### Manage

```bash
claude mcp remove ascend-tools
claude mcp remove ascend-tools-dev
codex mcp list
codex mcp remove ascend-tools
codex mcp remove ascend-tools-dev
```

## Tools reference

### list_workspaces

List workspaces with optional filters.

| Parameter | Required | Type | Description |
|-----------|----------|------|-------------|
| `environment` | no | string | Filter by environment title |
| `project` | no | string | Filter by project title |

### get_workspace

Get a workspace by title.

| Parameter | Required | Type | Description |
|-----------|----------|------|-------------|
| `title` | yes | string | Workspace title |

### create_workspace

Create a new workspace.

| Parameter | Required | Type | Description |
|-----------|----------|------|-------------|
| `title` | yes | string | Workspace title |
| `environment` | yes | string | Environment name (or UUID) |
| `project` | yes | string | Project name (or UUID) |
| `profile` | yes | string | Configuration profile |
| `git_branch` | yes | string | Git branch |
| `git_branch_base` | no | string | Base git branch |
| `size` | no | string | Size (e.g. Small, Medium, Large) |
| `storage_size` | no | integer | Storage size in GB |
| `auto_snooze_timeout_minutes` | no | integer | Minutes of inactivity before auto-snooze |

### update_workspace

Update an existing workspace. Only provided fields are changed.

| Parameter | Required | Type | Description |
|-----------|----------|------|-------------|
| `current_title` | yes | string | Current workspace title |
| `uuid` | no | string | UUID override (skip title lookup) |
| `title` | no | string | New title |
| `git_branch` | no | string | New git branch |
| `git_branch_base` | no | string | New base git branch |
| `profile` | no | string | New profile |
| `size` | no | string | New size |
| `storage_size` | no | integer | New storage size in GB |
| `auto_snooze_timeout_minutes` | no | integer | New auto-snooze timeout in minutes |

### pause_workspace

Pause a running workspace.

| Parameter | Required | Type | Description |
|-----------|----------|------|-------------|
| `title` | yes | string | Workspace title |

### resume_workspace

Resume a paused workspace.

| Parameter | Required | Type | Description |
|-----------|----------|------|-------------|
| `title` | yes | string | Workspace title |

### delete_workspace

Delete a workspace.

| Parameter | Required | Type | Description |
|-----------|----------|------|-------------|
| `title` | yes | string | Workspace title |

### list_deployments

List deployments with optional filters.

| Parameter | Required | Type | Description |
|-----------|----------|------|-------------|
| `environment` | no | string | Filter by environment title |
| `project` | no | string | Filter by project title |

### get_deployment

Get a deployment by title.

| Parameter | Required | Type | Description |
|-----------|----------|------|-------------|
| `title` | yes | string | Deployment title |

### create_deployment

Create a new deployment.

| Parameter | Required | Type | Description |
|-----------|----------|------|-------------|
| `title` | yes | string | Deployment title |
| `environment` | yes | string | Environment name (or UUID) |
| `project` | yes | string | Project name (or UUID) |
| `profile` | yes | string | Configuration profile |
| `git_branch` | yes | string | Git branch |
| `git_branch_base` | no | string | Base git branch |
| `size` | no | string | Size (e.g. Small, Medium, Large) |
| `storage_size` | no | integer | Storage size in GB |
| `enable_automations` | no | boolean | Enable automations |

### update_deployment

Update an existing deployment. Only provided fields are changed.

| Parameter | Required | Type | Description |
|-----------|----------|------|-------------|
| `current_title` | yes | string | Current deployment title |
| `uuid` | no | string | UUID override (skip title lookup) |
| `title` | no | string | New title |
| `git_branch` | no | string | New git branch |
| `git_branch_base` | no | string | New base git branch |
| `profile` | no | string | New profile |
| `size` | no | string | New size |
| `storage_size` | no | integer | New storage size in GB |
| `enable_automations` | no | boolean | Enable or disable automations |

### pause_deployment_automations

Pause automations on a deployment.

| Parameter | Required | Type | Description |
|-----------|----------|------|-------------|
| `title` | yes | string | Deployment title |

### resume_deployment_automations

Resume automations on a deployment.

| Parameter | Required | Type | Description |
|-----------|----------|------|-------------|
| `title` | yes | string | Deployment title |

### delete_deployment

Delete a deployment.

| Parameter | Required | Type | Description |
|-----------|----------|------|-------------|
| `title` | yes | string | Deployment title |

### list_environments

List environments. No parameters.

### get_environment

Get an environment by title.

| Parameter | Required | Type | Description |
|-----------|----------|------|-------------|
| `title` | yes | string | Environment title |

### list_projects

List projects. No parameters.

### get_project

Get a project by title.

| Parameter | Required | Type | Description |
|-----------|----------|------|-------------|
| `title` | yes | string | Project title |

### list_profiles

List available profiles for a workspace, deployment, or project+branch.

| Parameter | Required | Type | Description |
|-----------|----------|------|-------------|
| `workspace` | no | string | Workspace title |
| `deployment` | no | string | Deployment title |
| `uuid` | no | string | UUID (direct override) |
| `project` | no | string | Project name (or UUID) — use with branch |
| `branch` | no | string | Git branch (required with project) |

### list_flows

List flows in a workspace or deployment.

| Parameter | Required | Type | Description |
|-----------|----------|------|-------------|
| `workspace` | no | string | Workspace title (provide one of workspace or deployment) |
| `deployment` | no | string | Deployment title (provide one of workspace or deployment) |

### run_flow

Trigger a flow run. Checks health first.

| Parameter | Required | Type | Description |
|-----------|----------|------|-------------|
| `workspace` | no | string | Workspace title (provide one of workspace or deployment) |
| `deployment` | no | string | Deployment title (provide one of workspace or deployment) |
| `flow` | yes | string | Flow name |
| `spec` | no | object | Flow run options (see below) |
| `resume` | no | boolean | Resume the workspace/deployment if paused before running |

### list_flow_runs

List flow runs with optional filters.

| Parameter | Required | Type | Description |
|-----------|----------|------|-------------|
| `workspace` | no | string | Workspace title (provide one of workspace or deployment) |
| `deployment` | no | string | Deployment title (provide one of workspace or deployment) |
| `status` | no | string | Filter by status (pending, running, succeeded, failed) |
| `flow` | no | string | Filter by flow name |
| `since` | no | string | Filter by start time (ISO 8601) |
| `until` | no | string | Filter by end time (ISO 8601) |
| `offset` | no | integer | Pagination offset |
| `limit` | no | integer | Pagination limit |

### get_flow_run

Get a flow run by name.

| Parameter | Required | Type | Description |
|-----------|----------|------|-------------|
| `workspace` | no | string | Workspace title (provide one of workspace or deployment) |
| `deployment` | no | string | Deployment title (provide one of workspace or deployment) |
| `name` | yes | string | Flow run name |

### list_otto_providers

List available Otto providers and their enabled models. No parameters.

### otto

Chat with Otto, the Ascend AI assistant.

| Parameter | Required | Type | Description |
|-----------|----------|------|-------------|
| `prompt` | yes | string | Message to send to Otto |
| `workspace` | no | string | Workspace title for context |
| `deployment` | no | string | Deployment title for context |
| `uuid` | no | string | UUID (direct override) |
| `provider` | no | string | LLM provider name |
| `model` | no | string | LLM model ID |
| `thread_id` | no | string | Thread ID to continue a conversation |

## Flow run spec

The `spec` parameter on `run_flow` accepts these fields:

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

## Troubleshooting

### Stale behavior after updating

If the MCP server shows old behavior after a code update, clear the uvx cache:

```bash
uvx --refresh ascend-tools --version
```

### Environment variables not inherited

Some IDE-launched shells, tmux sessions, or remote environments don't inherit your shell profile. Pass the env vars explicitly during `mcp add` (see setup instructions above).

### Codex startup error: `connection closed: initialize response`

This means the MCP process exited before Codex received the initialize response.

Most common fixes:

```bash
# 1) Re-add remote server with the correct Codex HTTP syntax
codex mcp add ascend-tools --url $ASCEND_MCP_URL

# 2) Re-add local stdio server with uvx
codex mcp remove ascend-tools-dev
codex mcp add \
  --env "ASCEND_SERVICE_ACCOUNT_ID=$ASCEND_SERVICE_ACCOUNT_ID" \
  --env "ASCEND_SERVICE_ACCOUNT_KEY=$ASCEND_SERVICE_ACCOUNT_KEY" \
  --env "ASCEND_INSTANCE_API_URL=$ASCEND_INSTANCE_API_URL" \
  ascend-tools-dev -- uvx ascend-tools mcp

# 3) Refresh uvx cache if behavior seems stale
uvx --refresh ascend-tools --version
```
