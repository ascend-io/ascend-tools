# tests

Integration tests and demo applications. Unit tests live in the crates themselves (`crates/*/tests/`).

| Path | Description |
|------|-------------|
| `rest.py` | Self-contained REST API test — raw HTTP + Ed25519 JWT, no SDK dependency |
| `rest.js` | Same as `rest.py` in Node.js — zero npm dependencies |
| `integration.py` | SDK integration tests (Python `ascend_tools.Client`) |
| `integration.sh` | CLI integration tests (`ascend-tools` binary) |
| `app/` | Demo htmx app exercising the JS SDK (workspaces, flows, Otto chat) |
