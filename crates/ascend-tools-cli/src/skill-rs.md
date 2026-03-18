---
name: ascend-tools-rust
description: Use the ascend-tools Rust SDK to manage Ascend workspaces, deployments, flows, and flow runs.
---

# ascend-tools Rust SDK

Manage Ascend workspaces, deployments, flows, and flow runs from Rust via the `ascend-tools-core` crate.

## Installation

```bash
cargo add ascend-tools-core
```

The crate name is `ascend-tools-core`. The library is imported as `ascend_tools`:

```rust
use ascend_tools::client::AscendClient;
```

## Authentication

Set three environment variables (from Ascend UI > Settings > Users > Create Service Account):

```bash
export ASCEND_SERVICE_ACCOUNT_ID="asc-sa-..."
export ASCEND_SERVICE_ACCOUNT_KEY="..."
export ASCEND_INSTANCE_API_URL="https://<instance-name>.api.instance.ascend.io"
```

Or pass credentials directly:

```rust
use ascend_tools::client::AscendClient;
use ascend_tools::config::Config;

let config = Config::with_overrides(
    Some("asc-sa-..."),
    Some("..."),
    Some("https://api.instance.ascend.io"),
)?;
let client = AscendClient::new(config)?;
```

`with_overrides` falls back to environment variables for any `None` fields.

## Usage

All methods return `ascend_tools::Result<T>`.

```rust
use ascend_tools::client::AscendClient;
use ascend_tools::config::Config;

let client = AscendClient::new(Config::from_env()?)?;
```

### Environments and projects

```rust
// List environments
let envs = client.list_environments()?;

// Get by title
let env = client.get_environment("Production")?;

// List projects
let projects = client.list_projects()?;

// Get by title
let project = client.get_project("My Project")?;

// List profiles
let runtime_uuid = client.resolve_runtime_target(
    Some("My Workspace"), None, None,
)?;
let profiles = client.list_profiles(Some(&runtime_uuid), None, None)?;
```

### Workspaces

```rust
use ascend_tools::models::{RuntimeFilters, RuntimeCreate, RuntimeUpdate};

client.list_workspaces(Default::default())?;
client.list_workspaces(RuntimeFilters {
    environment: Some("Production".into()),
    ..Default::default()
})?;
client.get_workspace("My Workspace", None)?;
client.pause_workspace("My Workspace", None)?;
client.resume_workspace("My Workspace", None)?;
client.delete_workspace("My Workspace", None)?;

// Create
let create = RuntimeCreate::new("My WS", "Production", "MyProject", "default", "main");
client.create_workspace(&create)?;

// Update
let mut update = RuntimeUpdate::default();
update.git_branch = Some("feature/abc".into());
client.update_workspace("My WS", None, &update)?;
```

### Deployments

```rust
client.list_deployments(Default::default())?;
client.get_deployment("My Deployment", None)?;
client.pause_deployment_automations("My Deployment", None)?;
client.resume_deployment_automations("My Deployment", None)?;
client.delete_deployment("My Deployment", None)?;
```

### Flows

```rust
let runtime_uuid = client.resolve_runtime_target(
    Some("My Workspace"), None, None,
)?;

// List flows
let flows = client.list_flows(&runtime_uuid)?;

// Trigger a flow run
let trigger = client.run_flow(&runtime_uuid, "sales", None, false)?;

// With resume (resumes workspace/deployment if paused)
let trigger = client.run_flow(&runtime_uuid, "sales", None, true)?;

// With spec
use serde_json::json;
let spec = json!({"full_refresh": true});
let trigger = client.run_flow(&runtime_uuid, "sales", Some(spec), true)?;
```

### Flow runs

```rust
use ascend_tools::models::FlowRunFilters;

let runtime_uuid = client.resolve_runtime_target(
    Some("My Workspace"), None, None,
)?;

// List flow runs (returns FlowRunList with items and truncated)
let result = client.list_flow_runs(&runtime_uuid, Default::default())?;

// Filter by status or flow name
let result = client.list_flow_runs(&runtime_uuid, FlowRunFilters {
    status: Some("running".into()),
    flow: Some("sales".into()),
    limit: Some(10),
    ..Default::default()
})?;

// Get a single flow run
let run = client.get_flow_run(&runtime_uuid, "fr-...")?;
```

### Otto (AI assistant)

```rust
use ascend_tools::models::{OttoChatRequest, OttoModel};

client.list_otto_providers()?;

let request = OttoChatRequest {
    prompt: "What flows are running?".into(),
    runtime_uuid: None,
    thread_id: None,
    model: None,
};
let response = client.otto(&request)?;
```

### Flow run spec

Pass `spec` as `Option<serde_json::Value>` to `run_flow` to control flow run behavior:

```rust
use serde_json::json;

client.run_flow(&runtime_uuid, "sales", Some(json!({"full_refresh": true})), false)?;
client.run_flow(&runtime_uuid, "sales", Some(json!({"run_tests": false})), false)?;
client.run_flow(&runtime_uuid, "sales", Some(json!({"parameters": {"key": "value"}})), false)?;
```

Available spec fields: `full_refresh`, `components`, `component_categories`, `parameters`, `run_tests`, `store_test_results`, `halt_flow_on_error`, `disable_optimizers`, `update_materialization_type`, `deep_data_pruning`, `backfill_missing_statistics`, `disable_incremental_metadata_collection`, `runner_overrides`.

## Error handling

All methods return `ascend_tools::Result<T>` with a typed `Error` enum:

```rust
use ascend_tools::Error;

match client.run_flow(&runtime_uuid, "sales", None, false) {
    Ok(trigger) => println!("triggered: {}", trigger.event_uuid),
    Err(Error::RuntimePaused) => println!("paused — use resume=true"),
    Err(Error::NotFound { kind, title }) => println!("{kind} '{title}' not found"),
    Err(e) => println!("error: {e}"),
}
```
