# Get started with ascend-tools

ascend-tools provides a CLI, Python SDK, JavaScript SDK, Rust SDK, and MCP server for the Ascend Instance web API. This guide walks you through authentication and your first commands.

## Prerequisites

- An Ascend Instance with permission to create service accounts
- [uv](https://docs.astral.sh/uv/) or [Node.js](https://nodejs.org/)

## Create a service account

### 1. Open the service accounts page

Navigate to **Settings > Users** in your Ascend Instance. Click **+ Create service account**.

![Settings > Users page with the "+ Create service account" button](https://storage.googleapis.com/docs-ascend-io/images/service-account-create.png)

### 2. Name your service account

Enter a name (e.g., `ascend-tools`) and click **Create service account**.

![Create service account dialog with name input](https://storage.googleapis.com/docs-ascend-io/images/service-account-name.png)

### 3. Copy your credentials

The confirmation dialog shows three values. Copy each one and store them securely -- the private key is shown only once.

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

Install with uv (Python):

```bash
curl -LsSf https://astral.sh/uv/install.sh | sh
uv tool install ascend-tools
```

Or install with npm (Node.js):

```bash
npm install -g ascend-tools
```

See [Installation](INSTALLATION.md) for other methods (Cargo, pre-built binaries).

## Verify your setup

```bash
ascend-tools workspace list
```

You should see a table of workspaces in your Instance.

## Run your first flow

List available flows in a workspace, then trigger a flow run:

```bash
ascend-tools flow list --workspace "My Workspace"
ascend-tools flow run <FLOW_NAME> --workspace "My Workspace"
```

From Python:

```python
from ascend_tools import Client

client = Client()
workspaces = client.list_workspaces()
flows = client.list_flows(workspace=workspaces[0]["title"])
result = client.run_flow(flow=flows[0]["name"], workspace=workspaces[0]["title"])
```

From JavaScript:

```javascript
import { Client } from "ascend-tools";

const client = new Client();
const workspaces = await client.listWorkspaces();
const flows = await client.listFlows(workspaces[0].title);
const result = await client.runFlow(flows[0].name, workspaces[0].title);
```

## Next steps

- [CLI guide](cli.md) -- all commands with examples
- [Python SDK guide](python.md) -- Client methods, return types, error handling
- [JavaScript SDK guide](javascript.md) -- async Client, streaming, TypeScript types
- [Rust SDK guide](rust.md) -- typed client with structs and error handling
- [MCP server guide](mcp.md) -- set up AI assistants with Ascend tools
- [Installation](INSTALLATION.md) -- all install methods
