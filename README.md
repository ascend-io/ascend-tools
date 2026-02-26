# ascend-tools

SDK, CLI, and MCP server for the Ascend REST API.

## Install

```bash
uv tool install ascend-tools
```

## Authentication

Set three environment variables (from Ascend UI > Settings > Users > Create Service Account):

```bash
export ASCEND_SERVICE_ACCOUNT_ID="asc-sa-..."
```

```bash
export ASCEND_SERVICE_ACCOUNT_KEY="..."
```

```bash
export ASCEND_INSTANCE_API_URL="https://api.instance.ascend.io"
```

## CLI

List runtimes:

```bash
ascend-tools runtime list
```

Get a runtime:

```bash
ascend-tools runtime get <UUID>
```

List flows:

```bash
ascend-tools flow list --runtime <UUID>
```

Run a flow:

```bash
ascend-tools flow run <FLOW_NAME> --runtime <UUID>
```

Run a flow with full refresh:

```bash
ascend-tools flow run <FLOW_NAME> --runtime <UUID> --spec '{"full_refresh": true}'
```

List flow runs:

```bash
ascend-tools flow list-runs --runtime <UUID>
```

Get a flow run:

```bash
ascend-tools flow get-run <RUN_NAME> --runtime <UUID>
```

JSON output:

```bash
ascend-tools -o json runtime list
```

## Python SDK

```python
from ascend_tools import Client

client = Client()  # reads from env vars
client.list_runtimes()
client.run_flow(runtime_uuid="...", flow_name="sales")
```

## MCP server

Start an MCP server for AI assistants (Claude Code, Claude Desktop, Cursor, etc.).

Stdio transport (default):

```bash
ascend-tools mcp
```

HTTP transport:

```bash
ascend-tools mcp --http --bind 127.0.0.1:8000
```

### Claude Code setup

```bash
claude mcp add --transport stdio ascend-tools -- uvx --from ./ascend-tools ascend-tools mcp
```

The Ascend auth env vars are inherited from your shell. Verify with `/mcp` inside Claude Code.

### Tools

| Tool | Description |
|------|-------------|
| `list_runtimes` | List runtimes with optional filters |
| `get_runtime` | Get a runtime by UUID |
| `list_flows` | List flows in a runtime |
| `run_flow` | Trigger a flow run (supports full_refresh, components, parameters, etc.) |
| `list_flow_runs` | List flow runs with filters |
| `get_flow_run` | Get a flow run by name |
