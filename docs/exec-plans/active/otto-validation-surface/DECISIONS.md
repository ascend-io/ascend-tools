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

## Open Questions

- Which exact structured reasoning/tool-progress events should be exposed as first-class public SDK/TUI/MCP surfaces in v1?

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
