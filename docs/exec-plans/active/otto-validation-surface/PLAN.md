# Otto Validation Surface

## Objective

Extend the `ascend-tools` Otto surfaces so they provide a fast, text-native validation and debugging layer for streamed reasoning, tool-call progress, and stream lifecycle behavior, making it easier to distinguish backend contract issues from consumer-UI issues.

## Success Contract

- `ascend-tools` can exercise Otto with explicit thinking levels, not only default behavior.
- The SDK/CLI/TUI/MCP surfaces expose enough structured stream information to validate reasoning and tool-call progress without going straight to a browser.
- Validation assets in this repo can help isolate whether a failure belongs to:
  - the backend contract
  - the `ascend-tools` SDK/CLI/TUI/MCP layer
  - a downstream UI consumer
- This repo supplements browser/UI testing rather than trying to replace it.

## Scope

- Otto request model in `ascend-tools-core`
- Otto streaming event model and SSE dispatch in `ascend-tools-core`
- Otto CLI/TUI/SDK/MCP surfaces that expose request selection or stream behavior
- Integration and raw-REST validation assets under `tests/`
- Public docs that explain how to use these surfaces for Otto validation

## Non-goals

- Reproducing a downstream UI's exact visual behavior inside `ascend-tools`
- Replacing browser/UI testing as the final proof for user-visible behavior
- Encoding private-repo implementation details into this public repo's docs
- Becoming the source of truth for provider-specific product semantics outside the `ascend-tools` API surface

## Current State

- `ascend-tools` already has:
  - raw REST validation scripts
  - SDK and CLI integration tests
  - a TUI
  - streaming support for text deltas and tool call start/output
- `ascend-tools` does not yet appear to expose structured streamed reasoning events or progressive tool-call argument deltas as first-class validation surfaces.
- The Otto request surface in this repo is still minimal compared with the explicit thinking-level contract being adopted in the coordinated workspace wave.

## Actor / Role Matrix

| Actor | Goal | Relevant behavior |
| --- | --- | --- |
| Engineer or agent debugging Otto | Quickly determine whether an Otto failure is backend-side before escalating to browser/UI proof | Uses raw REST, SDK, CLI, TUI, or MCP surfaces |
| SDK/CLI/TUI maintainer | Keep public Otto surfaces aligned with the real backend contract | Adds request fields, streaming events, and docs/tests |
| Reviewer | Verify that `ascend-tools` accelerates debugging without claiming to replace end-to-end UI proof | Checks contract clarity, tests, and public-doc honesty |

## Execution Phases

1. Define the public Otto validation contract this repo should expose
2. Extend request and streaming surfaces where needed
3. Add validation assets that isolate backend vs consumer issues
4. Document the supported debugging / validation workflow

## Plan Deltas

- Promote `ascend-tools` from implicit supporting repo to explicit active plan root in the coordinated workspace wave.
- Treat this repo as a backend-isolation and validation surface, not just a generic SDK/CLI package.
- Keep the plan public-repo-safe by describing downstream consumers generically rather than naming private repos.

## Needs Human Input

None at this repo-local scope.

## Deferred / Follow-up Work

- Richer conversation/thread inspection helpers if later debugging needs them
- Additional structured event surfaces beyond the minimum needed for Otto validation
- Broader tutorial/demo material once the validation contract stabilizes

## Execution Log

- 2026-03-28: Created the initial ASE plan root for the public `ascend-tools` repo as part of the coordinated Otto validation workspace wave.
