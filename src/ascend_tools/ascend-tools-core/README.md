# ascend-tools-core

Rust SDK for the [Ascend](https://www.ascend.io) REST API.

This is the core library used by [`ascend-tools-cli`](../ascend-tools-cli) and [`ascend-tools-mcp`](../ascend-tools-mcp). It can also be used directly as a Rust dependency.

## Usage

```rust
use ascend_tools::client::AscendClient;
use ascend_tools::config::Config;

let config = Config::from_env()?;
let client = AscendClient::new(config)?;

let runtimes = client.list_runtimes(Default::default())?;
let flows = client.list_flows(&runtimes[0].uuid)?;
client.run_flow(&runtimes[0].uuid, &flows[0].name, None, false)?;
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
| `list_runtimes` | List runtimes with optional filters |
| `get_runtime` | Get a runtime by UUID |
| `resume_runtime` | Resume a paused runtime |
| `pause_runtime` | Pause a running runtime |
| `list_flows` | List flows in a runtime |
| `run_flow` | Trigger a flow run (checks health, optional resume/spec) |
| `list_flow_runs` | List flow runs with filters |
| `get_flow_run` | Get a flow run by name |

See the [top-level README](../../../README.md) for full documentation.
