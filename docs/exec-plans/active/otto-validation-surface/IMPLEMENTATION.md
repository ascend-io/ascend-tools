# Otto Validation Surface Implementation

## Repos And Surfaces In Scope

### Primary repo: `ascend-tools`

- `crates/ascend-tools-core/src/models.rs`
- `crates/ascend-tools-core/src/client.rs`
- `crates/ascend-tools-core/src/sse.rs`
- `crates/ascend-tools-cli/src/otto.rs`
- `crates/ascend-tools-tui/src/lib.rs`
- `crates/ascend-tools-mcp/src/server.rs`
- `crates/ascend-tools-py/src/lib.rs`
- `crates/ascend-tools-js/src/lib.rs`
- `tests/rest.js`
- `tests/integration.py`
- `tests/integration.sh`
- `crates/ascend-tools-core/tests/client_http.rs`
- relevant public docs under `docs/`

## Boundary Contracts

### 1. Request contract

The public Otto request surface in this repo should be able to carry:

- prompt
- runtime/deployment/workspace target
- model selection
- explicit thinking level
- conversation/thread continuation

This repo should not guess or hide the thinking selection once the explicit contract exists.

### 2. Stream event contract

The public Otto streaming surface should expose enough structured events to support backend isolation:

- text delta
- reasoning/thinking progress when emitted
- tool-call start
- progressive tool-call argument updates when emitted
- tool-call output
- terminal status / error

For the current wave, that contract must be available through the CLI itself, not only through direct raw REST scripts.

### 3. Surface layering

- CLI is the current-wave higher-level ordered-event proof surface for this repo's translation / exposure layer
- Python SDK is a current-wave request-contract surface and should be explicit about any ordered-event limitations
- raw REST scripts validate the backend contract directly and remain a debugging fallback rather than a substitute for missing CLI/SDK capabilities
- TUI and MCP are follow-up surfaces until they can send explicit `thinking` and expose the minimum ordered-event contract
- downstream browser/UIs remain separate consumers outside this repo's contract

## Ordered Phases

### Phase 1: Request widening

- extend the Otto request model to carry explicit thinking-level selection
- thread that selection through the current-wave gating surfaces first (raw REST asset, Python SDK, and CLI), then widen TUI/MCP if they are kept in scope

### Phase 2: Streaming event widening

- expand the structured event model beyond text deltas and coarse tool-call events where needed
- ensure the CLI can inspect the widened behavior without needing a browser, and do not credit TUI/MCP until they reach the same minimum contract

### Phase 3: Validation assets

- add or extend CLI/SDK validation assets so they can prove higher-level behavior independently of a browser consumer
- keep raw REST available for backend-only debugging when CLI/SDK evidence reveals a discrepancy
- keep validation assets explicit about which surfaces are merge-gating now versus deferred follow-up
- keep the separation clear between backend failures and `ascend-tools`-layer failures

### Phase 4: Public docs

- document how to use these surfaces for Otto validation and debugging
- keep the wording generic and public-repo-safe

## Allowed Change Boundaries

- Otto request/streaming surfaces in this repo
- validation assets in `tests/`
- public-facing docs in this repo

## Review-Readiness Conditions

- The request and stream contracts are explicit enough for later agents or humans to use for debugging
- The current-wave gate is explicit about which surfaces count now and which remain deferred
- The repo can help distinguish backend failures from downstream consumer failures
- Public docs remain free of private-repo references
