# ascend-tools-mcp

[![crates.io](https://img.shields.io/crates/v/ascend-tools-mcp.svg)](https://crates.io/crates/ascend-tools-mcp)

[MCP](https://modelcontextprotocol.io) server for the [Ascend](https://www.ascend.io) Instance web API, exposing SDK methods as tools for AI assistants (Claude Code, Claude Desktop, Cursor, etc.).

Built on [`ascend-tools-core`](https://crates.io/crates/ascend-tools-core) and [`rmcp`](https://crates.io/crates/rmcp).

## Transports

- **stdio** (default): communicates over stdin/stdout. Used by Claude Code and most MCP clients.
- **HTTP**: Streamable HTTP on `/mcp`. Used for remote/shared deployments.

## Usage

The MCP server is typically started via the CLI:

```bash
ascend-tools mcp              # stdio
ascend-tools mcp --http       # HTTP on 127.0.0.1:8000
```

### Claude Code

```bash
claude mcp add --transport stdio ascend-tools-dev -- uvx ascend-tools mcp    # via uv
claude mcp add --transport stdio ascend-tools-dev -- npx ascend-tools mcp    # via npm
```

### Codex CLI

```bash
codex mcp add ascend-tools-dev -- uvx ascend-tools mcp
```

## Tools

25 tools covering workspaces, deployments, environments, projects, profiles, flows, flow runs, and Otto.

| Category | Tools |
|----------|-------|
| Workspaces | list, get, create, update, pause, resume, delete |
| Deployments | list, get, create, update, pause/resume automations, delete |
| Resources | list/get environments, list/get projects, list profiles |
| Flows | list, run, list runs, get run |
| Otto | list providers, chat |

See the [full documentation](https://github.com/ascend-io/ascend-tools) for the complete tools reference.
