# ascend-tools-mcp

[MCP](https://modelcontextprotocol.io) server for the [Ascend](https://www.ascend.io) REST API, exposing SDK methods as tools for AI assistants (Claude Code, Claude Desktop, Cursor, etc.).

Built on [`ascend-tools-core`](../ascend-tools-core) and [`rmcp`](https://crates.io/crates/rmcp).

## Transports

- **stdio** (default): communicates over stdin/stdout. Used by Claude Code and most MCP clients.
- **HTTP**: Streamable HTTP on `/mcp`. Used for remote/shared deployments.

## Tools

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

## Usage

The MCP server is typically started via the CLI:

```bash
ascend-tools mcp              # stdio
ascend-tools mcp --http       # HTTP on 127.0.0.1:8000
```

### Claude Code

```bash
claude mcp add --transport stdio ascend-tools-dev -- uvx ascend-tools mcp
```

### Codex CLI

```bash
codex mcp add ascend-tools-dev -- uvx ascend-tools mcp
```

See the [top-level README](../../../README.md) for full documentation.
