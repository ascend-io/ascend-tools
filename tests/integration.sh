#!/usr/bin/env bash
# Integration tests for the ascend-tools CLI.
# Requires a running ASE workspace and ASCEND_SERVICE_ACCOUNT_ID,
# ASCEND_SERVICE_ACCOUNT_KEY, and ASCEND_INSTANCE_API_URL set.
set -euo pipefail

CLI="uv run ascend-tools"
PASS=0
FAIL=0
SKIP=0

pass() { echo "  PASS: $1"; PASS=$((PASS + 1)); }
fail() { echo "  FAIL: $1 — $2"; FAIL=$((FAIL + 1)); }
skip() { echo "  SKIP: $1"; SKIP=$((SKIP + 1)); }

# Run `flow run` with retries for transient readiness states.
run_flow_retry() {
  local flow_name="$1"
  local runtime_uuid="$2"
  shift 2

  local out=""
  local rc=1
  local delay
  for delay in 0 5 10 15 15 15; do
    if [ "$delay" -gt 0 ]; then
      sleep "$delay"
    fi

    set +e
    out=$($CLI -o json flow run "$flow_name" --uuid "$runtime_uuid" "$@" 2>&1)
    rc=$?
    set -e

    if [ "$rc" -eq 0 ]; then
      echo "$out"
      return 0
    fi

    if echo "$out" | grep -qi "starting\|no health status\|initializing"; then
      continue
    fi

    echo "$out"
    return "$rc"
  done

  echo "$out"
  return "$rc"
}

# ---------- preflight ----------

echo "=== preflight ==="

for var in ASCEND_SERVICE_ACCOUNT_ID ASCEND_SERVICE_ACCOUNT_KEY ASCEND_INSTANCE_API_URL; do
  if [ -z "${!var:-}" ]; then
    echo "ERROR: $var is not set" >&2
    exit 1
  fi
done
pass "env vars set"

# ---------- workspaces ----------

echo "=== workspaces ==="

# list workspaces (text)
TEXT=$($CLI workspace list 2>&1)
if echo "$TEXT" | grep -Eq "^TITLE[[:space:]]+UUID[[:space:]]+HEALTH"; then
  pass "workspace list (text) has header"
else
  fail "workspace list (text)" "missing header row"
fi

# list workspaces (json)
JSON=$($CLI -o json workspace list 2>&1)
COUNT=$(echo "$JSON" | jq 'length')
if [ "$COUNT" -gt 0 ]; then
  pass "workspace list (json) returned $COUNT workspace(s)"
else
  skip "no workspaces found — skipping remaining tests"
  echo ""
  echo "=== results ==="
  TOTAL=$((PASS + FAIL + SKIP))
  echo "$PASS passed, $FAIL failed, $SKIP skipped (of $TOTAL)"
  [ "$FAIL" -gt 0 ] && exit 1
  echo "all tests passed"
  exit 0
fi

RUNTIME_UUID=$(echo "$JSON" | jq -r '.[0].uuid')
RUNTIME_TITLE=$(echo "$JSON" | jq -r '.[0].title')
echo "  using workspace: $RUNTIME_TITLE ($RUNTIME_UUID)"

# get workspace by title
GET_JSON=$($CLI -o json workspace get "$RUNTIME_TITLE" 2>&1)
GOT_UUID=$(echo "$GET_JSON" | jq -r '.uuid')
if [ "$GOT_UUID" = "$RUNTIME_UUID" ]; then
  pass "workspace get returns correct uuid"
else
  fail "workspace get" "expected $RUNTIME_UUID, got $GOT_UUID"
fi

# verify expected fields
for field in uuid id title kind project_uuid environment_uuid created_at updated_at; do
  VAL=$(echo "$GET_JSON" | jq -r ".$field")
  if [ "$VAL" != "null" ] && [ -n "$VAL" ]; then
    pass "workspace get has field '$field'"
  else
    fail "workspace get" "missing or null field '$field'"
  fi
done

# ---------- environments ----------

echo "=== environments ==="

ENV_JSON=$($CLI -o json environment list 2>&1)
ENV_COUNT=$(echo "$ENV_JSON" | jq 'length')
if [ "$ENV_COUNT" -gt 0 ]; then
  pass "environment list returned $ENV_COUNT environment(s)"
  ENV_TITLE=$(echo "$ENV_JSON" | jq -r '.[0].title')
  ENV_GET=$($CLI -o json environment get "$ENV_TITLE" 2>&1)
  GOT_ENV_TITLE=$(echo "$ENV_GET" | jq -r '.title')
  if [ "$GOT_ENV_TITLE" = "$ENV_TITLE" ]; then
    pass "environment get works"
  else
    fail "environment get" "expected $ENV_TITLE, got $GOT_ENV_TITLE"
  fi
else
  skip "no environments found"
fi

# ---------- projects ----------

echo "=== projects ==="

PROJ_JSON=$($CLI -o json project list 2>&1)
PROJ_COUNT=$(echo "$PROJ_JSON" | jq 'length')
if [ "$PROJ_COUNT" -gt 0 ]; then
  pass "project list returned $PROJ_COUNT project(s)"
  PROJ_TITLE=$(echo "$PROJ_JSON" | jq -r '.[0].title')
  PROJ_GET=$($CLI -o json project get "$PROJ_TITLE" 2>&1)
  GOT_PROJ_TITLE=$(echo "$PROJ_GET" | jq -r '.title')
  if [ "$GOT_PROJ_TITLE" = "$PROJ_TITLE" ]; then
    pass "project get works"
  else
    fail "project get" "expected $PROJ_TITLE, got $GOT_PROJ_TITLE"
  fi
else
  skip "no projects found"
fi

# ---------- profiles ----------

echo "=== profiles ==="

PROFILES=$($CLI -o json profile list --workspace "$RUNTIME_TITLE" 2>&1)
PROFILE_COUNT=$(echo "$PROFILES" | jq 'length')
if [ "$PROFILE_COUNT" -gt 0 ]; then
  pass "profile list returned $PROFILE_COUNT profile(s)"
else
  skip "no profiles found"
fi

# ---------- flows ----------

echo "=== flows ==="

FLOWS_JSON=$($CLI -o json flow list --uuid "$RUNTIME_UUID" 2>&1)
FLOW_COUNT=$(echo "$FLOWS_JSON" | jq 'length')
if [ "$FLOW_COUNT" -gt 0 ]; then
  pass "flow list returned $FLOW_COUNT flow(s)"
else
  skip "no flows found — skipping flow runs and trigger tests"
  echo ""
  echo "=== results ==="
  TOTAL=$((PASS + FAIL + SKIP))
  echo "$PASS passed, $FAIL failed, $SKIP skipped (of $TOTAL)"
  [ "$FAIL" -gt 0 ] && exit 1
  echo "all tests passed"
  exit 0
fi

FLOW_NAME=$(echo "$FLOWS_JSON" | jq -r '.[0].name')
echo "  using flow: $FLOW_NAME"

# ---------- flow runs (before) ----------

echo "=== flow runs (before trigger) ==="

RUNS_BEFORE=$($CLI -o json flow list-runs --uuid "$RUNTIME_UUID" --flow "$FLOW_NAME" 2>&1)
RUNS_BEFORE_COUNT=$(echo "$RUNS_BEFORE" | jq 'length')
pass "flow list-runs returned $RUNS_BEFORE_COUNT run(s) before trigger"

# test get-run on existing run
if [ "$RUNS_BEFORE_COUNT" -gt 0 ]; then
  EXISTING_RUN_NAME=$(echo "$RUNS_BEFORE" | jq -r '.[0].name')
  GET_RUN_JSON=$($CLI -o json flow get-run "$EXISTING_RUN_NAME" --uuid "$RUNTIME_UUID" 2>&1)
  GOT_RUN_NAME=$(echo "$GET_RUN_JSON" | jq -r '.name')
  if [ "$GOT_RUN_NAME" = "$EXISTING_RUN_NAME" ]; then
    pass "flow get-run returns correct run"
  else
    fail "flow get-run" "expected $EXISTING_RUN_NAME, got $GOT_RUN_NAME"
  fi

  for field in name flow status runtime_uuid build_uuid created_at; do
    VAL=$(echo "$GET_RUN_JSON" | jq -r ".$field")
    if [ "$VAL" != "null" ] && [ -n "$VAL" ]; then
      pass "flow get-run has field '$field'"
    else
      fail "flow get-run" "missing or null field '$field'"
    fi
  done
fi

# ---------- trigger flow run ----------

echo "=== trigger flow run ==="

set +e
TRIGGER_JSON=$(run_flow_retry "$FLOW_NAME" "$RUNTIME_UUID" --resume)
TRIGGER_RC=$?
set -e
if [ "$TRIGGER_RC" -ne 0 ]; then
  fail "flow run" "$TRIGGER_JSON"
  echo ""
  echo "=== results ==="
  TOTAL=$((PASS + FAIL + SKIP))
  echo "$PASS passed, $FAIL failed, $SKIP skipped (of $TOTAL)"
  exit 1
fi

EVENT_UUID=$(echo "$TRIGGER_JSON" | jq -r '.event_uuid')
EVENT_TYPE=$(echo "$TRIGGER_JSON" | jq -r '.event_type')

if [ -n "$EVENT_UUID" ] && [ "$EVENT_UUID" != "null" ]; then
  pass "flow run returned event_uuid: $EVENT_UUID"
else
  fail "flow run" "missing event_uuid"
fi

if [ "$EVENT_TYPE" = "ScheduleFlowRun" ]; then
  pass "flow run event_type is ScheduleFlowRun"
else
  fail "flow run" "unexpected event_type: $EVENT_TYPE"
fi

# ---------- flow runs (after) ----------

echo "=== flow runs (after trigger) ==="

RUNS_AFTER_COUNT="$RUNS_BEFORE_COUNT"
for delay in 2 3 5 5; do
  sleep "$delay"
  RUNS_AFTER=$($CLI -o json flow list-runs --uuid "$RUNTIME_UUID" --flow "$FLOW_NAME" 2>&1)
  RUNS_AFTER_COUNT=$(echo "$RUNS_AFTER" | jq 'length')
  if [ "$RUNS_AFTER_COUNT" -gt "$RUNS_BEFORE_COUNT" ]; then
    break
  fi
done

if [ "$RUNS_AFTER_COUNT" -gt "$RUNS_BEFORE_COUNT" ]; then
  pass "flow run count increased: $RUNS_BEFORE_COUNT -> $RUNS_AFTER_COUNT"
else
  skip "flow run not yet materialized after 15s (flow runner may be catching up)"
fi

NEWEST_RUN_NAME=$(echo "$RUNS_AFTER" | jq -r '.[0].name')
NEWEST_RUN_STATUS=$(echo "$RUNS_AFTER" | jq -r '.[0].status')
pass "newest run: $NEWEST_RUN_NAME (status: $NEWEST_RUN_STATUS)"

GET_NEW_RUN=$($CLI -o json flow get-run "$NEWEST_RUN_NAME" --uuid "$RUNTIME_UUID" 2>&1)
GOT_NEW_NAME=$(echo "$GET_NEW_RUN" | jq -r '.name')
if [ "$GOT_NEW_NAME" = "$NEWEST_RUN_NAME" ]; then
  pass "flow get-run on new run works"
else
  fail "flow get-run on new run" "expected $NEWEST_RUN_NAME, got $GOT_NEW_NAME"
fi

# ---------- spec ----------

echo "=== run_flow with spec ==="

set +e
SPEC_EMPTY=$(run_flow_retry "$FLOW_NAME" "$RUNTIME_UUID" --resume --spec '{}')
SPEC_EMPTY_RC=$?
set -e
if [ "$SPEC_EMPTY_RC" -eq 0 ] && echo "$SPEC_EMPTY" | jq -e '.event_uuid' > /dev/null 2>&1; then
  pass "flow run --spec '{}' works"
else
  fail "flow run --spec '{}'" "$SPEC_EMPTY"
fi

# ---------- workspace pause/resume ----------

echo "=== workspace pause ==="

PAUSE_JSON=$($CLI -o json workspace pause "$RUNTIME_TITLE" 2>&1)
PAUSED=$(echo "$PAUSE_JSON" | jq -r '.paused')
if [ "$PAUSED" = "true" ]; then
  pass "workspace pause sets paused=true"
else
  fail "workspace pause" "expected paused=true, got $PAUSED"
fi

# flow run without --resume should fail
PAUSED_ERR=$($CLI -o json flow run "$FLOW_NAME" --uuid "$RUNTIME_UUID" 2>&1 || true)
if echo "$PAUSED_ERR" | grep -qi "paused\|resume\|no health status\|initializing\|starting"; then
  pass "flow run on paused workspace fails with descriptive error"
else
  fail "flow run on paused workspace" "expected state error, got: $PAUSED_ERR"
fi

echo "=== workspace resume ==="

set +e
RESUME_TRIGGER=$(run_flow_retry "$FLOW_NAME" "$RUNTIME_UUID" --resume)
RESUME_TRIGGER_RC=$?
set -e
if [ "$RESUME_TRIGGER_RC" -eq 0 ] && echo "$RESUME_TRIGGER" | jq -e '.event_uuid' > /dev/null 2>&1; then
  pass "flow run --resume succeeds"
else
  fail "flow run --resume" "$RESUME_TRIGGER"
fi

AFTER_RESUME=$($CLI -o json workspace get "$RUNTIME_TITLE" 2>&1)
PAUSED_AFTER=$(echo "$AFTER_RESUME" | jq -r '.paused')
if [ "$PAUSED_AFTER" = "false" ]; then
  pass "workspace is unpaused after --resume"
else
  fail "workspace after --resume" "expected paused=false, got $PAUSED_AFTER"
fi

# wait for health to restore
for delay in 2 3 5 5; do
  sleep "$delay"
  HEALTH=$($CLI -o json workspace get "$RUNTIME_TITLE" 2>&1 | jq -r '.health')
  [ "$HEALTH" != "null" ] && break
done
if [ "$HEALTH" != "null" ]; then
  pass "workspace health restored: $HEALTH"
else
  skip "workspace health not yet available after 15s"
fi

# resume on already-running workspace should be idempotent
RESUME_IDEM=$($CLI -o json workspace resume "$RUNTIME_TITLE" 2>&1)
PAUSED_IDEM=$(echo "$RESUME_IDEM" | jq -r '.paused')
if [ "$PAUSED_IDEM" = "false" ]; then
  pass "workspace resume is idempotent"
else
  fail "workspace resume idempotent" "expected paused=false, got $PAUSED_IDEM"
fi

# ---------- summary ----------

echo ""
echo "=== results ==="
TOTAL=$((PASS + FAIL + SKIP))
echo "$PASS passed, $FAIL failed, $SKIP skipped (of $TOTAL)"
if [ "$FAIL" -gt 0 ]; then
  echo "$FAIL FAILED"
  exit 1
fi
echo "all tests passed"
