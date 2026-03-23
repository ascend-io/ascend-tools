# ascend-tools

[![GitHub Release](https://img.shields.io/github/v/release/ascend-io/ascend-tools?color=blue)](https://github.com/ascend-io/ascend-tools/releases)
[![PyPI](https://img.shields.io/pypi/v/ascend-tools?color=blue)](https://pypi.org/project/ascend-tools/)
[![npm](https://img.shields.io/npm/v/ascend-tools?color=blue)](https://www.npmjs.com/package/ascend-tools)
[![crates.io](https://img.shields.io/crates/v/ascend-tools-core?color=blue)](https://crates.io/crates/ascend-tools-core)
[![CI](https://img.shields.io/github/actions/workflow/status/ascend-io/ascend-tools/ci.yml?branch=main&label=CI)](https://github.com/ascend-io/ascend-tools/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/license-MIT-8A2BE2.svg)](https://github.com/ascend-io/ascend-tools/blob/main/LICENSE)

CLI, SDK, and MCP server for the Ascend Instance web API.

## Getting started

Don't have an Ascend Instance yet? Sign up:

```bash
ascend-tools signup
```

## Authentication

To authenticate, you need to create a Service Account and set three environment variables including the secret key (Settings > Users > Service Accounts > Create Service Account):

```bash
export ASCEND_SERVICE_ACCOUNT_ID="asc-sa-..."
export ASCEND_SERVICE_ACCOUNT_KEY="..."
export ASCEND_INSTANCE_API_URL="https://<instance-name>.api.instance.ascend.io"
```

## CLI

Install via [GitHub releases](https://github.com/ascend-io/ascend-tools/releases), [PyPI](https://pypi.org/project/ascend-tools/), [npm](https://www.npmjs.com/package/ascend-tools), [crates.io](https://crates.io/crates/ascend-tools-cli), or [source](docs/DEVELOPMENT.md).

Python/uv:

```bash
uv tool install ascend-tools
```

Node.js/npm:

```bash
npm install -g ascend-tools
```

Rust/cargo:

```bash
cargo install ascend-tools-cli
```

Without "installing", you can try `ascend-tools` via `uvx`:

```bash
uvx ascend-tools workspace list
```

Or `npx`:

```bash
npx ascend-tools workspace list
```

See [docs](docs/cli.md) for more details.

### Interactive TUI

Run Otto in an interactive terminal user interface (TUI):

```bash
ascend-tools otto tui
```

Vi keybindings by default. Type `/help` for commands, `/emacs` to switch modes.

### MCP server

Connect AI assistants (Claude Code, Codex CLI, Cursor, etc.) to Ascend via `uvx`:

```bash
claude mcp add --transport stdio ascend-tools-dev -- uvx ascend-tools mcp
```

Or `npx`:

```bash
claude mcp add --transport stdio ascend-tools-dev -- npx ascend-tools mcp
```

See [docs](docs/mcp.md) for more details.

### Skills

Install reference skills for AI coding assistants:

```bash
ascend-tools skill install --target .claude/skills --all
```

Available flags: `--cli` (default), `--python`, `--javascript`, `--rust`, `--mcp`, `--all`.

## Python SDK

Add `ascend-tools` to your Python project:

```bash
uv add ascend-tools
```

Then use the `Client` class:

```python
from ascend_tools import Client

client = Client()
client.list_workspaces()
client.run_flow(flow="My Flow", workspace="My Workspace")
```

See [docs](docs/python.md) for more details.

## JavaScript SDK

Add `ascend-tools` to your Node.js project:

```bash
npm add ascend-tools
```

Then use the `Client` class:

```javascript
import { Client } from "ascend-tools";

const client = new Client();
const workspaces = await client.listWorkspaces();
await client.runFlow("My Flow", "My Workspace");
```

See [docs](docs/javascript.md) for more details.

## Rust SDK

Add `ascend-tools-core` to your Rust project:

```bash
cargo add ascend-tools-core
```

Then use the `AscendClient` struct:

```rust
use ascend_tools::client::AscendClient;
use ascend_tools::config::Config;

let client = AscendClient::new(Config::from_env()?)?;
let workspaces = client.list_workspaces(Default::default())?;
```

See [docs](docs/rust.md) for more details.

## Documentation

- [Quickstart](docs/quickstart.md): create a service account, install, and run your first flow
- [Installation](docs/installation.md): all install methods
- [CLI](docs/cli.md): all commands with examples
- [Python SDK](docs/python.md): Client methods, return types, error handling
- [JavaScript SDK](docs/javascript.md): async Client methods, streaming, TypeScript types
- [Rust SDK](docs/rust.md): typed client with structs and error handling
- [MCP server](docs/mcp.md): set up AI assistants with Ascend tools
- [Development](docs/DEVELOPMENT.md): contributor setup, architecture, release process

## License

[MIT License](LICENSE)
