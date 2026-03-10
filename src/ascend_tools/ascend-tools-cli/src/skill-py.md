---
name: ascend-tools-python
description: Use the ascend-tools Python SDK to manage Ascend workspaces, deployments, flows, and flow runs.
---

# ascend-tools Python SDK

Manage Ascend workspaces, deployments, flows, and flow runs from Python via the `ascend-tools` SDK.

## Installation

```bash
uv add ascend-tools
```

Upgrade to the latest version:

```bash
uv add --upgrade ascend-tools
```

## Authentication

Set three environment variables (from Ascend UI > Settings > Users > Create Service Account):

```bash
export ASCEND_SERVICE_ACCOUNT_ID="asc-sa-..."
export ASCEND_SERVICE_ACCOUNT_KEY="..."
export ASCEND_INSTANCE_API_URL="https://<instance-name>.api.instance.ascend.io"
```

Or pass credentials directly:

```python
from ascend_tools import Client

client = Client(
    service_account_id="asc-sa-...",
    service_account_key="...",
    instance_api_url="https://<instance-name>.api.instance.ascend.io",
)
```

## Usage

All parameters are keyword-only. All methods return `dict` or `list[dict]`.

```python
from ascend_tools import Client

client = Client()
```

### Environments and projects

```python
# List environments
client.list_environments()

# Resolve by title
client.resolve_environment(title="Production")

# List projects
client.list_projects()

# Resolve by title
client.resolve_project(title="My Project")
```

### Workspaces

```python
client.list_workspaces()
client.list_workspaces(environment="Production", project="My Project")
client.get_workspace(title="My Workspace")
client.pause_workspace(title="My Workspace")
client.resume_workspace(title="My Workspace")
client.delete_workspace(title="My Workspace")
```

### Deployments

```python
client.list_deployments()
client.get_deployment(title="My Deployment")
client.delete_deployment(title="My Deployment")
```

### Runtimes (low-level)

```python
client.list_runtimes()
client.list_runtimes(kind="deployment", project_uuid="...", environment_uuid="...")
client.get_runtime(uuid="...")
client.resolve_runtime_by_title(title="My Workspace", kind="workspace")
client.create_runtime(title="...", kind="workspace", environment="...", project="...", profile_name="...", working_git_branch="...")
client.update_runtime(uuid="...", title="New Title")
client.delete_runtime(uuid="...")
client.pause_runtime(uuid="...")
client.resume_runtime(uuid="...")
```

### Flows

```python
# List flows in a runtime
client.list_flows(runtime_uuid="...")

# Trigger a flow run
client.run_flow(runtime_uuid="...", flow_name="sales")

# Resume a paused runtime before running
client.run_flow(runtime_uuid="...", flow_name="sales", resume=True)

# Pass a spec to control behavior
client.run_flow(
    runtime_uuid="...",
    flow_name="sales",
    spec={"components": ["component_a", "component_b"]},
)
```

### Flow runs

```python
# List flow runs (returns {"items": [...], "truncated": bool})
client.list_flow_runs(runtime_uuid="...")

# Filter by status, flow name, or time range
client.list_flow_runs(runtime_uuid="...", status="running")
client.list_flow_runs(runtime_uuid="...", flow_name="sales", limit=10)
client.list_flow_runs(runtime_uuid="...", since="2025-01-01T00:00:00Z", until="2025-12-31T23:59:59Z")

# Paginate
client.list_flow_runs(runtime_uuid="...", offset=10, limit=50)

# Get a single flow run
client.get_flow_run(runtime_uuid="...", name="fr-...")
```

### Otto (AI assistant)

```python
client.list_otto_providers()
client.otto_chat(prompt="What flows are running?")
client.otto_chat(prompt="Describe the sales flow", runtime_uuid="...")
```

### Flow run spec

Pass `spec` as a dict to `run_flow` to control flow run behavior:

```python
client.run_flow(runtime_uuid="...", flow_name="sales", spec={"full_refresh": True})
client.run_flow(runtime_uuid="...", flow_name="sales", spec={"run_tests": False})
client.run_flow(runtime_uuid="...", flow_name="sales", spec={"parameters": {"key": "value"}})
```

Available spec fields: `full_refresh`, `components`, `component_categories`, `parameters`, `run_tests`, `store_test_results`, `halt_flow_on_error`, `disable_optimizers`, `update_materialization_type`, `deep_data_pruning`, `backfill_missing_statistics`, `disable_incremental_metadata_collection`, `runner_overrides`.
