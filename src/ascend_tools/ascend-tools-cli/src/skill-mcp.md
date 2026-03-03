---
name: ascend-tools-mcp
description: Use the ascend-tools MCP server to manage Ascend runtimes, flows, and flow runs.
---

# ascend-tools MCP server

Manage Ascend runtimes, flows, and flow runs via MCP tools.

> Private preview: `ascend-tools` is currently in private preview. Contact your Ascend representative to request access via Service Accounts on your Instance.

## Installation

Add to Claude Code:

```bash
claude mcp add --transport stdio ascend-tools -- uvx ascend-tools mcp
```

If env vars are not inherited from your shell, pass them explicitly:

```bash
claude mcp add --transport stdio \
  -e ASCEND_SERVICE_ACCOUNT_ID="$ASCEND_SERVICE_ACCOUNT_ID" \
  -e ASCEND_SERVICE_ACCOUNT_KEY="$ASCEND_SERVICE_ACCOUNT_KEY" \
  -e ASCEND_INSTANCE_API_URL="$ASCEND_INSTANCE_API_URL" \
  ascend-tools -- uvx ascend-tools mcp
```

Upgrade to the latest version (if stale behavior appears after a release):

```bash
uvx --refresh ascend-tools --version
```

## Authentication

Set three environment variables (from Ascend UI > Settings > Users > Create Service Account):

```bash
export ASCEND_SERVICE_ACCOUNT_ID="asc-sa-..."
export ASCEND_SERVICE_ACCOUNT_KEY="..."
export ASCEND_INSTANCE_API_URL="https://<instance-name>.api.instance.ascend.io"
```

## Tools

### list_runtimes

List runtimes with optional filters.

| Parameter | Required | Description |
|-----------|----------|-------------|
| `id` | no | Filter by runtime ID |
| `kind` | no | Filter by runtime kind |
| `project_uuid` | no | Filter by project UUID |
| `environment_uuid` | no | Filter by environment UUID |

### get_runtime

Get a runtime by UUID.

| Parameter | Required | Description |
|-----------|----------|-------------|
| `uuid` | yes | Runtime UUID |

### resume_runtime

Resume a paused runtime.

| Parameter | Required | Description |
|-----------|----------|-------------|
| `runtime_uuid` | yes | Runtime UUID |

### pause_runtime

Pause a running runtime.

| Parameter | Required | Description |
|-----------|----------|-------------|
| `runtime_uuid` | yes | Runtime UUID |

### list_flows

List flows in a runtime.

| Parameter | Required | Description |
|-----------|----------|-------------|
| `runtime_uuid` | yes | Runtime UUID |

### run_flow

Trigger a flow run. Checks runtime health first; use `resume: true` to resume a paused runtime before running.

| Parameter | Required | Description |
|-----------|----------|-------------|
| `runtime_uuid` | yes | Runtime UUID |
| `flow_name` | yes | Flow name |
| `spec` | no | Flow run options (see below) |
| `resume` | no | Resume the runtime if paused before submitting |

### list_flow_runs

List flow runs with optional filters.

| Parameter | Required | Description |
|-----------|----------|-------------|
| `runtime_uuid` | yes | Runtime UUID |
| `status` | no | Filter by status |
| `flow_name` | no | Filter by flow name |
| `since` | no | Filter by start time (ISO 8601) |
| `until` | no | Filter by end time (ISO 8601) |
| `offset` | no | Pagination offset |
| `limit` | no | Pagination limit |

### get_flow_run

Get a flow run by name.

| Parameter | Required | Description |
|-----------|----------|-------------|
| `runtime_uuid` | yes | Runtime UUID |
| `name` | yes | Flow run name |

## Flow run spec

Pass `spec` to `run_flow` to control flow run behavior. All fields are optional:

| Field | Description |
|-------|-------------|
| `full_refresh` | Drop all data and recompute from scratch (destructive) |
| `components` | List of component names to run |
| `component_categories` | List of component categories to run |
| `parameters` | Custom parameters dict passed to the flow |
| `run_tests` | Run tests after processing (default: true) |
| `store_test_results` | Store test results |
| `halt_flow_on_error` | Halt the flow on error |
| `disable_optimizers` | Disable optimizers |
| `update_materialization_type` | Update materialization types (may drop and recompute data) |
| `deep_data_pruning` | Full table scan for Smart Table data maintenance |
| `backfill_missing_statistics` | Backfill statistics for existing data blocks |
| `disable_incremental_metadata_collection` | Disable incremental metadata collection |
| `runner_overrides` | Runner config overrides (e.g. `{"size": "Medium"}`) |
