# Use the Python SDK

Manage Ascend workspaces, deployments, flows, and flow runs from Python.

## Install

Requires [uv](https://docs.astral.sh/uv/) (see [Installation](INSTALLATION.md) for setup).

```bash
uv add ascend-tools
```

Upgrade to the latest version:

```bash
uv add --upgrade ascend-tools
```

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

### Resolve an environment by title

```python
env = client.resolve_environment(title="Production")
```

Returns `dict` with the matching environment. Raises an error if not found or ambiguous.

### List projects

```python
projects = client.list_projects()
```

### Resolve a project by title

```python
project = client.resolve_project(title="My Project")
```

Returns `dict` with the matching project. Raises an error if not found or ambiguous.

## Manage runtimes

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

### List runtimes (low-level)

```python
runtimes = client.list_runtimes()
client.list_runtimes(kind="deployment", project_uuid="...", environment_uuid="...")
```

Returns `list[dict]`.

### Get a runtime (low-level)

```python
runtime = client.get_runtime(uuid="<RUNTIME_UUID>")
```

Returns `dict` with fields: `uuid`, `id`, `title`, `kind`, `project_uuid`, `environment_uuid`, `build_uuid`, `created_at`, `updated_at`, `health`, `paused`.

### Resolve a runtime by title

```python
runtime = client.resolve_runtime_by_title(title="My Workspace", kind="workspace")
runtime = client.resolve_runtime_by_title(title="My Deployment", kind="deployment")
```

### Create a runtime

```python
runtime = client.create_runtime(
    environment_uuid="...",
    project_uuid="...",
    build_uuid="...",
    title="My Workspace",
    kind="workspace",
)
```

### Update a runtime

```python
client.update_runtime(uuid="...", title="New Title")
client.update_runtime(uuid="...", build_uuid="...", paused=True)
```

### Delete a runtime

```python
client.delete_runtime(uuid="...")
```

### Pause and resume

```python
client.pause_runtime(uuid="<RUNTIME_UUID>")
client.resume_runtime(uuid="<RUNTIME_UUID>")
```

## Manage flows

### List flows

```python
flows = client.list_flows(runtime_uuid="<RUNTIME_UUID>")
```

Returns `list[dict]`, each with a `name` field.

### Run a flow

```python
result = client.run_flow(runtime_uuid="<RUNTIME_UUID>", flow_name="sales")
```

Resume a paused runtime before running:

```python
result = client.run_flow(
    runtime_uuid="<RUNTIME_UUID>",
    flow_name="sales",
    resume=True,
)
```

Pass a spec dict for advanced options:

```python
result = client.run_flow(
    runtime_uuid="<RUNTIME_UUID>",
    flow_name="sales",
    spec={"full_refresh": True},
)
```

```python
result = client.run_flow(
    runtime_uuid="<RUNTIME_UUID>",
    flow_name="sales",
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
result = client.list_flow_runs(runtime_uuid="<RUNTIME_UUID>")
runs = result["items"]       # list[dict]
truncated = result["truncated"]  # bool
```

Filter by status, flow name, time range, or paginate:

```python
client.list_flow_runs(runtime_uuid="...", status="running")
client.list_flow_runs(runtime_uuid="...", flow_name="sales")
client.list_flow_runs(runtime_uuid="...", since="2025-01-01T00:00:00Z")
client.list_flow_runs(runtime_uuid="...", limit=10, offset=20)
```

### Get a flow run

```python
run = client.get_flow_run(runtime_uuid="<RUNTIME_UUID>", name="fr-...")
```

Returns `dict` with fields: `name`, `flow`, `build_uuid`, `runtime_uuid`, `status`, `created_at`, `error`.

## Otto (AI assistant)

```python
# List providers and models
providers = client.list_otto_providers()

# Chat
response = client.otto_chat(prompt="What flows are running?")
response = client.otto_chat(prompt="Describe the sales flow", runtime_uuid="...")
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
- Runtime state errors (paused, starting, error state)

```python
try:
    client.run_flow(runtime_uuid="...", flow_name="sales")
except Exception as e:
    print(f"Error: {e}")
```
