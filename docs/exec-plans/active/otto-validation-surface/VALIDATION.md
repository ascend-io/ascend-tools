# Otto Validation Surface Validation

## Validation Goals

- Prove `ascend-tools` can carry explicit thinking selection through its Otto request surfaces
- Prove `ascend-tools` exposes enough stream information to help isolate backend vs consumer failures
- Prove raw REST and higher-level SDK/CLI/TUI/MCP paths tell a coherent debugging story

## Validation Matrix

| Scenario | What must be true |
| --- | --- |
| Raw REST check | backend behavior can be exercised without SDK dependency |
| SDK check | public SDK surface preserves the intended Otto request and response contract |
| CLI check | CLI can issue the relevant Otto requests and report useful output |
| TUI check | streamed behavior is inspectable in a text-native UI |
| Thinking selection | explicit thinking level can be passed through request surfaces |
| Tool progress | tool-call progress is exposed well enough to support backend isolation |

## Local Validation Gate

- `bin/check`
- targeted Rust tests for Otto request/stream behavior
- targeted integration tests under `tests/`

## Manual Scenarios

### 1. Raw REST path

- Use `tests/rest.py` or `tests/rest.js`
- Verify the backend contract directly

### 2. SDK path

- Use `tests/integration.py`
- Verify SDK request/response behavior

### 3. CLI path

- Use `tests/integration.sh`
- Verify CLI request flow and output shape

### 4. TUI / streaming path

- Run `ascend-tools otto tui`
- Verify that streamed behavior remains inspectable without a browser

## Rollout / Sequencing

- This repo supplements downstream consumer testing; it does not replace it.
- Keep docs generic and public-repo-safe.

## Stop Conditions

- Stop if the repo still cannot distinguish backend contract failures from higher-level consumer failures.
- Stop if the added validation story depends on private-repo implementation details rather than public Otto contract behavior.
