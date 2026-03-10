# ascend-tools-core

Rust SDK for the [Ascend](https://www.ascend.io) REST API.

This is the core library used by [`ascend-tools-cli`](../ascend-tools-cli) and [`ascend-tools-mcp`](../ascend-tools-mcp). It can also be used directly as a Rust dependency.

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

The SDK authenticates via Ascend service accounts using Ed25519 JWT signing. Set three environment variables:

```bash
export ASCEND_SERVICE_ACCOUNT_ID="asc-sa-..."
export ASCEND_SERVICE_ACCOUNT_KEY="..."
export ASCEND_INSTANCE_API_URL="https://api.instance.ascend.io"
```

Token exchange and caching are handled automatically.

## API

| Method | Description |
|--------|-------------|
| `list_workspaces` | List workspaces with optional filters |
| `get_workspace` | Get a workspace by title |
| `create_workspace` | Create a new workspace |
| `update_workspace` | Update a workspace |
| `pause_workspace` | Pause a workspace |
| `resume_workspace` | Resume a paused workspace |
| `delete_workspace` | Delete a workspace |
| `list_deployments` | List deployments with optional filters |
| `get_deployment` | Get a deployment by title |
| `create_deployment` | Create a new deployment |
| `update_deployment` | Update a deployment |
| `delete_deployment` | Delete a deployment |
| `list_environments` | List environments |
| `list_projects` | List projects |
| `list_profiles` | List available profiles |
| `list_flows` | List flows in a runtime |
| `run_flow` | Trigger a flow run (checks health, optional resume/spec) |
| `list_flow_runs` | List flow runs with filters |
| `get_flow_run` | Get a flow run by name |
| `list_otto_providers` | List Otto providers and models |
| `otto_chat` | Chat with Otto AI assistant |

See the [top-level README](../../../README.md) for full documentation.
