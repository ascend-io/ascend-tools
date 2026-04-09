# Otto Validation Surface

## Objective

Extend the `ascend-tools` Otto surfaces so they provide a fast, text-native validation and debugging layer for streamed reasoning, tool-call progress, and stream lifecycle behavior, making it easier to distinguish backend contract issues from consumer-UI issues.

## Success Contract

- `ascend-tools` can exercise Otto with explicit thinking levels, not only default behavior.
- CLI (`tests/integration.sh` plus `ascend-tools otto run --jsonl`) is the current-wave public proof surface for exact request provenance and ordered stream inspection.
- Python SDK (`tests/integration.py`) remains a current-wave request-contract surface, but it does not substitute for ordered-event proof until it exposes the same minimum contract.
- Raw REST (`tests/rest.js`) is a supporting debug surface for backend isolation, not a substitute for missing CLI/SDK validation capabilities.
- At least one higher-level public validation surface preserves ordered event inspection plus provenance metadata (`base URL`, provider, model, explicit `thinking`, thread/request identifiers, and terminal status) so the browser is not the first place a human has to inspect payload order.
- TUI and MCP only count in the current wave if they can send explicit `thinking` and expose the same minimum ordered-event contract; otherwise they remain explicit follow-up surfaces rather than implicit proof.
- Validation assets in this repo can help isolate whether a failure belongs to:
  - the backend contract
  - the `ascend-tools` request/stream surface in use
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
  - a raw REST Otto SSE asset in `tests/rest.js`
  - a Python SDK integration asset in `tests/integration.py`
  - a CLI integration asset in `tests/integration.sh`
  - deterministic request/stream parser tests in `crates/ascend-tools-core/tests/client_http.rs`
  - a TUI and MCP surface
- `tests/rest.py` is not the Otto streaming proof asset for this workspace wave.
- The current heads widened request support and now add a structured CLI `--jsonl` path for raw ordered Otto updates; TUI and MCP still send `thinking: None` and therefore are not current-wave gating proof surfaces until widened.
- Raw REST remains valuable for lower-level debugging, but the repo should reject "implementation complete" if CLI/SDK surfaces still cannot show the exact request/provenance and ordered events QA needs.

## Actor / Role Matrix

| Actor | Goal | Relevant behavior |
| --- | --- | --- |
| Engineer or agent debugging Otto | Quickly determine whether an Otto failure is backend-side before escalating to browser/UI proof | Uses raw REST, SDK, CLI, TUI, or MCP surfaces |
| SDK/CLI/TUI maintainer | Keep public Otto surfaces aligned with the real backend contract | Adds request fields, streaming events, and docs/tests |
| Reviewer | Verify that `ascend-tools` accelerates debugging without claiming to replace end-to-end UI proof | Checks contract clarity, tests, and public-doc honesty |

## Execution Phases

1. Define the minimum public Otto validation contract this repo should expose, including which surfaces are current-wave gating versus deferred follow-up.
2. Extend request and streaming surfaces where needed.
3. Add validation assets that prove explicit `thinking` serialization, ordered event families, terminal classification, and cross-surface parity.
4. Document the supported debugging / validation workflow, including provenance and environment-authority expectations.

## Plan Deltas

- Promote `ascend-tools` from implicit supporting repo to explicit active plan root in the coordinated workspace wave.
- Treat this repo as a backend-isolation and validation surface, not just a generic SDK/CLI package.
- Keep the plan public-repo-safe by describing downstream consumers generically rather than naming private repos.

## Needs Human Input

None at this repo-local scope.

## Deferred / Follow-up Work

- Richer conversation/thread inspection helpers if later debugging needs them
- Explicit `thinking` plus ordered-event parity for TUI and MCP if they are brought into the validation gate later
- Additional structured event surfaces beyond the minimum needed for Otto validation
- Broader tutorial/demo material once the validation contract stabilizes

## Execution Log

- 2026-03-28: Created the initial ASE plan root for the public `ascend-tools` repo as part of the coordinated Otto validation workspace wave.
