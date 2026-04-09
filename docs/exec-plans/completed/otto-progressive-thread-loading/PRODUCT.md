# Otto Progressive Thread Loading

## User Value

Otto conversations should stay usable in terminal and SDK surfaces as conversation counts and message counts grow. Users should not have to wait for every message in a long thread to download before they can reopen it, resume it, or continue working.

## Actors And Permissions

| Actor | Responsibility | Constraint |
|---|---|---|
| CLI user | Lists conversations, inspects a thread, continues a conversation | Should not need to understand transport-level bootstrap or checkpoint semantics |
| TUI user | Reopens a thread, watches live output, resumes later | Should get responsive history access even when the thread is large |
| SDK user | Uses Rust, Python, or JavaScript client methods | Needs a stable public model that can represent summary-only and progressively loaded state |
| MCP consumer | Calls conversation tools through the MCP server | Needs contract-consistent behavior without hand-maintaining transport details |
| Core client maintainer | Evolves the shared Rust transport/model layer | Must avoid per-language drift |

## User Experience Contract

### Conversation list

- Listing conversations should return recent summaries without downloading thread bodies.
- The list flow should remain paginated or otherwise bounded as thread counts grow.

### Conversation open

- Opening or resuming a conversation should no longer imply “download the full thread body first.”
- The client should be able to show recent context first, then fetch older history progressively as needed.
- If a checkpoint is available, the reopen path should catch up from that checkpoint instead of treating the thread as brand new.
- If a checkpoint is stale or unusable, the client should fall back to a safe recent bootstrap path rather than surfacing broken or partial thread state.

### Continue a conversation

- Sending a new prompt into an existing conversation should not require a full history re-fetch first.
- Live output should continue to stream through the current granular event path.
- Long-running conversations should remain resumable even after the thread grows beyond what is reasonable to fetch as one snapshot.

### Machine-readable CLI streaming

- `otto run` should be able to emit a JSONL stream of Otto events for testing, auditing, and agentic implementation loops.
- That output should make real API behavior observable without requiring the browser for every regression.
- It is a testing and audit surface, not a replacement for browser validation.

### Consumer-specific expectations

- CLI may stay mostly stateless as long as explicit “inspect conversation” flows can page or stream responsibly.
- TUI is the main reopen/resume surface and is therefore the main candidate for local checkpoint storage if persistence is chosen.
- Python, JavaScript, and MCP should not invent separate semantics; they should expose whatever contract the Rust core owns.

## Primary Journeys

### List conversations

1. User asks for recent conversations.
2. Client receives summaries only.
3. User can choose one conversation without having paid the cost of full thread-body download for every row.

### Reopen a large thread in the TUI

1. User launches `otto tui --conversation ...`.
2. Client loads the most relevant recent history first.
3. Older history becomes available without blocking first render.
4. New output continues on the same live stream.

### Continue a large thread from the CLI

1. User runs `otto run ... --conversation ...`.
2. Client resolves the target conversation efficiently.
3. Send continues on the existing thread without requiring a full body hydration first.

### Audit a real invocation from the CLI

1. User runs `otto run ...` in JSONL mode.
2. Each event is emitted as structured machine-readable output.
3. The trace can be saved, diffed, and used in tests or agentic debugging loops.

### SDK/MCP conversation access

1. Caller lists conversations or fetches one by title/ID.
2. The public contract makes clear whether the returned state is a summary, a bootstrap slice, or a fully materialized thread.

## Visible States And Transitions

| Surface | States |
|---|---|
| CLI list | page loaded, empty, next page available, error |
| TUI thread | recent history ready, older history loading, catch-up from checkpoint, stale-checkpoint fallback, live streaming, interrupted, error |
| SDK/MCP response | summary-only, bootstrap page, live stream, complete materialization if explicitly requested |
| `otto run` JSONL | event stream, terminal success, terminal error, saved trace |

## Acceptance Scenes

- A long conversation can be reopened in the TUI without a blocking full-history fetch.
- Continuing a conversation through CLI or SDK does not require hidden “get the entire thread body first” work.
- Python, JavaScript, and MCP surfaces describe the new conversation-loading behavior accurately.
- Removing `thread.details` does not break any first-party `ascend-tools` flow.
- JSONL CLI traces are rich enough to support backend/client testing and agentic audit loops.

## Non-goals

- Full offline-first conversation sync in v1
- Cross-consumer shared local database storage in v1 by default
- Perfect feature parity between stateless SDKs and the stateful TUI if their UX needs differ
