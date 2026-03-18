---
name: ascend-tools-javascript
description: Use the ascend-tools JavaScript SDK to manage Ascend workspaces, deployments, flows, and flow runs.
---

# ascend-tools JavaScript SDK

Manage Ascend workspaces, deployments, flows, and flow runs from JavaScript/TypeScript via the `ascend-tools` SDK.

## Installation

```bash
npm add ascend-tools
```

Upgrade to the latest version:

```bash
npm update ascend-tools
```

## Authentication

Set three environment variables (from Ascend UI > Settings > Users > Create Service Account):

```bash
export ASCEND_SERVICE_ACCOUNT_ID="asc-sa-..."
export ASCEND_SERVICE_ACCOUNT_KEY="..."
export ASCEND_INSTANCE_API_URL="https://<instance-name>.api.instance.ascend.io"
```

Or pass credentials directly:

```javascript
import { Client } from "ascend-tools";

const client = new Client(
  "asc-sa-...",               // serviceAccountId
  "...",                       // serviceAccountKey
  "https://api.instance.ascend.io", // instanceApiUrl
);
```

## Usage

All methods are async and return plain objects/arrays.

```javascript
import { Client } from "ascend-tools";

const client = new Client();
```

### Environments and projects

```javascript
// List environments
await client.listEnvironments();

// Get by title
await client.getEnvironment("Production");

// List projects
await client.listProjects();

// Get by title
await client.getProject("My Project");

// List profiles
await client.listProfiles("My Workspace");
```

### Workspaces

```javascript
await client.listWorkspaces();
await client.listWorkspaces(null, null, "Production");
await client.getWorkspace("My Workspace");
await client.pauseWorkspace("My Workspace");
await client.resumeWorkspace("My Workspace");
await client.deleteWorkspace("My Workspace");
```

### Deployments

```javascript
await client.listDeployments();
await client.getDeployment("My Deployment");
await client.deleteDeployment("My Deployment");
```

### Flows

```javascript
// List flows in a workspace or deployment
await client.listFlows("My Workspace");
await client.listFlows(null, "My Deployment");

// Trigger a flow run
await client.runFlow("sales", "My Workspace");

// Pass a spec to control behavior
await client.runFlow("sales", "My Workspace", null, null, { full_refresh: true });
```

### Flow runs

```javascript
// List flow runs (returns { items: [...], truncated: bool })
await client.listFlowRuns("My Workspace");

// Filter by status or flow name
await client.listFlowRuns("My Workspace", null, null, "running", "sales");

// Get a single flow run
await client.getFlowRun("fr-...", "My Workspace");
```

### Otto (AI assistant)

```javascript
await client.listOttoProviders();
await client.otto("What flows are running?");
await client.otto("Describe the sales flow", "My Workspace");
```

### Flow run spec

Pass `spec` as an object to `runFlow` to control flow run behavior:

```javascript
await client.runFlow("sales", "My Workspace", null, null, { full_refresh: true });
await client.runFlow("sales", "My Workspace", null, null, { run_tests: false });
await client.runFlow("sales", "My Workspace", null, null, { parameters: { key: "value" } });
```

Available spec fields: `full_refresh`, `components`, `component_categories`, `parameters`, `run_tests`, `store_test_results`, `halt_flow_on_error`, `disable_optimizers`, `update_materialization_type`, `deep_data_pruning`, `backfill_missing_statistics`, `disable_incremental_metadata_collection`, `runner_overrides`.
