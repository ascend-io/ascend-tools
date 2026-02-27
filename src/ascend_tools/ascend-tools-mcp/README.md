# ascend-tools-mcp

[MCP](https://modelcontextprotocol.io) server for the [Ascend](https://www.ascend.io) REST API, exposing SDK methods as tools for AI assistants (Claude Code, Claude Desktop, Cursor, etc.).

Built on [`ascend-tools-core`](../ascend-tools-core) and [`rmcp`](https://crates.io/crates/rmcp).

## Transports

- **stdio** (default): communicates over stdin/stdout. Used by Claude Code and most MCP clients.
- **HTTP**: Streamable HTTP on `/mcp`. Used for remote/shared deployments.

## Tools

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

## Usage

The MCP server is typically started via the CLI:

```bash
ascend-tools mcp              # stdio
ascend-tools mcp --http       # HTTP on 127.0.0.1:8000
```

### Claude Code

```bash
claude mcp add --transport stdio ascend-tools -- uvx --from ./ascend-tools ascend-tools mcp
```

### Codex CLI

```bash
codex mcp add ascend-tools -- uvx --from "$(pwd)" ascend-tools mcp
```

See the [top-level README](../../../README.md) for full documentation.
