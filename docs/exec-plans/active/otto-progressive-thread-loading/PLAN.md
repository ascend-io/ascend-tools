# Otto Progressive Thread Loading

## Objective

Enable `ascend-tools` to work with large Otto thread lists and large conversations without depending on full thread snapshots or legacy `thread.details`, while keeping all first-party surfaces aligned through one Rust core implementation.

## Success Contract

- `ascend-tools-core` is the canonical client layer for summary listing, progressive thread bootstrap, checkpoint-based reopen, and live thread updates.
- No first-party `ascend-tools` surface requires legacy `thread.details`.
- TUI resume and long-thread viewing no longer assume a full `get_conversation()` snapshot before the user can interact.
- `otto run` can emit a machine-readable JSONL event stream that mirrors real Otto interactions closely enough to strengthen agentic testing and audit loops.
- CLI, Python, JavaScript, and MCP contract surfaces stay aligned with the Rust core and with the public docs and type surfaces.
- Persistence stays an explicit design choice. V1 may stay stateless or adopt TUI-local persistence only if the UX benefit is clear.

## Scope

### In scope

- `crates/ascend-tools-core`
- `crates/ascend-tools-tui`
- `crates/ascend-tools-cli`
- `crates/ascend-tools-py`
- `crates/ascend-tools-js`
- `crates/ascend-tools-mcp`
- public docs and public type surfaces for those consumers
- `otto run` JSONL streaming output for machine-readable testing and audit traces
- contract alignment with:
  - `ascend-backend/docs/exec-plans/active/otto-fast-thread-loading/`
  - `ascend-ui/docs/exec-plans/active/otto-ui-cache/`

### Out of scope by default

- Shared offline sync across all consumers
- Backend storage design beyond the client-facing contract it exposes
- Rewriting the TUI rendering model beyond what progressive thread loading or optional local persistence requires
- Unrelated CLI, runtime, or flow features

## Non-goals

- Making every `ascend-tools` consumer fully stateful
- Shipping a cross-process database cache unless it is explicitly chosen later
- Preserving compatibility with `thread.details`
- Fetching every thread summary or every message body up front on first open

## Current State

### Client architecture today

- `crates/ascend-tools-core/src/client.rs` is the one Otto transport implementation used by all first-party consumers.
- Conversation summaries already use `GET /api/v1/otto/threads` and `ConversationList`, but thread open still uses `get_conversation()` and fetches the full thread body.
- `otto_streaming()` already uses `/api/v1/otto/threads/{id}/updates`, but it only consumes granular live events and treats bootstrap events as irrelevant noise.
- CLI output today is limited to human-readable text or whole-object JSON; it does not yet expose a machine-readable event-by-event Otto stream for testing.
- `Conversation.messages` is optional in `crates/ascend-tools-core/src/models.rs`, which already reflects a summary-vs-body split, but not a page-by-page or checkpoint-aware thread-loading model.

### TUI state today

- `crates/ascend-tools-tui/src/lib.rs` resumes a conversation by calling `client.get_conversation(&tid)` in the background and converting the returned full thread body into a simplified local message list.
- The TUI persists input history to `~/.ascend-tools/history`.
- The TUI does not currently persist thread summaries, message bodies, or reopen checkpoints.

### Wrapper and docs state today

- Python, JavaScript, and MCP all ride the core client, but their docs and type surfaces lag the current conversation surface in places.
- The repo has one authoritative implementation path and several public contract surfaces that will need to stay in sync as progressive loading is introduced.

## Actor / Role Matrix

| Actor | Surface | Required outcome |
|---|---|---|
| CLI user | `otto run`, `otto conversation *` | Large conversations remain usable without hidden full-snapshot assumptions |
| TUI user | `otto tui` | Resume, reopen, and long-thread viewing stay responsive as threads grow |
| SDK user | Rust, Python, JavaScript | Public APIs can represent summary-only and progressively loaded conversation state |
| MCP consumer | `ascend-tools mcp` | Tools expose conversation access without silently relying on full thread snapshots |
| Maintainer | `ascend-tools-core` | One canonical contract feeds all first-party interfaces |

## Execution Phases

### Phase 1: Consume the shared thread contract

- Adopt the shared `/updates` lifecycle from the backend root.
- Lock the tools-side interpretation of:
  - paginated summary listing
  - recent bootstrap
  - separate paged older-history retrieval
  - `after=<message_id>` reopen anchors
  - `before=<message_id>` history-page anchors
- Keep v1 stateless unless TUI-local persistence proves necessary.

### Phase 2: Core transport and model layer

- Extend `ascend-tools-core` to represent summary state, progressive bootstrap state, catch-up delta state, and live thread events explicitly.
- Replace full-snapshot assumptions in conversation-open flows.
- Keep `get_conversation()` as the explicit full-materialization path.
- Add lower-level progressive read APIs for surfaces that need bootstrap, history paging, and checkpoint-based reopen without hidden full-thread hydration.

### Phase 3: TUI adoption

- Update the TUI to open or resume long threads without requiring a full body snapshot before render.
- If chosen, add TUI-local persistence for reopen checkpoints and/or recent thread state.
- Preserve current prompt-history behavior while separating it from thread-state persistence.

### Phase 4: First-party consumer alignment

- Align CLI, Python, JavaScript, and MCP surfaces with the new core contract.
- Add JSONL streaming output for `otto run` so agentic loops and tests can capture real event traces directly from the CLI.
- Update public docs and public type surfaces together with the implementation.

### Phase 5: Validation and rollout

- Validate long-thread bootstrap, checkpoint reopen, and send-while-viewing behavior.
- Validate that no first-party consumer requires `thread.details`.
- Align rollout with backend thread-details removal and UI contract changes.

## Needs Human Input

- None at plan time. The current plan is locked to:
  - stateless v1 by default
  - TUI-local persistence only if implementation evidence demands it
  - paginated summary listing in v1
  - explicit progressive read APIs alongside an explicit full-materialization path
  - CLI JSONL streaming output as part of the testing/audit surface

## Deferred / Follow-up Work

- Shared cross-consumer persistence beyond the TUI
- Full offline thread sync
- Richer typed rendering of tool calls, non-user/assistant history, and context metadata in the TUI
- Any future redesign of the public conversation model that is not necessary for progressive loading
