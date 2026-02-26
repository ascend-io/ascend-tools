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

Resume a runtime:

```bash
ascend-tools runtime resume <UUID>
```

Pause a runtime:

```bash
ascend-tools runtime pause <UUID>
```

List flows:

```bash
ascend-tools flow list --runtime <UUID>
```

Run a flow:

```bash
ascend-tools flow run <FLOW_NAME> --runtime <UUID>
```

Run a flow and resume the runtime first if paused:

```bash
ascend-tools flow run <FLOW_NAME> --runtime <UUID> --resume
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
claude mcp add --transport stdio ascend-tools -- uvx --refresh --from ./ascend-tools ascend-tools mcp
```

The Ascend auth env vars are inherited from your shell. Verify with `/mcp` inside Claude Code.
If Claude is launched without your shell env, add vars explicitly:

```bash
claude mcp add --transport stdio \
  -e ASCEND_SERVICE_ACCOUNT_ID="$ASCEND_SERVICE_ACCOUNT_ID" \
  -e ASCEND_SERVICE_ACCOUNT_KEY="$ASCEND_SERVICE_ACCOUNT_KEY" \
  -e ASCEND_INSTANCE_API_URL="$ASCEND_INSTANCE_API_URL" \
  ascend-tools -- uvx --refresh --from ./ascend-tools ascend-tools mcp
```

### Codex CLI setup

```bash
codex mcp add ascend-tools -- uvx --refresh --from "$(pwd)" ascend-tools mcp
```

If Codex is launched without your shell env, add vars explicitly:

```bash
codex mcp add \
  --env "ASCEND_SERVICE_ACCOUNT_ID=$ASCEND_SERVICE_ACCOUNT_ID" \
  --env "ASCEND_SERVICE_ACCOUNT_KEY=$ASCEND_SERVICE_ACCOUNT_KEY" \
  --env "ASCEND_INSTANCE_API_URL=$ASCEND_INSTANCE_API_URL" \
  ascend-tools -- uvx --refresh --from "$(pwd)" ascend-tools mcp
```

Inspect the MCP server config:

```bash
codex mcp get ascend-tools --json
```

List all configured MCP servers:

```bash
codex mcp list
```

Remove the config:

```bash
codex mcp remove ascend-tools
```

The Ascend auth env vars are inherited from your shell when Codex launches the server.

### Tools

| Tool | Description |
|------|-------------|
| `list_runtimes` | List runtimes with optional filters |
| `get_runtime` | Get a runtime by UUID |
| `resume_runtime` | Resume a paused runtime |
| `pause_runtime` | Pause a running runtime |
| `list_flows` | List flows in a runtime |
| `run_flow` | Trigger a flow run (supports resume, full_refresh, components, parameters, etc.) |
| `list_flow_runs` | List flow runs with filters |
| `get_flow_run` | Get a flow run by name |
