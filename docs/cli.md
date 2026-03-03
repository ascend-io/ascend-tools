# Use the CLI

Manage Ascend runtimes, flows, and flow runs from the command line.

## Install

```bash
uv tool install ascend-tools
```

Upgrade to the latest version:

```bash
uv tool install --upgrade ascend-tools
```

See [Installation](INSTALLATION.md) for other methods (Cargo, pre-built binaries).

## Authenticate

Set three environment variables (see [Quickstart](QUICKSTART.md) for the full service account creation walkthrough):

```bash
export ASCEND_SERVICE_ACCOUNT_ID="<YOUR_SERVICE_ACCOUNT_ID>"
export ASCEND_SERVICE_ACCOUNT_KEY="<YOUR_SERVICE_ACCOUNT_KEY>"
export ASCEND_INSTANCE_API_URL="<YOUR_INSTANCE_API_URL>"
```

You can also pass credentials as CLI flags: `--service-account-id`, `--service-account-key`, `--instance-api-url`. Flags override environment variables.

## Manage runtimes

### List runtimes

```bash
ascend-tools runtime list
```

Filter by ID, kind, project, or environment:

```bash
ascend-tools runtime list --id my-runtime
ascend-tools runtime list --kind deployment
ascend-tools runtime list --project-uuid <UUID>
ascend-tools runtime list --environment-uuid <UUID>
```

### Get a runtime

```bash
ascend-tools runtime get <RUNTIME_UUID>
```

### Pause a runtime

```bash
ascend-tools runtime pause <RUNTIME_UUID>
```

### Resume a runtime

```bash
ascend-tools runtime resume <RUNTIME_UUID>
```

## Manage flows

### List flows in a runtime

```bash
ascend-tools flow list --runtime <RUNTIME_UUID>
```

### Run a flow

```bash
ascend-tools flow run <FLOW_NAME> --runtime <RUNTIME_UUID>
```

Resume a paused runtime before running:

```bash
ascend-tools flow run <FLOW_NAME> --runtime <RUNTIME_UUID> --resume
```

Pass a flow run spec for advanced options:

```bash
ascend-tools flow run <FLOW_NAME> --runtime <RUNTIME_UUID> \
  --spec '{"full_refresh": true}'
```

### Flow run spec options

| Field | Type | Description |
|-------|------|-------------|
| `full_refresh` | bool | Drop all internal data and recompute from scratch. **Destructive.** |
| `components` | list | Run only these components (by name). Omit to run all. |
| `component_categories` | list | Run only components in these categories. |
| `parameters` | object | Custom parameters passed to the flow. |
| `run_tests` | bool | Run tests after processing data. Defaults to true. |
| `store_test_results` | bool | Store test results. |
| `halt_flow_on_error` | bool | Stop the flow on error. |
| `disable_optimizers` | bool | Disable optimizers. |
| `update_materialization_type` | bool | Update component materialization types. **May drop data.** |
| `deep_data_pruning` | bool | Full table scan for Smart Table data maintenance. |
| `backfill_missing_statistics` | bool | Backfill statistics for data blocks without them. |
| `disable_incremental_metadata_collection` | bool | Disable incremental read/transform metadata collection. |
| `runner_overrides` | object | Runner config overrides (e.g., `{"size": "Medium"}`). |

## Monitor flow runs

### List flow runs

```bash
ascend-tools flow list-runs --runtime <RUNTIME_UUID>
```

Filter by status, flow name, time range, or paginate:

```bash
ascend-tools flow list-runs --runtime <RUNTIME_UUID> --status running
ascend-tools flow list-runs --runtime <RUNTIME_UUID> --flow-name sales
ascend-tools flow list-runs --runtime <RUNTIME_UUID> --since 2025-01-01T00:00:00Z
ascend-tools flow list-runs --runtime <RUNTIME_UUID> --limit 10 --offset 20
```

### Get a flow run

```bash
ascend-tools flow get-run <RUN_NAME> --runtime <RUNTIME_UUID>
```

## Output formats

Default output is a human-readable table. Use `-o json` for machine-readable output:

```bash
ascend-tools -o json runtime list
ascend-tools -o json flow list-runs --runtime <RUNTIME_UUID>
```

Empty results print "No results." to stderr.

## Install AI assistant skills

Install reference skills for AI coding assistants (Claude Code, Codex, etc.):

```bash
ascend-tools skill install --target .claude/skills --all
```

Available flags: `--cli` (default), `--python`, `--mcp`, `--all`.
