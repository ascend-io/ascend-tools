---
name: ascend-tools-cli
description: Use the ascend-tools CLI to manage Ascend workspaces, deployments, flows, and flow runs.
---

# ascend-tools CLI

Manage Ascend workspaces, deployments, flows, and flow runs via the `ascend-tools` CLI.

## Installation

```bash
uvx ascend-tools --help
```

Or install permanently:

```bash
uv tool install ascend-tools
```

Upgrade to the latest version:

```bash
uv tool install --upgrade ascend-tools
```

## Authentication

Set three environment variables (from Ascend UI > Settings > Users > Create Service Account):

```bash
export ASCEND_SERVICE_ACCOUNT_ID="asc-sa-..."
export ASCEND_SERVICE_ACCOUNT_KEY="..."
export ASCEND_INSTANCE_API_URL="https://<instance-name>.api.instance.ascend.io"
```

## Commands

### Workspaces

```bash
ascend-tools workspace list [--environment <NAME>] [--project <NAME>]
ascend-tools workspace get <TITLE>
ascend-tools workspace create --title <TITLE> --environment <NAME> --project <NAME> --profile <NAME> --git-branch <BRANCH> [--git-branch-base, --size, --storage-size, --auto-snooze-timeout-minutes]
ascend-tools workspace update <TITLE> [--title, --git-branch, --git-branch-base, --profile, --size, --storage-size, --auto-snooze-timeout-minutes]
ascend-tools workspace pause <TITLE>
ascend-tools workspace resume <TITLE>
ascend-tools workspace delete <TITLE>
```

### Deployments

```bash
ascend-tools deployment list [--environment <NAME>] [--project <NAME>]
ascend-tools deployment get <TITLE>
ascend-tools deployment create --title <TITLE> --environment <NAME> --project <NAME> --profile <NAME> --git-branch <BRANCH> [--git-branch-base, --size, --storage-size, --enable-automations]
ascend-tools deployment update <TITLE> [--title, --git-branch, --git-branch-base, --profile, --size, --storage-size, --enable-automations]
ascend-tools deployment pause-automations <TITLE>
ascend-tools deployment resume-automations <TITLE>
ascend-tools deployment delete <TITLE>
```

### Environments

```bash
ascend-tools environment list
ascend-tools environment get <TITLE>
```

### Projects

```bash
ascend-tools project list
ascend-tools project get <TITLE>
```

### Profiles

```bash
ascend-tools profile list --workspace <TITLE>
ascend-tools profile list --deployment <TITLE>
ascend-tools profile list --project <TITLE> --git-branch <BRANCH>
```

### Flows

```bash
ascend-tools flow list --workspace <TITLE> | --deployment <TITLE>
ascend-tools flow run <FLOW_NAME> --workspace <TITLE> | --deployment <TITLE> [--spec '<JSON>'] [--resume]
ascend-tools flow list-runs --workspace <TITLE> | --deployment <TITLE> [--status <STATUS>] [--flow <NAME>] [--since <ISO8601>] [--until <ISO8601>] [--offset <N>] [--limit <N>]
ascend-tools flow get-run <RUN_NAME> --workspace <TITLE> | --deployment <TITLE>
```

### Otto

```bash
ascend-tools otto run "<PROMPT>" [--workspace <TITLE>] [--provider <ID>] [--model <ID>]
ascend-tools otto providers list
ascend-tools otto models list [--provider <ID>]
ascend-tools otto tui [--workspace <TITLE>]
```

### Flow run spec

Pass `--spec` as JSON to control flow run behavior:

```bash
ascend-tools flow run my-flow --workspace "My Workspace" --spec '{"full_refresh": true}'
ascend-tools flow run my-flow --deployment "My Deployment" --spec '{"components": ["component_a", "component_b"]}'
ascend-tools flow run my-flow --workspace "My Workspace" --spec '{"run_tests": false}'
```

Available spec fields: `full_refresh`, `components`, `component_categories`, `parameters`, `run_tests`, `store_test_results`, `halt_flow_on_error`, `disable_optimizers`, `update_materialization_type`, `deep_data_pruning`, `backfill_missing_statistics`, `disable_incremental_metadata_collection`, `runner_overrides`.

## Output

Default output is a human-readable table. Use `-o json` for machine-readable output:

```bash
ascend-tools -o json workspace list
```

`--environment` and `--project` accept friendly names (titles), not UUIDs. UUIDs still work for all commands via `--uuid` flag.
