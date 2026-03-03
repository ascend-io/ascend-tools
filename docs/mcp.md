# Set up the MCP server

Connect AI assistants to Ascend using the ascend-tools MCP server.

## Overview

The `ascend-tools mcp` subcommand starts an [MCP](https://modelcontextprotocol.io/) (Model Context Protocol) server that exposes 8 tools for managing Ascend runtimes and flows. It works with Claude Code, Claude Desktop, Codex CLI, Cursor, and other MCP-compatible clients.

## Set up with Claude Code

### Install

```bash
claude mcp add ascend-tools -- uvx ascend-tools mcp
```

Auth environment variables (`ASCEND_SERVICE_ACCOUNT_ID`, `ASCEND_SERVICE_ACCOUNT_KEY`, `ASCEND_INSTANCE_API_URL`) are inherited from your shell. See [Quickstart](QUICKSTART.md) for the full service account creation walkthrough.

If Claude is launched without your shell env, pass them explicitly:

```bash
claude mcp add --transport stdio \
  -e ASCEND_SERVICE_ACCOUNT_ID="$ASCEND_SERVICE_ACCOUNT_ID" \
  -e ASCEND_SERVICE_ACCOUNT_KEY="$ASCEND_SERVICE_ACCOUNT_KEY" \
  -e ASCEND_INSTANCE_API_URL="$ASCEND_INSTANCE_API_URL" \
  ascend-tools -- uvx ascend-tools mcp
```

### Verify

Run `/mcp` inside Claude Code. You should see `ascend-tools` listed with 8 tools.

### Remove

```bash
claude mcp remove ascend-tools
```

## Set up with Codex CLI

### Install

```bash
codex mcp add ascend-tools -- uvx ascend-tools mcp
```

If Codex is launched without your shell env, pass them explicitly:

```bash
codex mcp add \
  --env "ASCEND_SERVICE_ACCOUNT_ID=$ASCEND_SERVICE_ACCOUNT_ID" \
  --env "ASCEND_SERVICE_ACCOUNT_KEY=$ASCEND_SERVICE_ACCOUNT_KEY" \
  --env "ASCEND_INSTANCE_API_URL=$ASCEND_INSTANCE_API_URL" \
  ascend-tools -- uvx ascend-tools mcp
```

### Inspect and manage

```bash
codex mcp get ascend-tools --json
codex mcp list
codex mcp remove ascend-tools
```

## Set up with other MCP clients

### Stdio transport (default)

```bash
ascend-tools mcp
```

Communicates over stdin/stdout. Most MCP clients use this transport.

### HTTP transport

```bash
ascend-tools mcp --http --bind 127.0.0.1:8000
```

Streamable HTTP on the `/mcp` endpoint. Use for remote or shared deployments, or clients that don't support stdio.

## Tools reference

### list_runtimes

List runtimes with optional filters.

| Parameter | Required | Type | Description |
|-----------|----------|------|-------------|
| `id` | no | string | Filter by runtime ID |
| `kind` | no | string | Filter by runtime kind |
| `project_uuid` | no | string | Filter by project UUID |
| `environment_uuid` | no | string | Filter by environment UUID |

### get_runtime

Get a runtime by UUID.

| Parameter | Required | Type | Description |
|-----------|----------|------|-------------|
| `uuid` | yes | string | Runtime UUID |

### resume_runtime

Resume a paused runtime.

| Parameter | Required | Type | Description |
|-----------|----------|------|-------------|
| `runtime_uuid` | yes | string | Runtime UUID |

### pause_runtime

Pause a running runtime.

| Parameter | Required | Type | Description |
|-----------|----------|------|-------------|
| `runtime_uuid` | yes | string | Runtime UUID |

### list_flows

List flows in a runtime.

| Parameter | Required | Type | Description |
|-----------|----------|------|-------------|
| `runtime_uuid` | yes | string | Runtime UUID |

### run_flow

Trigger a flow run. Checks runtime health first.

| Parameter | Required | Type | Description |
|-----------|----------|------|-------------|
| `runtime_uuid` | yes | string | Runtime UUID |
| `flow_name` | yes | string | Flow name |
| `spec` | no | object | Flow run options (see below) |
| `resume` | no | boolean | Resume the runtime if paused before running |

### list_flow_runs

List flow runs with optional filters.

| Parameter | Required | Type | Description |
|-----------|----------|------|-------------|
| `runtime_uuid` | yes | string | Runtime UUID |
| `status` | no | string | Filter by status (pending, running, succeeded, failed) |
| `flow_name` | no | string | Filter by flow name |
| `since` | no | string | Filter by start time (ISO 8601) |
| `until` | no | string | Filter by end time (ISO 8601) |
| `offset` | no | integer | Pagination offset |
| `limit` | no | integer | Pagination limit |

### get_flow_run

Get a flow run by name.

| Parameter | Required | Type | Description |
|-----------|----------|------|-------------|
| `runtime_uuid` | yes | string | Runtime UUID |
| `name` | yes | string | Flow run name |

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
