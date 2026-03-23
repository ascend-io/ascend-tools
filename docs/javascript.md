# Use the JavaScript SDK

Manage Ascend workspaces, deployments, flows, and flow runs from JavaScript/TypeScript.

## Install

```bash
npm add ascend-tools
```

Upgrade to the latest version:

```bash
npm update ascend-tools
```

## CLI

The npm package includes the full `ascend-tools` CLI:

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

See [CLI guide](cli.md) for all commands.

## Authenticate

### From environment variables

```javascript
import { Client } from "ascend-tools";

const client = new Client(); // reads ASCEND_SERVICE_ACCOUNT_ID, etc. from env
```

See [Quickstart](quickstart.md) for the full service account creation walkthrough.

### With explicit credentials

```javascript
const client = new Client(
  "asc-sa-...",                      // serviceAccountId
  "...",                              // serviceAccountKey
  "https://api.instance.ascend.io",   // instanceApiUrl
);
```

## Environments and projects

### List environments

```javascript
const environments = await client.listEnvironments();
```

### Get an environment by title

```javascript
const env = await client.getEnvironment("Production");
```

### List projects

```javascript
const projects = await client.listProjects();
```

### Get a project by title

```javascript
const project = await client.getProject("My Project");
```

### List profiles

```javascript
const profiles = await client.listProfiles("My Workspace");
const profiles = await client.listProfiles(null, "My Deployment");
const profiles = await client.listProfiles(null, null, null, "My Project", "main");
```

## Manage workspaces and deployments

### Workspaces

```javascript
await client.listWorkspaces();
await client.listWorkspaces(null, null, "Production");
await client.getWorkspace("My Workspace");
await client.pauseWorkspace("My Workspace");
await client.resumeWorkspace("My Workspace");
await client.deleteWorkspace("My Workspace");
```

### Create a workspace

```javascript
const ws = await client.createWorkspace(
  "My Workspace",   // title
  "Production",     // environment
  "My Project",     // project
  "default",        // profile
  "main",           // gitBranch
);
```

### Update a workspace

```javascript
const ws = await client.updateWorkspace(
  "My Workspace",   // title
  null,              // uuid
  "New Title",       // newTitle
  "feature/abc",     // gitBranch
);
```

### Deployments

```javascript
await client.listDeployments();
await client.getDeployment("My Deployment");
await client.pauseDeploymentAutomations("My Deployment");
await client.resumeDeploymentAutomations("My Deployment");
await client.deleteDeployment("My Deployment");
```

### Create a deployment

```javascript
const dep = await client.createDeployment(
  "My Deployment",   // title
  "Production",      // environment
  "My Project",      // project
  "default",         // profile
  "main",            // gitBranch
);
```

### Update a deployment

```javascript
const dep = await client.updateDeployment(
  "My Deployment",   // title
  null,              // uuid
  null,              // newTitle
  null,              // gitBranch
  null,              // gitBranchBase
  null,              // profile
  null,              // size
  null,              // storageSize
  true,              // enableAutomations
);
```

## Manage flows

### List flows

```javascript
const flows = await client.listFlows("My Workspace");
const flows = await client.listFlows(null, "My Deployment");
```

### Run a flow

```javascript
const result = await client.runFlow("sales", "My Workspace");
```

Resume a paused workspace before running:

```javascript
const result = await client.runFlow(
  "sales",           // flow
  "My Workspace",    // workspace
  null,              // deployment
  null,              // uuid
  null,              // spec
  true,              // resume
);
```

Pass a spec object for advanced options:

```javascript
const result = await client.runFlow(
  "sales",           // flow
  "My Workspace",    // workspace
  null,              // deployment
  null,              // uuid
  { full_refresh: true },  // spec
);
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

```javascript
const result = await client.listFlowRuns("My Workspace");
const runs = result.items;         // array
const truncated = result.truncated; // bool
```

Filter by status, flow name, time range, or paginate:

```javascript
await client.listFlowRuns("My Workspace", null, null, "running");
await client.listFlowRuns(null, "My Deployment", null, null, "sales", null, null, null, 10);
await client.listFlowRuns("My Workspace", null, null, null, null, "2025-01-01T00:00:00Z");
```

### Get a flow run

```javascript
const run = await client.getFlowRun("fr-...", "My Workspace");
```

## Otto (AI assistant)

```javascript
// List providers and models
const providers = await client.listOttoProviders();

// Chat
const response = await client.otto("What flows are running?");
const response = await client.otto("Describe the sales flow", "My Workspace");
```

### Streaming

```javascript
const response = await client.ottoStreaming(
  "Describe the sales flow",
  (err, delta) => {
    if (err) console.error(err);
    else process.stdout.write(delta);
  },
  "My Workspace",
);
```

## Return types

- All methods are async (return Promises)
- All methods return plain objects or arrays
- TypeScript type definitions are included (`index.d.cts`)

## Error handling

The SDK throws errors for:

- Missing configuration (environment variables not set)
- Authentication failures (invalid or expired key)
- HTTP errors (API returns non-2xx status)
- State errors (paused, starting, error state)

```javascript
try {
  await client.runFlow("sales", "My Workspace");
} catch (e) {
  console.error(`Error: ${e.message}`);
}
```
