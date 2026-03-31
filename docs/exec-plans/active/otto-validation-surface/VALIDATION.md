# Otto Validation Surface Validation

## Validation Goals

- Prove `ascend-tools` can carry explicit thinking selection through its current-wave Otto request surfaces.
- Prove the CLI preserves enough ordered stream evidence and request provenance to isolate reasoning/tool-progress failures before a human has to open the browser.
- Prove raw REST remains only a supporting/debugging layer rather than the primary substitute for a missing public validation surface.
- Prove the CLI and any supporting lower-level surface tell a coherent debugging story for the same prompt family.
- Keep TUI and MCP honest: they only receive current-wave proof credit if they can send explicit `thinking` and expose the minimum ordered-event contract.
- Prove environment authority and provenance are recorded so mis-targeted runs do not masquerade as product regressions.

## Validation Matrix

| Surface / asset | What must be true now | Current-wave gate |
| --- | --- | --- |
| Raw REST (`tests/rest.js`) | exact request payload and raw SSE events remain available for backend-only debugging, but this path does not replace missing CLI/SDK proof | supporting only |
| Python SDK (`tests/integration.py`) | public SDK preserves request fields and the final response contract; if stream-order visibility is absent, the limitation is recorded explicitly | required for request contract; not sufficient alone for ordered-event proof |
| CLI (`tests/integration.sh` and `ascend-tools otto run ... --jsonl`) | explicit `thinking` reaches the request, request provenance is visible, raw ordered event families are preserved, and terminal status/error is machine-checkable | required |
| CLI one-shot completion semantics | a one-shot `otto run` exits cleanly when the first `thread.details` / thread snapshot already shows `is_processing=false`, instead of hanging on heartbeat pings | required |
| Core parser (`crates/ascend-tools-core/tests/client_http.rs`) | deterministic request serialization, ordered event-family parsing, terminal classification, and interruption handling remain correct | required |
| TUI (`crates/ascend-tools-tui/src/lib.rs`) | only counts after explicit `thinking` and the minimum ordered-event contract are implemented | deferred |
| MCP (`crates/ascend-tools-mcp/src/server.rs`) | only counts after explicit `thinking` and the minimum ordered-event contract are implemented | deferred |

## Local Validation Gate

- `bin/check`
- targeted Rust tests for Otto request/stream behavior in `crates/ascend-tools-core/tests/client_http.rs`
- `node tests/rest.js`
- `uv run --script tests/integration.py`
- `bash tests/integration.sh`

## Environment Authority / Provenance

- Record the base URL or instance being exercised, the workspace/runtime identity, provider, model, explicit `thinking` value, and resulting thread/request identifiers.
- Record whether the command used the current workspace binary or an installed binary.
- When comparing surfaces, use the same prompt family and preserve that provenance across CLI, SDK, any supporting raw REST probe, and any later browser proof.
- If a surface cannot echo enough provenance to prove what it exercised, do not count it as full proof for isolation decisions.
- If QA has to drop to direct raw REST because the CLI still cannot show exact request payloads or ordered events, treat that as an implementation gap rather than successful validation.

## Manual Scenarios

### 1. Raw REST path

- Use `tests/rest.js`
- Use it only after the public surface has already been exercised or when debugging a discrepancy.
- Verify the backend contract directly with:
  - explicit `thinking` selection
  - ordered SSE event names
  - terminal event or error classification
  - recorded thread/provenance metadata

### 2. SDK path

- Use `tests/integration.py`
- Verify SDK request/response behavior
- If the SDK path does not expose ordered stream detail, record that limitation explicitly instead of crediting it for stream-order proof

### 3. CLI path

- Use `tests/integration.sh`
- Use `ascend-tools otto run ... --jsonl`
- Verify CLI request flow, explicit `thinking`, request provenance output, raw ordered event payloads, thread identifiers, and terminal status/error
- Include at least one substantial prompt that predictably exercises reasoning and/or tool use; do not treat `ping`-class prompts as sufficient coverage for streaming correctness by themselves
- Verify the one-shot command exits correctly when the updates stream starts with a completed thread snapshot instead of a later `thread.done`

### 4. Cross-surface parity

- Run the same prompt family through the CLI first, then add raw REST only if lower-level isolation is needed
- Verify request parameters, ordered event families, terminal status, and persisted identifiers tell the same story before using the browser as the next debugging surface
- At least one shared prompt family must be non-trivial enough to force the brittle path under review, for example reasoning plus tool use or larger streamed output rather than a short final-only answer

### 5. Error and terminal-state path

- Exercise one invalid or unsupported thinking selection
- Exercise one interrupted or `response.error` path
- Verify the chosen surface exposes a structured failure or terminal classification instead of collapsing everything to opaque final text

### 6. Deferred surfaces honesty

- Do not count TUI or MCP as current-wave validation gates until they can both send exact `thinking` values and expose the minimum ordered-event contract
- If they are inspected anyway, record them as exploratory or follow-up evidence rather than merge-gating proof

## Rollout / Sequencing

- This repo supplements downstream consumer testing; it does not replace it.
- The CLI should be exercised before browser/UI debugging is treated as the next honest move.
- Raw REST is a lower-level comparison tool after the CLI/SDK path, not the default proof path when the public validation surface is still underpowered.
- Same-case parity across surfaces is a prerequisite for crediting `ascend-tools` as a backend-isolation layer.
- TUI and MCP stay explicit follow-up surfaces unless their request and ordered-event parity is implemented and proven.
- Keep docs generic and public-repo-safe.

## Stop Conditions

- Stop if the repo still cannot distinguish backend contract failures from higher-level consumer failures.
- Stop if the CLI cannot show the exact sent request payload or the ordered event families needed for the question.
- Stop if the credited proof prompts are so trivial that they avoid the exact reasoning, tool-call, or larger-stream path that the human would obviously try next.
- Stop if QA is relying on raw REST because the CLI/SDK surface still cannot reproduce the same prompt family with matching provenance.
- Stop if a surface is being credited as proof even though it cannot show the exact sent `thinking` value or the ordered event families needed for the question.
- Stop if the validation story depends on TUI or MCP while they still send `thinking: None`.
- Stop if the added validation story depends on private-repo implementation details rather than public Otto contract behavior.
