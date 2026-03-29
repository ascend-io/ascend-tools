# Otto Validation Surface Product Contract

## User Value

`ascend-tools` should give engineers and agents a faster, more readable Otto validation surface than a browser alone, while still staying public-repo-safe and avoiding downstream UI assumptions.

The repo should let a user:

- invoke Otto with explicit model and thinking selections
- observe streamed text, reasoning/tool progress, and stream terminal state
- validate backend behavior using raw REST, SDK, CLI, TUI, or MCP surfaces

## Actors

| Actor | Job to be done |
| --- | --- |
| Engineer or agent debugging Otto | Isolate whether a failure belongs to the backend contract or to a downstream UI consumer |
| CLI/TUI user | Run Otto quickly from a terminal and inspect structured stream behavior |
| SDK/MCP consumer | Build higher-level validation or automation on top of a structured Otto surface |

## Primary Journeys

### Journey 1: Raw backend check

1. User runs a raw REST validation script.
2. User verifies whether the backend itself already returns the wrong data.
3. If raw REST fails, the issue is backend-side.

### Journey 2: SDK/CLI/TUI check

1. User runs the Otto flow through `ascend-tools`.
2. User inspects streamed text, tool-call progress, and thinking-related signals.
3. If raw REST passes but `ascend-tools` fails, the issue is in this repo's contract layer.

### Journey 3: Downstream UI escalation

1. Raw REST and `ascend-tools` both behave correctly.
2. The downstream consumer still misbehaves.
3. The remaining issue is likely browser/UI-specific rather than backend-contract-specific.

## Visible / Exposed Contract

`ascend-tools` should expose enough Otto behavior to support debugging and validation in text-native surfaces:

- explicit model selection
- explicit thinking-level selection
- streamed text deltas
- tool-call start/output details
- progressive tool-call argument visibility when available
- stream terminal status and error classification

## Non-goals

- Reproducing a downstream UI's exact presentation
- Serving as the final proof layer for user-visible UX
- Depending on private-repo documentation or implementation details
