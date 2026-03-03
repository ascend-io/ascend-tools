# Get started with ascend-tools

> **Private preview**: ascend-tools is currently in private preview. Contact your Ascend representative to request access via service accounts on your Instance.

ascend-tools provides a CLI, Python SDK, Rust SDK, and MCP server for the Ascend REST API. This guide walks you through authentication and your first commands.

## Prerequisites

- An Ascend Instance with permission to create service accounts
- [uv](https://docs.astral.sh/uv/) (installed in the next section)

## Create a service account

### 1. Open the service accounts page

Navigate to **Settings > Users** in your Ascend Instance. Click **+ Create service account**.

![Settings > Users page with the "+ Create service account" button](https://storage.googleapis.com/docs-ascend-io/images/service-account-create.png)

### 2. Name your service account

Enter a name (e.g., `ascend-tools`) and click **Create service account**.

![Create service account dialog with name input](https://storage.googleapis.com/docs-ascend-io/images/service-account-name.png)

### 3. Copy your credentials

The confirmation dialog shows three values. Copy each one and store them securely — the private key is shown only once.

![Service account created dialog showing credentials and environment variables](https://storage.googleapis.com/docs-ascend-io/images/service-account-details.png)

## Set environment variables

Export the three values from the previous step:

```bash
export ASCEND_SERVICE_ACCOUNT_ID="<YOUR_SERVICE_ACCOUNT_ID>"
export ASCEND_SERVICE_ACCOUNT_KEY="<YOUR_SERVICE_ACCOUNT_KEY>"
export ASCEND_INSTANCE_API_URL="<YOUR_INSTANCE_API_URL>"
```

These are the only credentials you need. The SDK handles JWT signing, token exchange, and caching automatically.

Add these to your shell profile (`~/.zshrc` or `~/.bashrc`) so they persist across sessions.

## Install

Install [uv](https://docs.astral.sh/uv/) (if you don't have it):

```bash
curl -LsSf https://astral.sh/uv/install.sh | sh
```

Install ascend-tools:

```bash
uv tool install ascend-tools
```

See [Installation](INSTALLATION.md) for other methods (Cargo, pre-built binaries).

## Verify your setup

```bash
ascend-tools runtime list
```

You should see a table of runtimes in your Instance.

## Run your first flow

List available flows in a runtime, then trigger a flow run:

```bash
ascend-tools flow list --runtime <RUNTIME_UUID>
ascend-tools flow run <FLOW_NAME> --runtime <RUNTIME_UUID>
```

Or from Python:

```python
from ascend_tools import Client

client = Client()
flows = client.list_flows(runtime_uuid="<RUNTIME_UUID>")
result = client.run_flow(runtime_uuid="<RUNTIME_UUID>", flow_name="<FLOW_NAME>")
```

## Next steps

- [CLI guide](cli.md) -- all commands with examples
- [Python SDK guide](python.md) -- Client methods, return types, error handling
- [Rust SDK guide](rust.md) -- typed client with structs and error handling
- [MCP server guide](mcp.md) -- set up AI assistants with Ascend tools
- [Installation](INSTALLATION.md) -- all install methods (Cargo, pre-built binaries)
