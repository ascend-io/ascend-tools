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

### Flows

```python
# List flows in a workspace or deployment
client.list_flows(workspace="My Workspace")
client.list_flows(deployment="My Deployment")

# Trigger a flow run
client.run_flow(flow="sales", workspace="My Workspace")

# Resume a paused workspace before running
client.run_flow(flow="sales", workspace="My Workspace", resume=True)

# Pass a spec to control behavior
client.run_flow(
    flow="sales",
    workspace="My Workspace",
    spec={"components": ["component_a", "component_b"]},
)
```

### Flow runs

```python
# List flow runs (returns {"items": [...], "truncated": bool})
client.list_flow_runs(workspace="My Workspace")

# Filter by status, flow name, or time range
client.list_flow_runs(workspace="My Workspace", status="running")
client.list_flow_runs(deployment="My Deployment", flow="sales", limit=10)
client.list_flow_runs(workspace="My Workspace", since="2025-01-01T00:00:00Z", until="2025-12-31T23:59:59Z")

# Paginate
client.list_flow_runs(workspace="My Workspace", offset=10, limit=50)

# Get a single flow run
client.get_flow_run(name="fr-...", workspace="My Workspace")
```

### Flow run spec

Pass `spec` as a dict to `run_flow` to control flow run behavior:

```python
client.run_flow(flow="sales", workspace="My Workspace", spec={"full_refresh": True})
client.run_flow(flow="sales", workspace="My Workspace", spec={"run_tests": False})
client.run_flow(flow="sales", workspace="My Workspace", spec={"parameters": {"key": "value"}})
```

Available spec fields: `full_refresh`, `components`, `component_categories`, `parameters`, `run_tests`, `store_test_results`, `halt_flow_on_error`, `disable_optimizers`, `update_materialization_type`, `deep_data_pruning`, `backfill_missing_statistics`, `disable_incremental_metadata_collection`, `runner_overrides`.
