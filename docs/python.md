# Use the Python SDK

Manage Ascend workspaces, deployments, flows, and flow runs from Python.

## Install

```bash
uv add ascend-tools
```

Upgrade to the latest version:

```bash
uv add --upgrade ascend-tools
```

See [Installation](INSTALLATION.md) for all install methods.

## Authenticate

### From environment variables

```python
from ascend_tools import Client

client = Client()  # reads ASCEND_SERVICE_ACCOUNT_ID, etc. from env
```

See [Quickstart](QUICKSTART.md) for the full service account creation walkthrough.

### With explicit credentials

```python
client = Client(
    service_account_id="<YOUR_SERVICE_ACCOUNT_ID>",
    service_account_key="<YOUR_SERVICE_ACCOUNT_KEY>",
    instance_api_url="<YOUR_INSTANCE_API_URL>",
)
```

All parameters are keyword-only.

## Environments and projects

### List environments

```python
environments = client.list_environments()
```

### Get an environment by title

```python
env = client.get_environment(title="Production")
```

Returns `dict` with the matching environment. Raises an error if not found or ambiguous.

### List projects

```python
projects = client.list_projects()
```

### Get a project by title

```python
project = client.get_project(title="My Project")
```

Returns `dict` with the matching project. Raises an error if not found or ambiguous.

### List profiles

```python
profiles = client.list_profiles(workspace="My Workspace")
profiles = client.list_profiles(deployment="My Deployment")
profiles = client.list_profiles(project="My Project", branch="main")
```

Returns `list[str]` of profile names. Provide exactly one of workspace/deployment/uuid, or project+branch.

## Manage workspaces and deployments

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
client.list_deployments(environment="Production")
client.get_deployment(title="My Deployment")
client.delete_deployment(title="My Deployment")
```

## Manage flows

### List flows

```python
flows = client.list_flows(workspace="My Workspace")
flows = client.list_flows(deployment="My Deployment")
```

Returns `list[dict]`, each with a `name` field.

### Run a flow

```python
result = client.run_flow(flow="sales", workspace="My Workspace")
```

Resume a paused workspace before running:

```python
result = client.run_flow(
    flow="sales",
    workspace="My Workspace",
    resume=True,
)
```

Pass a spec dict for advanced options:

```python
result = client.run_flow(
    flow="sales",
    workspace="My Workspace",
    spec={"full_refresh": True},
)
```

```python
result = client.run_flow(
    flow="sales",
    workspace="My Workspace",
    spec={
        "components": ["transform_orders", "transform_customers"],
        "parameters": {"date": "2025-01-01"},
        "run_tests": False,
    },
    resume=True,
)
```

Returns `dict` with `event_uuid` and `event_type`.

See [CLI guide](cli.md#flow-run-spec-options) for the full spec options reference.

## Monitor flow runs

### List flow runs

```python
result = client.list_flow_runs(workspace="My Workspace")
runs = result["items"]       # list[dict]
truncated = result["truncated"]  # bool
```

Filter by status, flow name, time range, or paginate:

```python
client.list_flow_runs(workspace="My Workspace", status="running")
client.list_flow_runs(deployment="My Deployment", flow="sales")
client.list_flow_runs(workspace="My Workspace", since="2025-01-01T00:00:00Z")
client.list_flow_runs(workspace="My Workspace", limit=10, offset=20)
```

### Get a flow run

```python
run = client.get_flow_run(name="fr-...", workspace="My Workspace")
```

Returns `dict` with fields: `name`, `flow`, `build_uuid`, `runtime_uuid`, `status`, `created_at`, `error`.

## Otto (AI assistant)

```python
# List providers and models
providers = client.list_otto_providers()

# Chat
response = client.otto(prompt="What flows are running?")
response = client.otto(prompt="Describe the sales flow", workspace="My Workspace")
```

## Return types

- All methods return `dict` or `list[dict]`
- All parameters are keyword-only
- Type stubs are provided (`core.pyi`) for IDE autocomplete
- The package includes a `py.typed` marker (PEP 561)

## Error handling

The SDK raises exceptions for:

- Missing configuration (environment variables not set)
- Authentication failures (invalid or expired key)
- HTTP errors (API returns non-2xx status)
- State errors (paused, starting, error state)

```python
try:
    client.run_flow(flow="sales", workspace="My Workspace")
except Exception as e:
    print(f"Error: {e}")
```
