# Otto Progressive Thread Loading

## Validation Matrix

| Risk area | Automated proof | Manual / integrated proof |
|---|---|---|
| Core conversation listing | Core HTTP tests for bounded summary list behavior | List recent conversations as counts grow and confirm message bodies are not fetched eagerly |
| Core thread bootstrap | Core transport/model tests for recent bootstrap and catch-up semantics | Open a large thread and confirm it does not require a blocking full-thread snapshot |
| Checkpoint reopen | Tests for message-ID-based checkpoint catch-up behavior | Reopen a long conversation and confirm it resumes from the correct `after=<message_id>` anchor |
| Older-history paging | Core tests for separate paged history retrieval using `before=<message_id>` | Page older history and confirm deterministic expansion of the local thread without requiring full initial hydration |
| TUI resume | TUI-focused tests for partial history and reopen flows | Reopen a large conversation and confirm useful recent history appears before older history completes |
| CLI and SDK contract | Tests and surface assertions for Rust/Python/JS/MCP APIs | Validate public surfaces and docs against the final implementation contract |
| Legacy bootstrap removal | Core tests proving `thread.details` is not required | Validate that no first-party `ascend-tools` flow depends on the removed bootstrap event |
| CLI JSONL trace output | CLI tests for event-by-event JSONL output shape and ordering | Run `otto run` in JSONL mode and confirm the trace is usable for test fixtures and agentic audits |

## Required Checks

- `bin/check`

## Manual Scenarios

### Conversation list

1. List recent conversations without pulling message bodies.
2. Verify pagination or bounded list behavior remains usable as conversation count grows.

### TUI reopen

1. Launch the TUI against a long existing conversation.
2. Confirm recent history appears before the entire thread body is fetched.
3. Confirm new messages continue to stream normally.

### Continue an existing conversation

1. Use CLI or SDK resume flows against a long thread.
2. Confirm send succeeds without requiring a full-history hydration first.

### JSONL audit trace

1. Run `otto run` in JSONL mode against a real conversation flow.
2. Capture the event stream to disk.
3. Confirm the trace is machine-readable, ordered, and rich enough to support fixture generation and agentic debugging.

### Public surface alignment

1. Validate Python, JavaScript, and MCP surfaces against the updated core contract.
2. Validate public docs and type surfaces against the final implementation.

## Rollout And Sequencing

- Coordinate rollout with backend removal of `thread.details`.
- Do not remove legacy bootstrap support in backend until tools-side consumers have an agreed replacement contract.
- If local persistence is added, keep it scoped and documented as a TUI design choice rather than a silent repo-wide cache layer.

## Rollback / Stop Conditions

- If the core contract is still ambiguous after backend endpoint changes, stop and resolve the transport model before updating wrappers.
- If TUI reopen still requires full-body hydration to be usable, stop and revisit whether local checkpoint persistence is required.
- If public docs and types drift from the implementation, stop and update them before declaring the plan review-ready.
