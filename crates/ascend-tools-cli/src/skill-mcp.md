---
name: ascend-tools-mcp
description: Use the ascend-tools MCP server to manage Ascend workspaces, deployments, flows, and flow runs.
---

# ascend-tools MCP server

Manage Ascend workspaces, deployments, flows, and flow runs via MCP tools.

## Setup

### Remote server (recommended)

Copy the MCP URL from **Settings > Instance > MCP Server** in the Ascend UI, then:

```bash
claude mcp add --transport http ascend-tools $ASCEND_MCP_URL
```

Authentication is handled automatically via OAuth. No service account or env vars needed.

### Local server (alternative)

For offline development or custom configurations:

```bash
claude mcp add --transport stdio ascend-tools-dev -- uvx ascend-tools mcp
```

Requires service account env vars (from Ascend UI > Settings > Users > Create Service Account):

```bash
export ASCEND_SERVICE_ACCOUNT_ID="asc-sa-..."
export ASCEND_SERVICE_ACCOUNT_KEY="..."
export ASCEND_INSTANCE_API_URL="https://api.<instance>.ascend.io"
```

If env vars are not inherited from your shell, pass them explicitly to `claude mcp add` with `-e`.

For local setup via uv/uvx, use server name `ascend-tools-dev`.

## Tools

### list_workspaces

List workspaces with optional filters.

| Parameter | Required | Description |
|-----------|----------|-------------|
| `environment` | no | Filter by environment title |
| `project` | no | Filter by project title |

### get_workspace

Get a workspace by title.

| Parameter | Required | Description |
|-----------|----------|-------------|
| `title` | yes | Workspace title |

### create_workspace

Create a new workspace.

| Parameter | Required | Description |
|-----------|----------|-------------|
| `title` | yes | Workspace title |
| `environment` | yes | Environment name (or UUID) |
| `project` | yes | Project name (or UUID) |
| `profile` | yes | Configuration profile |
| `git_branch` | yes | Git branch |
| `git_branch_base` | no | Base git branch |
| `size` | no | Size (e.g. Small, Medium, Large) |
| `storage_size` | no | Storage size in GB |
| `auto_snooze_timeout_minutes` | no | Minutes of inactivity before auto-snooze |

### update_workspace

Update an existing workspace. Only provided fields are changed.

| Parameter | Required | Description |
|-----------|----------|-------------|
| `current_title` | yes | Current workspace title |
| `uuid` | no | UUID override (skip title lookup) |
| `title` | no | New title |
| `git_branch` | no | New git branch |
| `git_branch_base` | no | New base git branch |
| `profile` | no | New profile |
| `size` | no | New size |
| `storage_size` | no | New storage size in GB |
| `auto_snooze_timeout_minutes` | no | New auto-snooze timeout in minutes |

### pause_workspace

Pause a running workspace.

| Parameter | Required | Description |
|-----------|----------|-------------|
| `title` | yes | Workspace title |

### resume_workspace

Resume a paused workspace.

| Parameter | Required | Description |
|-----------|----------|-------------|
| `title` | yes | Workspace title |

### delete_workspace

Delete a workspace.

| Parameter | Required | Description |
|-----------|----------|-------------|
| `title` | yes | Workspace title |

### list_deployments

List deployments with optional filters.

| Parameter | Required | Description |
|-----------|----------|-------------|
| `environment` | no | Filter by environment title |
| `project` | no | Filter by project title |

### get_deployment

Get a deployment by title.

| Parameter | Required | Description |
|-----------|----------|-------------|
| `title` | yes | Deployment title |

### create_deployment

Create a new deployment.

| Parameter | Required | Description |
|-----------|----------|-------------|
| `title` | yes | Deployment title |
| `environment` | yes | Environment name (or UUID) |
| `project` | yes | Project name (or UUID) |
| `profile` | yes | Configuration profile |
| `git_branch` | yes | Git branch |
| `git_branch_base` | no | Base git branch |
| `size` | no | Size (e.g. Small, Medium, Large) |
| `storage_size` | no | Storage size in GB |
| `enable_automations` | no | Enable automations |

### update_deployment

Update an existing deployment. Only provided fields are changed.

| Parameter | Required | Description |
|-----------|----------|-------------|
| `current_title` | yes | Current deployment title |
| `uuid` | no | UUID override (skip title lookup) |
| `title` | no | New title |
| `git_branch` | no | New git branch |
| `git_branch_base` | no | New base git branch |
| `profile` | no | New profile |
| `size` | no | New size |
| `storage_size` | no | New storage size in GB |
| `enable_automations` | no | Enable or disable automations |

### pause_deployment_automations

Pause automations on a deployment.

| Parameter | Required | Description |
|-----------|----------|-------------|
| `title` | yes | Deployment title |

### resume_deployment_automations

Resume automations on a deployment.

| Parameter | Required | Description |
|-----------|----------|-------------|
| `title` | yes | Deployment title |

### delete_deployment

Delete a deployment.

| Parameter | Required | Description |
|-----------|----------|-------------|
| `title` | yes | Deployment title |

### list_environments

List environments. No parameters.

### get_environment

Get an environment by title.

| Parameter | Required | Description |
|-----------|----------|-------------|
| `title` | yes | Environment title |

### list_projects

List projects. No parameters.

### get_project

Get a project by title.

| Parameter | Required | Description |
|-----------|----------|-------------|
| `title` | yes | Project title |

### list_profiles

List available profiles for a workspace, deployment, or project+branch.

| Parameter | Required | Description |
|-----------|----------|-------------|
| `workspace` | no | Workspace title |
| `deployment` | no | Deployment title |
| `uuid` | no | UUID (direct override) |
| `project` | no | Project name (or UUID) — use with branch |
| `branch` | no | Git branch (required with project) |

### list_flows

List flows in a workspace or deployment.

| Parameter | Required | Description |
|-----------|----------|-------------|
| `workspace` | no | Workspace title (provide one of workspace or deployment) |
| `deployment` | no | Deployment title (provide one of workspace or deployment) |

### run_flow

Trigger a flow run. Checks health first; use `resume: true` to resume a paused workspace/deployment before running.

| Parameter | Required | Description |
|-----------|----------|-------------|
| `workspace` | no | Workspace title (provide one of workspace or deployment) |
| `deployment` | no | Deployment title (provide one of workspace or deployment) |
| `flow` | yes | Flow name |
| `spec` | no | Flow run options (see below) |
| `resume` | no | Resume the workspace/deployment if paused before submitting |

### list_flow_runs

List flow runs with optional filters.

| Parameter | Required | Description |
|-----------|----------|-------------|
| `workspace` | no | Workspace title (provide one of workspace or deployment) |
| `deployment` | no | Deployment title (provide one of workspace or deployment) |
| `status` | no | Filter by status |
| `flow` | no | Filter by flow name |
| `since` | no | Filter by start time (ISO 8601) |
| `until` | no | Filter by end time (ISO 8601) |
| `offset` | no | Pagination offset |
| `limit` | no | Pagination limit |

### get_flow_run

Get a flow run by name.

| Parameter | Required | Description |
|-----------|----------|-------------|
| `workspace` | no | Workspace title (provide one of workspace or deployment) |
| `deployment` | no | Deployment title (provide one of workspace or deployment) |
| `name` | yes | Flow run name |

### list_otto_providers

List available Otto providers and their enabled models. No parameters.

### otto

Chat with Otto, the Ascend AI assistant.

| Parameter | Required | Description |
|-----------|----------|-------------|
| `prompt` | yes | Message to send to Otto |
| `workspace` | no | Workspace title for context |
| `deployment` | no | Deployment title for context |
| `uuid` | no | UUID (direct override) |
| `provider` | no | LLM provider name |
| `model` | no | LLM model ID |
| `thread_id` | no | Thread ID to continue a conversation |

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
