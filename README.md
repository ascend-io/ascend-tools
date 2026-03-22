# ascend-tools

CLI, SDK, and MCP server for the Ascend Instance web API.

[![PyPI](https://img.shields.io/pypi/v/ascend-tools?color=blue)](https://pypi.org/project/ascend-tools/)
[![npm](https://img.shields.io/npm/v/ascend-tools?color=blue)](https://www.npmjs.com/package/ascend-tools)
[![crates.io](https://img.shields.io/crates/v/ascend-tools-core?color=blue)](https://crates.io/crates/ascend-tools-core)
[![CI](https://img.shields.io/github/actions/workflow/status/ascend-io/ascend-tools/ci.yml?branch=main&label=CI)](https://github.com/ascend-io/ascend-tools/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/license-MIT-8A2BE2.svg)](https://github.com/ascend-io/ascend-tools/blob/main/LICENSE)

## Getting started

Don't have an Ascend Instance yet? Sign up:

```bash
ascend-tools signup
```

## Authentication

Set three environment variables (from Ascend UI > Settings > Users > Create Service Account):

```bash
export ASCEND_SERVICE_ACCOUNT_ID="asc-sa-..."
export ASCEND_SERVICE_ACCOUNT_KEY="..."
export ASCEND_INSTANCE_API_URL="https://<instance-name>.api.instance.ascend.io"
```

## CLI

Install via [PyPI](https://pypi.org/project/ascend-tools/), [npm](https://www.npmjs.com/package/ascend-tools), [crates.io](https://crates.io/crates/ascend-tools-cli), [GitHub releases](https://github.com/ascend-io/ascend-tools/releases), or [source](docs/development.md).

Python:

```bash
uv tool install ascend-tools
```

Node.js:

```bash
npm install -g ascend-tools
```

Rust:

```bash
cargo install ascend-tools-cli
```

Or run directly without installing via `uvx`:

```bash
uvx ascend-tools workspace list
```

Or `npx`:

```bash
npx ascend-tools workspace list
```

### Interactive TUI

Run Otto in an interactive terminal user interface (TUI):

```bash
ascend-tools otto tui
```

Vi keybindings by default. Type `/help` for commands, `/emacs` to switch modes.

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

## Diagnostics and logging

- **`ascend-tools-core`** (Rust library used by the CLI and other crates): malformed SSE JSON lines and other parse skips are reported on **stderr** via `eprintln!`. There is no `tracing` dependency in this crate, so **`RUST_LOG` does not control core SSE diagnostics**.
- **`ascend-tools-mcp`**: uses `tracing` / `tracing-subscriber`; set **`RUST_LOG`** (e.g. `RUST_LOG=info`) for MCP server logs.

The JavaScript and Python SDK clients do not yet expose the Otto thread SSE **`?after=`** delta cursor; use the Rust `OttoChatRequest.sse_after_message_id` field when you need delta mode from the SDK.

## MCP server

Connect AI assistants (Claude Code, Codex CLI, Cursor, etc.) to Ascend via `uvx`:

```bash
claude mcp add --transport stdio ascend-tools-dev -- uvx ascend-tools mcp
```

Or `npx`:

```bash
claude mcp add --transport stdio ascend-tools-dev -- npx ascend-tools mcp
```

See [MCP server guide](docs/mcp.md) for Codex CLI setup and the full tools reference.

## Skills

Install reference skills for AI coding assistants:

```bash
ascend-tools skill install --target .claude/skills --all
```

Available flags: `--cli` (default), `--python`, `--javascript`, `--rust`, `--mcp`, `--all`.

## Development

```bash
bin/setup
bin/check
bin/build
bin/format
```

See the [development guide](docs/development.md) for the full contributor setup.

## Documentation

- [Quickstart](docs/QUICKSTART.md): create a service account, install, and run your first flow
- [Installation](docs/INSTALLATION.md): all install methods
- [CLI](docs/cli.md): all commands with examples
- [Python SDK](docs/python.md): Client methods, return types, error handling
- [JavaScript SDK](docs/javascript.md): async Client methods, streaming, TypeScript types
- [Rust SDK](docs/rust.md): typed client with structs and error handling
- [MCP server](docs/mcp.md): set up AI assistants with Ascend tools
- [Development](docs/development.md): contributor setup, architecture, release process
