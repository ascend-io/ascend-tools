# ascend-tools

CLI, SDK, and MCP server for the Ascend REST API.

> Private preview: `ascend-tools` is currently in private preview. Contact your Ascend representative to request access via Service Accounts on your Instance.

## Install

Pre-built binaries are available for Linux and macOS. Windows users should use Linux/macOS.

```bash
uv tool install ascend-tools
```

## Authentication

Set three environment variables (from Ascend UI > Settings > Users > Create Service Account):

```bash
export ASCEND_SERVICE_ACCOUNT_ID="asc-sa-..."
export ASCEND_SERVICE_ACCOUNT_KEY="..."
export ASCEND_INSTANCE_API_URL="https://<instance-name>.api.instance.ascend.io"
```

## CLI

List workspaces:

```bash
ascend-tools workspace list
```

Get a workspace:

```bash
ascend-tools workspace get "My Workspace"
```

Pause / resume a workspace:

```bash
ascend-tools workspace pause "My Workspace"
ascend-tools workspace resume "My Workspace"
```

List and run flows:

```bash
ascend-tools flow list --workspace "My Workspace"
ascend-tools flow run <FLOW_NAME> --workspace "My Workspace"
ascend-tools flow run <FLOW_NAME> --workspace "My Workspace" --resume
ascend-tools flow run <FLOW_NAME> --workspace "My Workspace" --spec '{"full_refresh": true}'
```

Deployments work the same way:

```bash
ascend-tools deployment list
ascend-tools deployment get "My Deployment"
ascend-tools flow list --deployment "My Deployment"
```

JSON output:

```bash
ascend-tools -o json workspace list
```

## Python SDK

```python
from ascend_tools import Client

client = Client()  # reads from env vars
client.list_workspaces()
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
claude mcp add --transport stdio ascend-tools-dev -- uvx ascend-tools mcp
```

The Ascend auth env vars are inherited from your shell. Verify with `/mcp` inside Claude Code.
If Claude is launched without your shell env, add vars explicitly:

```bash
claude mcp add --transport stdio ascend-tools-dev \
  -e ASCEND_SERVICE_ACCOUNT_ID="$ASCEND_SERVICE_ACCOUNT_ID" \
  -e ASCEND_SERVICE_ACCOUNT_KEY="$ASCEND_SERVICE_ACCOUNT_KEY" \
  -e ASCEND_INSTANCE_API_URL="$ASCEND_INSTANCE_API_URL" \
  -- uvx ascend-tools mcp
```

### Codex CLI setup

```bash
codex mcp add ascend-tools-dev -- uvx ascend-tools mcp
```

If Codex is launched without your shell env, add vars explicitly:

```bash
codex mcp add \
  --env "ASCEND_SERVICE_ACCOUNT_ID=$ASCEND_SERVICE_ACCOUNT_ID" \
  --env "ASCEND_SERVICE_ACCOUNT_KEY=$ASCEND_SERVICE_ACCOUNT_KEY" \
  --env "ASCEND_INSTANCE_API_URL=$ASCEND_INSTANCE_API_URL" \
  ascend-tools-dev -- uvx ascend-tools mcp
```

Inspect the MCP server config:

```bash
codex mcp get ascend-tools-dev --json
```

List all configured MCP servers:

```bash
codex mcp list
```

Remove the config:

```bash
codex mcp remove ascend-tools-dev
```

The Ascend auth env vars are inherited from your shell when Codex launches the server.
If you have stale MCP behavior after updating code, run this once to refresh the uvx cache:

```bash
uvx --refresh ascend-tools --version
```

If Codex MCP startup fails with `connection closed: initialize response`, refresh once and re-add:

```bash
uvx --refresh ascend-tools --version
```

### Tools

| Tool | Description |
|------|-------------|
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
| `pause_deployment_automations` | Pause automations on a deployment |
| `resume_deployment_automations` | Resume automations on a deployment |
| `delete_deployment` | Delete a deployment |
| `list_environments` | List environments |
| `list_projects` | List projects |
| `list_profiles` | List profiles for a workspace, deployment, or project+branch |
| `list_flows` | List flows in a workspace or deployment |
| `run_flow` | Trigger a flow run |
| `list_flow_runs` | List flow runs with filters |
| `get_flow_run` | Get a flow run by name |
| `list_otto_providers` | List Otto providers and models |
| `otto_chat` | Chat with Otto AI assistant |

## Skills

Install reference skills for AI coding assistants (Claude Code, Codex, etc.):

```bash
ascend-tools skill install --target .claude/skills --all
```

Available skills: `--cli` (default), `--python`, `--mcp`, `--all`.
