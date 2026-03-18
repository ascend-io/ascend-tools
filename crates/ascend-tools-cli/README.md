# ascend-tools-cli

[![crates.io](https://img.shields.io/crates/v/ascend-tools-cli.svg)](https://crates.io/crates/ascend-tools-cli)

CLI for the [Ascend](https://www.ascend.io) Instance web API. Installs as the `ascend-tools` binary.

Built on [`ascend-tools-core`](https://crates.io/crates/ascend-tools-core). Embeds [`ascend-tools-mcp`](https://crates.io/crates/ascend-tools-mcp) for the `mcp` subcommand and [`ascend-tools-tui`](https://crates.io/crates/ascend-tools-tui) for `otto tui`.

## Install

```bash
cargo install ascend-tools-cli  # Rust
uv tool install ascend-tools    # Python
npm install -g ascend-tools     # Node.js
```

## Authentication

```bash
export ASCEND_SERVICE_ACCOUNT_ID="asc-sa-..."
export ASCEND_SERVICE_ACCOUNT_KEY="..."
export ASCEND_INSTANCE_API_URL="https://api.instance.ascend.io"
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

  environment list | get <TITLE>
  project list | get <TITLE>
  profile list --workspace <TITLE> | --deployment <TITLE> | --project <NAME> --git-branch <BRANCH>

  flow list --workspace <TITLE> | --deployment <TITLE>
  flow run <FLOW_NAME> --workspace <TITLE> | --deployment <TITLE> [--spec '{}'] [--resume]
  flow list-runs --workspace <TITLE> | --deployment <TITLE> [--status, --flow, --since, --until, --offset, --limit]
  flow get-run <RUN_NAME> --workspace <TITLE> | --deployment <TITLE>

  otto run <PROMPT> [--workspace <TITLE>] [--provider <ID>] [--model <ID>]
  otto provider list
  otto model list [--provider <ID>]
  otto tui [--workspace <TITLE>]

  mcp [--http] [--bind <ADDR>]
  skill install --target <PATH> [--cli] [--python] [--javascript] [--rust] [--mcp] [--all]
```

Default output is a human-readable table. Use `-o json` for machine-readable output. Auth can also be passed via `--service-account-id`, `--service-account-key`, and `--instance-api-url` flags.

See the [full documentation](https://github.com/ascend-io/ascend-tools) for more details.
