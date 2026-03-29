# Otto Progressive Thread Loading

## Resolved Decisions

### All first-party consumers are in scope

- The planning surface includes Rust core, CLI, TUI, Python, JavaScript, and MCP.
- The primary implementation leverage remains in `ascend-tools-core`.

### Core remains the canonical client abstraction owner

- Every first-party interface rides the Rust core.
- Transport semantics and data-model semantics should be solved in the core first, then projected into each surface.

### The client targets legacy-bootstrap removal

- The plan assumes the backend will retire `thread.details`.
- The client therefore targets progressive thread bootstrap and checkpoint-aware reopen instead of compatibility with full-thread bootstrap snapshots.

### Shared persistence is not assumed in v1

- Existing persistence is limited to TUI prompt history.
- The plan does not assume a shared local database or shared cache layer across all consumers.
- Local persistence, if chosen, should be justified by TUI reopen/resume UX.

### V1 stays stateless by default

- The default implementation stance is stateless clients.
- If local thread persistence is added, it should be TUI-local and justified by concrete reopen/resume evidence.

### Public API shape splits progressive reads from full materialization

- `get_conversation()` remains the explicit full-materialization path.
- Progressive bootstrap, paged older-history retrieval, and checkpoint-based reopen should be represented by lower-level read APIs rather than hidden behind every high-level convenience call.

### Summary listing stays paginated and bounded in v1

- The plan does not assume streamed summary bootstrap for v1.
- Summary listing remains a bounded REST-style list until there is evidence it becomes the next bottleneck.

### CLI JSONL streaming output is in scope

- `otto run` should support machine-readable JSONL event output.
- The purpose is testing, auditing, and agentic implementation loops that need real API traces without a browser.

### Docs and public type surfaces are part of the implementation scope

- Python stubs, JavaScript type surfaces, MCP tool descriptions, and public docs are part of the same change set as the core/client behavior.
- This avoids “implementation updated, public surface stale” drift.

## Open Questions

- No major plan-level questions remain. Reopen these only if implementation evidence shows:
  - stateless reopen is inadequate for the TUI, or
  - `get_conversation()` and progressive read APIs cannot coexist cleanly, or
  - summary-list scaling becomes the next bottleneck immediately

## Evidence Used

- `AGENTS.md`
- `ARCHITECTURE.md`
- `crates/ascend-tools-core/src/client.rs`
- `crates/ascend-tools-core/src/models.rs`
- `crates/ascend-tools-tui/src/lib.rs`
- public docs and binding/type surfaces across CLI, Python, JavaScript, and MCP
- related backend and UI plan roots for cross-repo contract alignment

## Needs Human Input

- None at plan time. The current plan is locked to:
  - stateless v1 by default
  - TUI-local persistence only if evidence demands it
  - explicit progressive read APIs alongside `get_conversation()`
  - paginated summary listing in v1
  - CLI JSONL event streaming output
