# ascend-tools-js

[napi-rs](https://napi.rs) bindings for the [Ascend](https://www.ascend.io) REST API SDK.

This crate produces the `@ascend-io/ascend-tools` native Node.js module. It exposes the `Client` class (from [`ascend-tools-core`](../ascend-tools-core)) to JavaScript/TypeScript via napi-rs, with async methods backed by `spawn_blocking`.

## Install

```bash
npm install @ascend-io/ascend-tools
```

## Usage

```javascript
import { Client } from "@ascend-io/ascend-tools";

const client = new Client(); // reads from env vars
const workspaces = await client.listWorkspaces();
const run = await client.runFlow("sales", "My Workspace");
```

All methods are async and return plain objects/arrays. See the [demo app](../../tests/app/) for a full example.

## Build

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

See the [top-level README](../../README.md) for full documentation.
