# Otto Progressive Thread Loading

## Validation Matrix

| Risk area | Automated proof | Manual / integrated proof |
|---|---|---|
| Core conversation listing | Core HTTP tests for bounded summary list behavior | List recent conversations as counts grow and confirm message bodies are not fetched eagerly |
| Integrated primary lifecycle | Maintained core/client proof that can open through `/updates`, page older history through `before=<message_id>`, reopen through `after=<message_id>`, and terminate through `thread.done` | Run the same lifecycle against a deterministic known-long-thread harness and confirm the tools surfaces witness every phase instead of only a short-thread happy path |
| Core thread bootstrap | Core transport/model tests for recent bootstrap and catch-up semantics | Open a large thread and confirm it does not require a blocking full-thread snapshot |
| Checkpoint reopen | Tests for message-ID-based checkpoint catch-up behavior | Reopen a long conversation and confirm it resumes from the correct `after=<message_id>` anchor |
| Older-history paging | Core tests for separate paged history retrieval using `before=<message_id>` | Page older history and confirm deterministic expansion of the local thread without requiring full initial hydration |
| TUI resume | TUI-focused tests for partial history and reopen flows, including stale/late history handling | Reopen a large conversation and confirm useful recent history appears before older history completes, with no duplicate or stale error bleed-through |
| CLI / SDK / wrapper contract | Tests and surface assertions for Rust/Python/JS/MCP APIs | Validate public surfaces and docs against the final implementation contract, including which surfaces do or do not expose progressive/raw-event semantics |
| Legacy bootstrap removal | Core tests proving `thread.details` is not required | Validate that no first-party `ascend-tools` flow depends on the removed bootstrap event |
| CLI JSONL trace output | CLI tests for event-by-event JSONL output shape, anchors, and ordering | Run `otto run` in JSONL mode and confirm the trace exposes the same contract the UI/raw API depend on, not merely a machine-readable subset |
| Provider / model / parameter matrix | Representative tools-side proof on default plus non-default request variants when capability differs | Repeat the lifecycle with explicit provider/model selection, resume/conversation targeting, and other behavior-shaping parameters that a careful human would obviously try |
| Error and recovery behavior | Focused tests for stale anchors, interrupted streams, and unhappy terminal branches on tools-side surfaces | Confirm CLI/TUI/wrapper behavior remains honest on invalid `before`/`after`, interrupted streams, reload/reconnect, and late history errors |

## Required Checks

- `bin/check`

## Current Workspace-Proven Commands

Verified in the `OttoClientCaching` workspace using the repo-local invocation form:

```bash
export ASCEND_INSTANCE_API_URL="https://ottoclientcaching-instance.api.local.ascend.dev"
THREAD_ID=019d3d5b-0daf-7020-82a0-14fc13bf1e64
BEFORE_ID=msg_c8bb07906288cb50
LATEST_BEFORE_FOLLOWUP=msg_0084ab438b55c4c40069cb330def6c81908bd535e5f35673ca
# Observed thread.done latest_message_id from the validated follow-up run:
LATEST_AFTER_FOLLOWUP=msg_01bb3ab25e4454b00069cb3bb2696c81939c5abeaff74e9f82

uv run --project ascend-tools ascend-tools otto conversation list --limit 5
uv run --project ascend-tools ascend-tools -o json otto conversation open "$THREAD_ID" --id
uv run --project ascend-tools ascend-tools -o json otto conversation history "$THREAD_ID" --id --before "$BEFORE_ID" --limit 5
uv run --project ascend-tools ascend-tools otto run "Reply with exactly 'plan-proof'." --provider OpenAI --model gpt-5.2 --conversation "$THREAD_ID" --jsonl
uv run --project ascend-tools ascend-tools -o json otto conversation open "$THREAD_ID" --id --after "$LATEST_BEFORE_FOLLOWUP"
ASCEND_INSTANCE_API_URL="https://ottoclientcaching-instance.app.local.ascend.dev" uv run --project ascend-tools ascend-tools -o json otto conversation list --limit 1
```

Notes:

- The last command is the exact local-dev host-normalization proof: it intentionally supplies the mistaken `instance.app.local` host and succeeds through the shared config rewrite to `instance.api.local`.
- Use the repo-local `uv run --project ascend-tools ascend-tools ...` form in this workspace instead of assuming `ascend-tools` is on `PATH`.

## Manual Scenarios

### Conversation list

1. List recent conversations without pulling message bodies.
2. Verify pagination or bounded list behavior remains usable as conversation count grows.

### TUI reopen

1. Launch the TUI against a deterministic known-long-thread conversation that will exceed the preview window.
2. Confirm recent history appears before the entire thread body is fetched.
3. Confirm older history pages through `before=<message_id>` rather than hidden full-thread hydration.
4. Confirm new messages continue to stream normally.
5. Confirm stale or late history failures do not bleed into a newer active generation.

### Continue an existing conversation

1. Use CLI or SDK resume flows against a long thread.
2. Confirm send succeeds without requiring a full-history hydration first.
3. Repeat with explicit provider/model selection and conversation-targeting parameters a careful human would obviously try.

### JSONL audit trace

1. Run `otto run` in JSONL mode against a real conversation flow.
2. Capture the event stream to disk.
3. Confirm the trace is machine-readable, ordered, and rich enough to support fixture generation and agentic debugging.
4. Confirm the trace exposes the anchors, event families, terminal events, and ordering needed to compare against raw API and UI behavior.

### Same-lifecycle parity

1. Exercise one known thread through raw API, tools JSONL, and the relevant browser/UI proof path.
2. Confirm the same lifecycle semantics are preserved:
   - recent bootstrap
   - older-history paging
   - reopen anchor
   - `thread.done`
3. If a wrapper surface cannot express or inspect the same contract, record that gap explicitly instead of counting it as parity.

### Public surface alignment

1. Validate Python, JavaScript, and MCP surfaces against the updated core contract.
2. Validate public docs and type surfaces against the final implementation.
3. Confirm each public surface is explicit about whether it exposes summary-only, progressive lifecycle, raw event trace, or full-materialization behavior.

### Error and recovery

1. Force stale or invalid `after` anchors and confirm honest fallback or error handling.
2. Force invalid `before` anchors or interrupted streams and confirm the surface fails honestly.
3. Confirm reconnect/retry behavior after `thread.done` or other terminal branches is explicit and correct.

## Rollout And Sequencing

- Coordinate rollout with backend removal of `thread.details`.
- Do not remove legacy bootstrap support in backend until tools-side consumers have an agreed replacement contract.
- If local persistence is added, keep it scoped and documented as a TUI design choice rather than a silent repo-wide cache layer.

## Rollback / Stop Conditions

- If the core contract is still ambiguous after backend endpoint changes, stop and resolve the transport model before updating wrappers.
- If TUI reopen still requires full-body hydration to be usable, stop and revisit whether local checkpoint persistence is required.
- If public docs and types drift from the implementation, stop and update them before declaring the plan review-ready.
- If the proof harness never actually forces `before=<message_id>` on a known long thread, stop and fix the harness instead of claiming older-history validation.
- If JSONL or wrapper surfaces cannot expose the same anchors, event families, or ordering the UI/raw API depend on, stop and carry that as missing parity proof rather than declaring cross-surface validation complete.
