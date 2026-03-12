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

  workspace list [--environment <NAME>] [--project <NAME>]
  workspace get <TITLE>
  workspace create --title <TITLE> --environment <NAME> --project <NAME> --profile <NAME> --git-branch <BRANCH>
  workspace update <TITLE> [--title, --git-branch, --profile, --size, --storage-size]
  workspace pause <TITLE>
  workspace resume <TITLE>
  workspace delete <TITLE>

  deployment list [--environment <NAME>] [--project <NAME>]
  deployment get <TITLE>
  deployment create --title <TITLE> --environment <NAME> --project <NAME> --profile <NAME> --git-branch <BRANCH>
  deployment update <TITLE> [--title, --git-branch, --profile, --size, --storage-size]
  deployment pause-automations <TITLE>
  deployment resume-automations <TITLE>
  deployment delete <TITLE>

  flow list --workspace <TITLE> | --deployment <TITLE>
  flow run <FLOW_NAME> --workspace <TITLE> | --deployment <TITLE> [--spec '{}'] [--resume]
  flow list-runs --workspace <TITLE> | --deployment <TITLE> [--status, --flow, --since, --until, --offset, --limit]
  flow get-run <RUN_NAME> --workspace <TITLE> | --deployment <TITLE>

  otto run <PROMPT> [--workspace <TITLE>] [--provider <ID>] [--model <ID>]
  otto provider list
  otto model list [--provider <ID>]
  otto tui [--workspace <TITLE>]

  mcp [--http] [--bind <ADDR>]
  skill install --target <PATH> [--cli] [--python] [--mcp] [--all]
```

## Authentication

```bash
export ASCEND_SERVICE_ACCOUNT_ID="asc-sa-..."
export ASCEND_SERVICE_ACCOUNT_KEY="..."
export ASCEND_INSTANCE_API_URL="https://api.instance.ascend.io"
```

Auth can also be passed via `--service-account-id`, `--service-account-key`, and `--instance-api-url` flags.

See the [top-level README](../../../README.md) for full documentation.
