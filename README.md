# ascend-tools

CLI, SDK, and MCP server for the Ascend Instance web API.

[![PyPI](https://img.shields.io/pypi/v/ascend-tools)](https://pypi.org/project/ascend-tools/)
[![npm](https://img.shields.io/npm/v/ascend-tools)](https://www.npmjs.com/package/ascend-tools)
[![crates.io](https://img.shields.io/crates/v/ascend-tools-core)](https://crates.io/crates/ascend-tools-core)
[![CI](https://img.shields.io/github/actions/workflow/status/ascend-io/ascend-tools/ci.yml?branch=main&label=CI)](https://github.com/ascend-io/ascend-tools/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](https://github.com/ascend-io/ascend-tools/blob/main/LICENSE)

## Install

```bash
uv tool install ascend-tools    # Python
npm install -g ascend-tools     # Node.js
cargo install ascend-tools-cli  # Rust
```

See [Installation](docs/INSTALLATION.md) for all methods (pre-built binaries, `uvx`, `npx`, etc.).

## Authentication

Set three environment variables (from Ascend UI > Settings > Users > Create Service Account):

```bash
export ASCEND_SERVICE_ACCOUNT_ID="asc-sa-..."
export ASCEND_SERVICE_ACCOUNT_KEY="..."
export ASCEND_INSTANCE_API_URL="https://<instance-name>.api.instance.ascend.io"
```

## CLI

```bash
ascend-tools workspace list
ascend-tools workspace get "My Workspace"
ascend-tools workspace pause "My Workspace"
ascend-tools flow list --workspace "My Workspace"
ascend-tools flow run <FLOW_NAME> --workspace "My Workspace"
ascend-tools -o json workspace list
```

Run without installing:

```bash
uvx ascend-tools workspace list     # Python
npx ascend-tools workspace list     # Node.js
```

## Interactive TUI

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

client = Client()  # reads from env vars
client.list_workspaces()
client.run_flow(flow="sales", workspace="My Workspace")
```

## JavaScript SDK

```bash
npm add ascend-tools
```

```javascript
import { Client } from "ascend-tools";

const client = new Client(); // reads from env vars
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
claude mcp add --transport stdio ascend-tools-dev -- uvx ascend-tools mcp    # via uv
claude mcp add --transport stdio ascend-tools-dev -- npx ascend-tools mcp    # via npm
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
bin/setup       # install toolchain (Rust, uv, npm deps)
bin/check       # lint + test (Rust, Python, JS)
bin/build       # build all (Rust, Python, JS)
bin/format      # auto-format (Rust, Python)
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
