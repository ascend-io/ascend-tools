# Otto Validation Surface Product Contract

## User Value

`ascend-tools` should give engineers and agents a faster, more readable Otto validation surface than a browser alone, while still staying public-repo-safe and avoiding downstream UI assumptions.

The repo should let a user:

- invoke Otto with explicit model and thinking selections
- observe streamed text, reasoning/tool progress, and stream terminal state
- validate backend behavior first through the public CLI/SDK surfaces, using raw REST only as a lower-level debugging fallback

## Actors

| Actor | Job to be done |
| --- | --- |
| Engineer or agent debugging Otto | Isolate whether a failure belongs to the backend contract or to a downstream UI consumer |
| CLI/TUI user | Run Otto quickly from a terminal and inspect structured stream behavior |
| SDK/MCP consumer | Build higher-level validation or automation on top of a structured Otto surface |

## Primary Journeys

### Journey 1: Public-surface check

1. User runs the Otto flow through the CLI or SDK.
2. User inspects the exact request/provenance plus streamed text, tool-call progress, and thinking-related signals.
3. If the public surface cannot expose enough detail for that inspection, the implementation is incomplete.

### Journey 2: Raw backend comparison

1. User runs a raw REST validation script after the public surface already showed a discrepancy or needs lower-level isolation.
2. User verifies whether the backend itself already returns the wrong data.
3. If raw REST passes but `ascend-tools` fails, the issue is in this repo's contract layer.

### Journey 3: Downstream UI escalation

1. `ascend-tools` behaves correctly and any supporting raw REST comparison agrees.
2. The downstream consumer still misbehaves.
3. The remaining issue is likely browser/UI-specific rather than backend-contract-specific.

## Visible / Exposed Contract

`ascend-tools` should expose enough Otto behavior to support debugging and validation in text-native surfaces:

- explicit model selection
- explicit thinking-level selection
- provenance metadata sufficient to prove what was exercised (`base URL` or instance, provider, model, explicit `thinking`, and thread/request identifiers where available)
- streamed text deltas
- reasoning-delta visibility
- tool-call start/output details
- progressive tool-call argument visibility when available
- stream terminal status and error classification
- at least one ordered event-inspection path that can show the same turn as a sequence rather than only a final response summary
- explicit unknown/raw-event surfacing when a provider emits a family that the normalized surface does not yet understand, so validation surfaces do not silently drop evidence

## Non-goals

- Reproducing a downstream UI's exact presentation
- Serving as the final proof layer for user-visible UX
- Depending on private-repo documentation or implementation details
