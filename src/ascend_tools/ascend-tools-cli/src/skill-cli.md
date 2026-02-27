---
name: ascend-tools
description: Use the ascend-tools CLI to manage Ascend runtimes, flows, and flow runs.
---

# ascend-tools

Manage Ascend runtimes, flows, and flow runs via the `ascend-tools` CLI.

## Authentication

Set three environment variables (from Ascend UI > Settings > Users > Create Service Account):

```bash
export ASCEND_SERVICE_ACCOUNT_ID="asc-sa-..."
export ASCEND_SERVICE_ACCOUNT_KEY="..."
export ASCEND_INSTANCE_API_URL="https://api.instance.ascend.io"
```

## Commands

### Runtimes

```bash
ascend-tools runtime list [--id <ID>] [--kind <KIND>] [--project-uuid <UUID>] [--environment-uuid <UUID>]
ascend-tools runtime get <UUID>
ascend-tools runtime resume <UUID>
ascend-tools runtime pause <UUID>
```

### Flows

```bash
ascend-tools flow list --runtime <UUID>
ascend-tools flow run <FLOW_NAME> --runtime <UUID> [--spec '<JSON>'] [--resume]
ascend-tools flow list-runs --runtime <UUID> [--status <STATUS>] [--flow-name <NAME>] [--since <ISO8601>] [--until <ISO8601>] [--offset <N>] [--limit <N>]
ascend-tools flow get-run <RUN_NAME> --runtime <UUID>
```

### Flow run spec

Pass `--spec` as JSON to control flow run behavior:

```bash
ascend-tools flow run my-flow --runtime <UUID> --spec '{"full_refresh": true}'
ascend-tools flow run my-flow --runtime <UUID> --spec '{"components": ["component_a", "component_b"]}'
ascend-tools flow run my-flow --runtime <UUID> --spec '{"run_tests": false}'
```

Available spec fields: `full_refresh`, `components`, `component_categories`, `parameters`, `run_tests`, `store_test_results`, `halt_flow_on_error`, `disable_optimizers`, `update_materialization_type`, `deep_data_pruning`, `backfill_missing_statistics`, `disable_incremental_metadata_collection`, `runner_overrides`.

## Output

Default output is a human-readable table. Use `-o json` for machine-readable output:

```bash
ascend-tools -o json runtime list
```
