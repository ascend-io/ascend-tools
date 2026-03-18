# ascend-tools-core

[![crates.io](https://img.shields.io/crates/v/ascend-tools-core.svg)](https://crates.io/crates/ascend-tools-core)

Rust SDK for the [Ascend](https://www.ascend.io) Instance web API.

This is the core library used by [`ascend-tools-cli`](https://crates.io/crates/ascend-tools-cli), [`ascend-tools-mcp`](https://crates.io/crates/ascend-tools-mcp), and the Python/JavaScript bindings. It can also be used directly as a Rust dependency.

## Install

```bash
cargo add ascend-tools-core
```

## Usage

```rust
use ascend_tools::client::AscendClient;
use ascend_tools::config::Config;

let config = Config::from_env()?;
let client = AscendClient::new(config)?;

let workspaces = client.list_workspaces(Default::default())?;
let flows = client.list_flows(&workspaces[0].uuid)?;
client.run_flow(&workspaces[0].uuid, &flows[0].name, None, false)?;
```

## Authentication

Set three environment variables (from Ascend UI > Settings > Users > Create Service Account):

```bash
export ASCEND_SERVICE_ACCOUNT_ID="asc-sa-..."
export ASCEND_SERVICE_ACCOUNT_KEY="..."
export ASCEND_INSTANCE_API_URL="https://api.instance.ascend.io"
```

The SDK handles Ed25519 JWT signing, token exchange, and caching automatically.

## API

| Method | Description |
|--------|-------------|
| `list_workspaces` | List workspaces with optional filters |
| `get_workspace` | Get a workspace by title |
| `create_workspace` | Create a new workspace |
| `update_workspace` | Update a workspace |
| `pause_workspace` / `resume_workspace` | Pause or resume a workspace |
| `delete_workspace` | Delete a workspace |
| `list_deployments` / `get_deployment` | List or get deployments |
| `create_deployment` / `update_deployment` | Create or update a deployment |
| `pause_deployment_automations` / `resume_deployment_automations` | Pause or resume automations |
| `delete_deployment` | Delete a deployment |
| `list_environments` / `get_environment` | List or get environments |
| `list_projects` / `get_project` | List or get projects |
| `list_profiles` | List available profiles |
| `list_flows` | List flows in a workspace or deployment |
| `run_flow` | Trigger a flow run (health check, optional resume/spec) |
| `list_flow_runs` / `get_flow_run` | List or get flow runs |
| `list_otto_providers` | List Otto providers and models |
| `otto` / `otto_streaming` | Chat with Otto AI assistant |

## Error handling

All methods return `ascend_tools::Result<T>` with a typed `Error` enum:

```rust
use ascend_tools::Error;

match client.run_flow(&uuid, "sales", None, false) {
    Ok(trigger) => println!("triggered: {}", trigger.event_uuid),
    Err(Error::RuntimePaused) => println!("paused — use resume=true"),
    Err(Error::NotFound { kind, title }) => println!("{kind} '{title}' not found"),
    Err(e) => println!("error: {e}"),
}
```

See the [full documentation](https://github.com/ascend-io/ascend-tools) for more details.
