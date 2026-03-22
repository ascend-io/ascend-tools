# ascend-tools

[![PyPI](https://img.shields.io/pypi/v/ascend-tools.svg)](https://pypi.org/project/ascend-tools/)

CLI, SDK, and MCP server for the [Ascend](https://www.ascend.io) Instance web API.

Rust core with [PyO3](https://pyo3.rs) bindings, built by [maturin](https://www.maturin.rs). Exposes the `Client` class and the full `ascend-tools` CLI.

## Install

```bash
uv tool install ascend-tools    # CLI
uv add ascend-tools             # library dependency
pip install ascend-tools        # pip
```

## CLI

```bash
ascend-tools workspace list
ascend-tools flow run "My Flow" --workspace "My Workspace"
ascend-tools otto tui
```

## SDK

```python
from ascend_tools import Client

client = Client()  # reads from env vars
client.list_workspaces()
client.run_flow(flow="sales", workspace="My Workspace")
```

All methods return `dict` or `list[dict]`. All parameters are keyword-only.

**Otto thread SSE:** delta sync with `?after=<message_id>` is implemented in the Rust core for CLI/SDK consumers that use `sse_after_message_id`. The Python `Client` wrapper does not expose that field yet; streams use full progressive snapshots.

## Authentication

Set three environment variables (from Ascend UI > Settings > Users > Create Service Account):

```bash
export ASCEND_SERVICE_ACCOUNT_ID="asc-sa-..."
export ASCEND_SERVICE_ACCOUNT_KEY="..."
export ASCEND_INSTANCE_API_URL="https://api.instance.ascend.io"
```

## MCP server

Start an MCP server for AI assistants:

```bash
ascend-tools mcp
```

See the [full documentation](https://github.com/ascend-io/ascend-tools) for more details.
