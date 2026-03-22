# ascend-tools

CLI, SDK, and MCP server for the [Ascend](https://www.ascend.io) Instance web API.

Rust core with [napi-rs](https://napi.rs) bindings for Node.js. Exposes the `Client` class and the full `ascend-tools` CLI.

## Install

```bash
npm install ascend-tools
```

## CLI

```bash
npx ascend-tools workspace list
npx ascend-tools flow run "My Flow" --workspace "My Workspace"
npx ascend-tools otto tui
```

Or install globally:

```bash
npm install -g ascend-tools
ascend-tools workspace list
```

## SDK

```javascript
import { Client } from "ascend-tools";

const client = new Client(); // reads from env vars
const workspaces = await client.listWorkspaces();
const run = await client.runFlow("sales", "My Workspace");
```

All methods are async and return plain objects/arrays. See the [demo app](../../tests/app/) for a full example.

**Otto thread SSE:** delta sync via `?after=<message_id>` on `GET …/updates` is supported by the Rust `ascend-tools-core` client (`sse_after_message_id` on the chat request). The Node binding does not expose that cursor yet; new threads always open the updates stream without `after` (progressive `thread.preview` / `thread.history`).

## Authentication

Set three environment variables (from Ascend UI > Settings > Users > Create Service Account):

```bash
export ASCEND_SERVICE_ACCOUNT_ID="asc-sa-..."
export ASCEND_SERVICE_ACCOUNT_KEY="..."
export ASCEND_INSTANCE_API_URL="https://<instance-name>.api.instance.ascend.io"
```

Or pass them directly:

```javascript
const client = new Client(
  "asc-sa-...",                      // serviceAccountId
  "...",                              // serviceAccountKey
  "https://api.instance.ascend.io",   // instanceApiUrl
);
```

## MCP server

Start an MCP server for AI assistants:

```bash
npx ascend-tools mcp
```

## Build from source

```bash
npm install
npm run build       # release
npm run build:debug # debug
npm test
```

Requires Rust toolchain and `@napi-rs/cli`.

## Targets

| Platform | Architecture |
|----------|-------------|
| macOS | x86_64, aarch64 |
| Linux | x86_64, aarch64 |

See the [full documentation](https://github.com/ascend-io/ascend-tools) for more details.
