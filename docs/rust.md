# Use the Rust SDK

> The Rust SDK (`ascend-tools-core`) is not yet published on crates.io. Contact your Ascend representative if you're interested in using the Rust SDK directly.

Manage Ascend runtimes, flows, and flow runs from Rust.

## Install

```bash
cargo add ascend-tools-core
```

The crate name is `ascend-tools-core`. The library is imported as `ascend_tools`:

```rust
use ascend_tools::client::AscendClient;
```

## Authenticate

### From environment variables

```rust
use ascend_tools::client::AscendClient;
use ascend_tools::config::Config;

let config = Config::from_env()?;
let client = AscendClient::new(config)?;
```

See [Quickstart](QUICKSTART.md) for the full service account creation walkthrough.

### With explicit credentials

```rust
let config = Config::with_overrides(
    Some("<YOUR_SERVICE_ACCOUNT_ID>"),
    Some("<YOUR_SERVICE_ACCOUNT_KEY>"),
    Some("<YOUR_INSTANCE_API_URL>"),
)?;
let client = AscendClient::new(config)?;
```

`with_overrides` falls back to environment variables for any `None` fields.

## Manage runtimes

### List runtimes

```rust
use ascend_tools::models::RuntimeFilters;

let runtimes = client.list_runtimes(Default::default())?;

// With filters
let runtimes = client.list_runtimes(RuntimeFilters {
    kind: Some("deployment".into()),
    ..Default::default()
})?;
```

Returns `Vec<Runtime>`.

### Get a runtime

```rust
let runtime = client.get_runtime("<RUNTIME_UUID>")?;
println!("{} ({})", runtime.id, runtime.uuid);
```

### Pause and resume

```rust
client.pause_runtime("<RUNTIME_UUID>")?;
client.resume_runtime("<RUNTIME_UUID>")?;
```

## Manage flows

### List flows

```rust
let flows = client.list_flows("<RUNTIME_UUID>")?;
for flow in &flows {
    println!("{}", flow.name);
}
```

Returns `Vec<Flow>`.

### Run a flow

```rust
use serde_json::json;

// Basic
let trigger = client.run_flow("<RUNTIME_UUID>", "<FLOW_NAME>", None, false)?;

// With resume
let trigger = client.run_flow("<RUNTIME_UUID>", "<FLOW_NAME>", None, true)?;

// With spec
let spec = json!({"full_refresh": true});
let trigger = client.run_flow("<RUNTIME_UUID>", "<FLOW_NAME>", Some(spec), true)?;

println!("event_uuid: {}", trigger.event_uuid);
```

The `spec` parameter is `Option<serde_json::Value>`. See [CLI guide](cli.md#flow-run-spec-options) for the full spec options reference.

The SDK automatically checks runtime health before submitting and returns typed errors for paused, starting, or error states.

## Monitor flow runs

### List flow runs

```rust
use ascend_tools::models::FlowRunFilters;

let result = client.list_flow_runs("<RUNTIME_UUID>", Default::default())?;
for run in &result.items {
    println!("{}: {}", run.name, run.status);
}

// With filters
let result = client.list_flow_runs("<RUNTIME_UUID>", FlowRunFilters {
    status: Some("running".into()),
    limit: Some(10),
    ..Default::default()
})?;
```

Returns `FlowRunList` with `items: Vec<FlowRun>` and `truncated: bool`.

### Get a flow run

```rust
let run = client.get_flow_run("<RUNTIME_UUID>", "fr-...")?;
println!("{}: {} ({})", run.name, run.status, run.flow);
```

## Types

| Type | Fields |
|------|--------|
| `Runtime` | `uuid`, `id`, `title`, `kind`, `project_uuid`, `environment_uuid`, `build_uuid`, `created_at`, `updated_at`, `health`, `paused` |
| `Flow` | `name` |
| `FlowRun` | `name`, `flow`, `build_uuid`, `runtime_uuid`, `status`, `created_at`, `error` |
| `FlowRunList` | `items`, `truncated` |
| `FlowRunTrigger` | `event_uuid`, `event_type` |
| `RuntimeFilters` | `id`, `kind`, `project_uuid`, `environment_uuid` |
| `FlowRunFilters` | `status`, `flow`, `since`, `until`, `offset`, `limit` |

All filter structs are `#[non_exhaustive]` and implement `Default`. Use `..Default::default()` when constructing.

## Error handling

All methods return `ascend_tools::Result<T>`. The error type is a typed enum:

```rust
use ascend_tools::Error;

match client.run_flow(uuid, flow, None, false) {
    Ok(trigger) => println!("triggered: {}", trigger.event_uuid),
    Err(Error::RuntimePaused) => println!("runtime is paused, use resume=true"),
    Err(Error::RuntimeStarting) => println!("runtime is still starting"),
    Err(Error::ApiError { status, message }) => println!("API error {status}: {message}"),
    Err(e) => println!("error: {e}"),
}
```

Key error variants:

| Variant | Description |
|---------|-------------|
| `MissingConfig` | Required env var or flag not set |
| `ApiError` | HTTP error from the Ascend API |
| `RuntimePaused` | Runtime is paused; use `resume=true` |
| `RuntimeStarting` | Runtime is starting, not yet ready |
| `RuntimeInErrorState` | Runtime is in error state |
| `RuntimeHealthMissing` | Runtime has no health status (may be initializing) |
