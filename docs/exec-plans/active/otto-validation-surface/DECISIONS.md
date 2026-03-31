# Otto Validation Surface Decisions

## Resolved Decisions

### 1. `ascend-tools` is a supplemental validation layer, not a browser replacement

Reason:

- text-native surfaces are faster for agents and humans to inspect
- browser/UI proof is still needed for final user-visible behavior

### 2. The repo should expose explicit thinking selection

Reason:

- validation is strongest when request inputs are explicit, not inferred
- this keeps the public contract aligned with the coordinated workspace wave

### 3. Structured stream surfaces matter as much as final responses

Reason:

- backend-isolation work requires visibility into reasoning and tool progress, not only terminal message text
- coarse text-only streaming is not enough to distinguish backend vs consumer bugs

### 4. Public docs must stay repo-local and generic

Reason:

- this repo is public
- plan/docs here should not reference private repo names or private implementation details

### 5. Current-wave proof gates on CLI, Python SDK, and core parser tests, with raw REST as supporting debug

Reason:

- the public validation surface should be able to stand on its own before QA drops to lower-level tooling
- CLI, Python SDK, and parser tests can be named precisely in the plan instead of treating `SDK/CLI/TUI/MCP` as one generic bucket
- raw REST still matters for backend-only debugging, but it should not hide missing functionality in the public validation surface
- later stages should not over-credit TUI or MCP before they reach contract parity

### 6. A validation surface only counts as backend-isolation proof if it shows explicit request parameters plus ordered stream detail

Reason:

- final-response-only output is not enough to localize reasoning, tool-progress, or event-order failures
- the minimum current-wave proof contract is explicit `thinking` request visibility plus ordered evidence for reasoning deltas, text deltas, tool-call start, tool-call argument deltas, tool-call output, and terminal/error status
- when a human or QA agent would naturally use the CLI/SDK first, the plan should upgrade that surface until it can carry the proof rather than routing around it with bespoke raw-REST code
- if a provider emits something outside that normalized set, the surface should expose it as unknown/raw rather than silently dropping it

### 7. TUI and MCP are deferred from the current validation gate until they can send explicit `thinking` and surface the minimum ordered-event contract

Reason:

- current heads still send `thinking: None` on those surfaces
- treating them as equivalent to raw REST, Python SDK, or CLI would create false confidence
- they can be promoted later once parity is implemented and proven

## Open Questions

- Once TUI and MCP reach request/stream parity, should they become same-wave gating surfaces or remain optional supporting probes?
- Which additional normalized event families beyond the current minimum contract are worth first-class public exposure after v1 parity lands?

## Evidence Used

- `AGENTS.md`
- `ARCHITECTURE.md`
- `docs/DEVELOPMENT.md`
- `tests/README.md`
- `crates/ascend-tools-core/src/models.rs`
- `crates/ascend-tools-core/src/client.rs`
- `crates/ascend-tools-tui/src/lib.rs`
- `tests/integration.py`

## Needs Human Input

None at this repo-local scope.
