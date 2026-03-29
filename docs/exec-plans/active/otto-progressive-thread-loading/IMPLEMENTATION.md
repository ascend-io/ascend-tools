# Otto Progressive Thread Loading

## Repos And Surfaces In Scope

### Canonical transport and models

- `crates/ascend-tools-core/src/client.rs`
- `crates/ascend-tools-core/src/models.rs`
- `crates/ascend-tools-core/tests/client_http.rs`

### Stateful terminal surface

- `crates/ascend-tools-tui/src/lib.rs`

### CLI and public wrappers

- `crates/ascend-tools-cli/src/otto.rs`
- `crates/ascend-tools-cli/src/conversation.rs`
- `crates/ascend-tools-py/src/lib.rs`
- `py/ascend_tools/core.pyi`
- `crates/ascend-tools-js/src/lib.rs`
- `crates/ascend-tools-js/index.d.ts`
- `crates/ascend-tools-mcp/src/server.rs`
- `crates/ascend-tools-mcp/src/params.rs`

### Public docs to keep aligned

- `docs/cli.md`
- `docs/rust.md`
- `docs/python.md`
- `docs/javascript.md`
- `docs/mcp.md`

## Boundary Contracts

### 1. One Rust core, many interfaces

- `ascend-tools-core` stays the only Otto transport/model implementation.
- CLI, TUI, Python, JavaScript, and MCP should not invent their own thread-loading semantics.

### 2. Summary list vs thread-body bootstrap

- Conversation listing remains a bounded summary operation.
- Thread open/resume must no longer assume that `get_conversation()` can always return an acceptable full body for large threads.
- The public client surface needs a way to distinguish summary state, initial bootstrap state, older-history pages, and live catch-up state.
- `get_conversation()` remains the explicit full-materialization path rather than a hidden prerequisite for every reopen or send.

### 3. Shared `/updates` contract

- The tools client consumes the shared backend contract after `thread.details` removal.
- A checkpoint such as `after` should narrow the amount of bootstrap work, but it should not be the thing that decides whether a delta phase exists at all.
- Expected shape:
  - bootstrap recent state first,
  - optionally page older history through a separate history-read surface,
  - then apply a catch-up delta relative to the bootstrap boundary or caller checkpoint,
  - then continue with granular live events.

This follows the common “snapshot/base plus delta/replay” pattern used by large real-time systems: deltas are meaningful only relative to a known baseline, while checkpoints make the baseline smaller rather than eliminating the concept of bootstrap.

### 4. Persistence boundary

- Today only TUI prompt history is persisted.
- V1 should not assume shared persistence across all consumers.
- If local thread persistence is chosen, it should be justified by TUI reopen/resume UX and remain TUI-local unless a broader shared design is explicitly chosen later.

### 4.5 CLI JSONL streaming output

- `otto run` should support a machine-readable JSONL stream of Otto events.
- The JSONL stream exists to make real request/response/event behavior observable in tests and agentic implementation loops.
- It should complement, not replace, browser validation and higher-level SDK testing.

### 5. Wrapper and docs alignment

- Public binding surfaces and docs are part of the implementation boundary, not trailing cleanup.
- Python, JavaScript, and MCP should accurately describe the final conversation-loading semantics the core owns.

## Ordered Phase Plan

### Phase A: Core contract and models

- Define core model types for:
  - conversation summary rows,
  - bootstrap slices,
  - older-history pages,
  - checkpoint or replay metadata,
  - live stream events
- Replace any full-snapshot assumptions baked into `get_conversation()` or `otto_streaming()` call paths.
- Keep `get_conversation()` as the explicit full-materialization path.
- Add progressive read primitives for:
  - recent bootstrap
  - older-history paging
  - checkpoint-based reopen

### Phase B: TUI adoption

- Replace `get_conversation()`-based resume as the only reopen path.
- Add progressive history load behavior to the TUI.
- Decide whether checkpoint persistence is needed locally for acceptable reopen performance.

### Phase C: CLI and binding alignment

- Add `otto run` JSONL streaming output for machine-readable event traces.
- Update CLI conversation commands and documentation accordingly.
- Update Python, JavaScript, and MCP public surfaces so callers can understand the difference between summaries, bootstrap slices, and full materializations.

### Phase D: Validation and rollout

- Validate that all first-party consumers survive `thread.details` removal.
- Validate large-thread reopen behavior in the TUI.
- Validate docs and type surfaces together with implementation changes.

## Allowed Change Boundaries

- Allowed by default:
  - core client and model files
  - TUI reopen/resume logic
  - CLI/binding/MCP docs and public types
  - repo-local plan docs under this plan root
- Related but not owned here:
  - backend endpoint design and rollout sequencing
  - UI cache ownership details
- Out of bounds without explicit approval:
  - shared cross-consumer persistence store
  - unrelated CLI/TUI redesign

## Review-Readiness Conditions

- The plan is explicit about how large-thread open differs from summary list access.
- The contract direction after `thread.details` removal is documented in the core client plan instead of only in backend notes.
- Persistence remains an intentional decision, not an implied requirement.
- Wrapper docs and public types are called out as implementation scope so they cannot silently drift.
