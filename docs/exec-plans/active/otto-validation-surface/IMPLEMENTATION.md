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
- `tests/rest.py`
- `tests/rest.js`
- `tests/integration.py`
- `tests/integration.sh`
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

### 3. Surface layering

- raw REST scripts validate the backend contract directly
- SDK/CLI/TUI/MCP validate this repo's translation / exposure layer
- downstream browser/UIs remain separate consumers outside this repo's contract

## Ordered Phases

### Phase 1: Request widening

- extend the Otto request model to carry explicit thinking-level selection
- thread that selection through CLI, SDK, TUI, and MCP entrypoints as appropriate

### Phase 2: Streaming event widening

- expand the structured event model beyond text deltas and coarse tool-call events where needed
- ensure TUI/CLI/SDK/MCP consumers can inspect the new behavior without needing a browser

### Phase 3: Validation assets

- add or extend raw REST and integration tests so they can prove backend behavior independently of a browser consumer
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
- The repo can help distinguish backend failures from downstream consumer failures
- Public docs remain free of private-repo references
