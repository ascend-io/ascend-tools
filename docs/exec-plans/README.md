# Execution Plans

Work plans for non-trivial engineering efforts. Check `active/` before starting multi-step work.

## Structure

- `active/` — plans currently in progress
- `completed/` — finished plans kept for future context

## Preferred Format

Active work in this repo should use ASE directory-shaped plan roots:

```text
docs/exec-plans/active/<plan_slug>/
  PLAN.md
  PRODUCT.md
  IMPLEMENTATION.md
  DECISIONS.md
  VALIDATION.md
```

Private ASE runtime state lives at the workspace root:

```text
.ase/<plan_slug>/
```

## Current Focus

- `active/otto-progressive-thread-loading/` — adopt progressive Otto thread loading across `ascend-tools-core`, TUI, CLI, Python, JavaScript, and MCP surfaces
