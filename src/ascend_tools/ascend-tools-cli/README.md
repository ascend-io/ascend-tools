# ascend-tools-cli

CLI for the [Ascend](https://www.ascend.io) REST API. Installs as the `ascend-tools` binary.

Built on [`ascend-tools-core`](../ascend-tools-core). Also embeds [`ascend-tools-mcp`](../ascend-tools-mcp) for the `mcp` subcommand.

## Install

```bash
cargo install ascend-tools-cli
```

Or via Python:

```bash
uv tool install ascend-tools
```

## Usage

```
ascend-tools [-o text|json] [-V]

  runtime list [--id, --kind, --project-uuid, --environment-uuid]
  runtime get <UUID>
  runtime resume <UUID>
  runtime pause <UUID>

  flow list --runtime <UUID>
  flow run <FLOW_NAME> --runtime <UUID> [--spec '{}'] [--resume]
  flow list-runs --runtime <UUID> [--status, --flow-name, --since, --until, --offset, --limit]
  flow get-run <RUN_NAME> --runtime <UUID>

  mcp [--http] [--bind <ADDR>]
  skill install --target <PATH>
```

## Authentication

```bash
export ASCEND_SERVICE_ACCOUNT_ID="asc-sa-..."
export ASCEND_SERVICE_ACCOUNT_KEY="..."
export ASCEND_INSTANCE_API_URL="https://api.instance.ascend.io"
```

Auth can also be passed via `--service-account-id`, `--service-account-key`, and `--instance-api-url` flags.

See the [top-level README](../../../README.md) for full documentation.
