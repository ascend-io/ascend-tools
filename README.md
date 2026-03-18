# ascend-tools

CLI, SDK, and MCP server for the Ascend Instance web API.

[![PyPI](https://img.shields.io/pypi/v/ascend-tools?color=8A2BE2)](https://pypi.org/project/ascend-tools/)
[![npm](https://img.shields.io/npm/v/ascend-tools?color=8A2BE2)](https://www.npmjs.com/package/ascend-tools)
[![crates.io](https://img.shields.io/crates/v/ascend-tools-core?color=8A2BE2)](https://crates.io/crates/ascend-tools-core)
[![CI](https://img.shields.io/github/actions/workflow/status/ascend-io/ascend-tools/ci.yml?branch=main&label=CI)](https://github.com/ascend-io/ascend-tools/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/license-MIT-8A2BE2.svg)](https://github.com/ascend-io/ascend-tools/blob/main/LICENSE)

## Authentication

Set three environment variables (from Ascend UI > Settings > Users > Create Service Account):

```bash
export ASCEND_SERVICE_ACCOUNT_ID="asc-sa-..."
export ASCEND_SERVICE_ACCOUNT_KEY="..."
export ASCEND_INSTANCE_API_URL="https://<instance-name>.api.instance.ascend.io"
```

## CLI

Install the CLI:

```bash
uv tool install ascend-tools
```

```bash
npm install -g ascend-tools
```

```bash
cargo install ascend-tools-cli
```

Or run without installing:

```bash
uvx ascend-tools workspace list
```

```bash
npx ascend-tools workspace list
```

### Examples

```bash
ascend-tools workspace list
ascend-tools workspace get "My Workspace"
ascend-tools workspace pause "My Workspace"
ascend-tools flow list --workspace "My Workspace"
ascend-tools flow run <FLOW_NAME> --workspace "My Workspace"
ascend-tools -o json workspace list
```

### Interactive TUI

```bash
ascend-tools otto tui
ascend-tools otto tui --workspace "My Workspace"
```

Vi keybindings by default. Type `/help` for commands, `/emacs` to switch modes.

## Python SDK

```bash
uv add ascend-tools
```

```python
from ascend_tools import Client

client = Client()
client.list_workspaces()
client.run_flow(flow="sales", workspace="My Workspace")
```

## JavaScript SDK

```bash
npm add ascend-tools
```

```javascript
import { Client } from "ascend-tools";

const client = new Client();
const workspaces = await client.listWorkspaces();
await client.runFlow("sales", "My Workspace");
```

## Rust SDK

```bash
cargo add ascend-tools-core
```

```rust
use ascend_tools::client::AscendClient;
use ascend_tools::config::Config;

let client = AscendClient::new(Config::from_env()?)?;
let workspaces = client.list_workspaces(Default::default())?;
```

## MCP server

Connect AI assistants (Claude Code, Cursor, etc.) to Ascend:

```bash
claude mcp add --transport stdio ascend-tools-dev -- uvx ascend-tools mcp
```

```bash
claude mcp add --transport stdio ascend-tools-dev -- npx ascend-tools mcp
```

See [MCP server guide](docs/mcp.md) for Codex CLI setup and the full tools reference.

## Skills

Install reference skills for AI coding assistants:

```bash
ascend-tools skill install --target .claude/skills --all
```

Available flags: `--cli` (default), `--python`, `--javascript`, `--mcp`, `--all`.

## Development

```bash
bin/setup
bin/check
bin/build
bin/format
```

See [Development guide](docs/development.md) for the full contributor setup.

## Documentation

- [Quickstart](docs/QUICKSTART.md) -- create a service account, install, and run your first flow
- [Installation](docs/INSTALLATION.md) -- all install methods
- [CLI](docs/cli.md) -- all commands with examples
- [Python SDK](docs/python.md) -- Client methods, return types, error handling
- [JavaScript SDK](docs/javascript.md) -- async Client methods, streaming, TypeScript types
- [Rust SDK](docs/rust.md) -- typed client with structs and error handling
- [MCP server](docs/mcp.md) -- set up AI assistants with Ascend tools
- [Development](docs/development.md) -- contributor setup, architecture, release process
