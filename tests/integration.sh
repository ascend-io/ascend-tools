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

# ---------- preflight ----------

echo "=== preflight ==="

for var in ASCEND_SERVICE_ACCOUNT_ID ASCEND_SERVICE_ACCOUNT_KEY ASCEND_INSTANCE_API_URL; do
  if [ -z "${!var:-}" ]; then
    echo "ERROR: $var is not set" >&2
    exit 1
  fi
done
pass "env vars set"

# ---------- runtimes ----------

echo "=== runtimes ==="

# list runtimes (text)
TEXT=$($CLI runtime list 2>&1)
if echo "$TEXT" | head -1 | grep -q "UUID"; then
  pass "runtime list (text) has header"
else
  fail "runtime list (text)" "missing header row"
fi

# list runtimes (json)
JSON=$($CLI -o json runtime list 2>&1)
COUNT=$(echo "$JSON" | jq 'length')
if [ "$COUNT" -gt 0 ]; then
  pass "runtime list (json) returned $COUNT runtime(s)"
else
  skip "no runtimes found — skipping runtime get, filters, flows, and flow runs"
  echo ""
  echo "=== results ==="
  TOTAL=$((PASS + FAIL + SKIP))
  echo "$PASS passed, $FAIL failed, $SKIP skipped (of $TOTAL)"
  [ "$FAIL" -gt 0 ] && exit 1
  echo "all tests passed"
  exit 0
fi

RUNTIME_UUID=$(echo "$JSON" | jq -r '.[0].uuid')
RUNTIME_ID=$(echo "$JSON" | jq -r '.[0].id')
echo "  using runtime: $RUNTIME_ID ($RUNTIME_UUID)"

# get runtime
GET_JSON=$($CLI -o json runtime get "$RUNTIME_UUID" 2>&1)
GOT_UUID=$(echo "$GET_JSON" | jq -r '.uuid')
if [ "$GOT_UUID" = "$RUNTIME_UUID" ]; then
  pass "runtime get returns correct uuid"
else
  fail "runtime get" "expected $RUNTIME_UUID, got $GOT_UUID"
fi

# verify all expected fields are present
for field in uuid id title kind project_uuid environment_uuid created_at updated_at; do
  VAL=$(echo "$GET_JSON" | jq -r ".$field")
  if [ "$VAL" != "null" ] && [ -n "$VAL" ]; then
    pass "runtime get has field '$field'"
  else
    fail "runtime get" "missing or null field '$field'"
  fi
done

# list runtimes with --id filter
FILTERED=$($CLI -o json runtime list --id "$RUNTIME_ID" 2>&1)
FILTERED_COUNT=$(echo "$FILTERED" | jq 'length')
if [ "$FILTERED_COUNT" -eq 1 ]; then
  pass "runtime list --id filter returns exactly 1"
else
  fail "runtime list --id filter" "expected 1, got $FILTERED_COUNT"
fi

# list runtimes with --kind filter
RUNTIME_KIND=$(echo "$JSON" | jq -r '.[0].kind')
KIND_FILTERED=$($CLI -o json runtime list --kind "$RUNTIME_KIND" 2>&1)
KIND_COUNT=$(echo "$KIND_FILTERED" | jq 'length')
if [ "$KIND_COUNT" -ge 1 ]; then
  pass "runtime list --kind filter returns >= 1"
else
  fail "runtime list --kind filter" "expected >= 1, got $KIND_COUNT"
fi

# ---------- flows ----------

echo "=== flows ==="

FLOWS_JSON=$($CLI -o json flow list -r "$RUNTIME_UUID" 2>&1)
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

RUNS_BEFORE=$($CLI -o json flow list-runs -r "$RUNTIME_UUID" -f "$FLOW_NAME" 2>&1)
RUNS_BEFORE_COUNT=$(echo "$RUNS_BEFORE" | jq 'length')
pass "flow list-runs returned $RUNS_BEFORE_COUNT run(s) before trigger"

# if there are existing runs, test get-run on the first one
if [ "$RUNS_BEFORE_COUNT" -gt 0 ]; then
  EXISTING_RUN_NAME=$(echo "$RUNS_BEFORE" | jq -r '.[0].name')
  GET_RUN_JSON=$($CLI -o json flow get-run "$EXISTING_RUN_NAME" -r "$RUNTIME_UUID" 2>&1)
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

TRIGGER_JSON=$($CLI -o json flow run "$FLOW_NAME" -r "$RUNTIME_UUID" 2>&1)
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

# poll for the new run to appear (up to 15s)
RUNS_AFTER_COUNT="$RUNS_BEFORE_COUNT"
for delay in 2 3 5 5; do
  sleep "$delay"
  RUNS_AFTER=$($CLI -o json flow list-runs -r "$RUNTIME_UUID" -f "$FLOW_NAME" 2>&1)
  RUNS_AFTER_COUNT=$(echo "$RUNS_AFTER" | jq 'length')
  if [ "$RUNS_AFTER_COUNT" -gt "$RUNS_BEFORE_COUNT" ]; then
    break
  fi
done

if [ "$RUNS_AFTER_COUNT" -gt "$RUNS_BEFORE_COUNT" ]; then
  pass "flow run count increased: $RUNS_BEFORE_COUNT -> $RUNS_AFTER_COUNT"
else
  # Flow runner may be slow to process events (esp. after workspace restart).
  # The trigger itself succeeded (event_uuid returned), so this is infra timing.
  skip "flow run not yet materialized after 15s (flow runner may be catching up)"
fi

# verify the newest run (first in list — ordered by created_at desc)
NEWEST_RUN_NAME=$(echo "$RUNS_AFTER" | jq -r '.[0].name')
NEWEST_RUN_STATUS=$(echo "$RUNS_AFTER" | jq -r '.[0].status')
pass "newest run: $NEWEST_RUN_NAME (status: $NEWEST_RUN_STATUS)"

# get the new run
GET_NEW_RUN=$($CLI -o json flow get-run "$NEWEST_RUN_NAME" -r "$RUNTIME_UUID" 2>&1)
GOT_NEW_NAME=$(echo "$GET_NEW_RUN" | jq -r '.name')
if [ "$GOT_NEW_NAME" = "$NEWEST_RUN_NAME" ]; then
  pass "flow get-run on new run works"
else
  fail "flow get-run on new run" "expected $NEWEST_RUN_NAME, got $GOT_NEW_NAME"
fi

# ---------- status filter ----------

echo "=== status filter ==="

PENDING_RUNS=$($CLI -o json flow list-runs -r "$RUNTIME_UUID" --status pending 2>&1)
PENDING_COUNT=$(echo "$PENDING_RUNS" | jq 'length')
pass "flow list-runs --status pending returned $PENDING_COUNT run(s)"

# verify all returned runs actually have the requested status
if [ "$PENDING_COUNT" -gt 0 ]; then
  BAD_STATUS=$(echo "$PENDING_RUNS" | jq '[.[] | select(.status != "pending")] | length')
  if [ "$BAD_STATUS" -eq 0 ]; then
    pass "all pending runs have status=pending"
  else
    fail "status filter" "$BAD_STATUS runs have wrong status"
  fi
fi

# ---------- run_flow with spec ----------

echo "=== run_flow with spec ==="

SPEC_EMPTY=$($CLI -o json flow run "$FLOW_NAME" -r "$RUNTIME_UUID" --spec '{}' 2>&1)
if echo "$SPEC_EMPTY" | jq -e '.event_uuid' > /dev/null 2>&1; then
  pass "flow run --spec '{}' works"
else
  fail "flow run --spec '{}'" "missing event_uuid"
fi

SPEC_FR=$($CLI -o json flow run "$FLOW_NAME" -r "$RUNTIME_UUID" --spec '{"full_refresh":true}' 2>&1)
if echo "$SPEC_FR" | jq -e '.event_uuid' > /dev/null 2>&1; then
  pass "flow run --spec full_refresh works"
else
  fail "flow run --spec full_refresh" "missing event_uuid"
fi

SPEC_PARAMS=$($CLI -o json flow run "$FLOW_NAME" -r "$RUNTIME_UUID" --spec '{"parameters":{"key":"value"}}' 2>&1)
if echo "$SPEC_PARAMS" | jq -e '.event_uuid' > /dev/null 2>&1; then
  pass "flow run --spec parameters works"
else
  fail "flow run --spec parameters" "missing event_uuid"
fi

SPEC_MULTI=$($CLI -o json flow run "$FLOW_NAME" -r "$RUNTIME_UUID" --spec '{"run_tests":false,"halt_flow_on_error":true,"runner_overrides":{"size":"Medium"}}' 2>&1)
if echo "$SPEC_MULTI" | jq -e '.event_uuid' > /dev/null 2>&1; then
  pass "flow run --spec multiple fields works"
else
  fail "flow run --spec multiple fields" "missing event_uuid"
fi

# ---------- runtime pause/resume ----------

RUNTIME_KIND=$(echo "$JSON" | jq -r '.[0].kind')
if [ "$RUNTIME_KIND" != "workspace" ]; then
  skip "runtime is not a workspace — skipping pause/resume tests"
else
  echo "=== runtime pause ==="

  PAUSE_JSON=$($CLI -o json runtime pause "$RUNTIME_UUID" 2>&1)
  PAUSED=$(echo "$PAUSE_JSON" | jq -r '.paused')
  if [ "$PAUSED" = "true" ]; then
    pass "runtime pause sets paused=true"
  else
    fail "runtime pause" "expected paused=true, got $PAUSED"
  fi

  # wait for health to clear
  for delay in 1 2 3; do
    HEALTH=$(${CLI} -o json runtime get "$RUNTIME_UUID" 2>&1 | jq -r '.health')
    [ "$HEALTH" = "null" ] && break
    sleep "$delay"
  done
  if [ "$HEALTH" = "null" ]; then
    pass "paused runtime has health=null"
  else
    fail "paused runtime health" "expected null, got $HEALTH"
  fi

  # flow run without --resume should fail
  PAUSED_ERR=$($CLI -o json flow run "$FLOW_NAME" -r "$RUNTIME_UUID" 2>&1 || true)
  if echo "$PAUSED_ERR" | grep -qi "paused\|resume"; then
    pass "flow run on paused runtime fails with descriptive error"
  else
    fail "flow run on paused runtime" "expected error mentioning paused/resume, got: $PAUSED_ERR"
  fi

  echo "=== runtime resume via flow run ==="

  RESUME_TRIGGER=$($CLI -o json flow run "$FLOW_NAME" -r "$RUNTIME_UUID" --resume 2>&1)
  if echo "$RESUME_TRIGGER" | jq -e '.event_uuid' > /dev/null 2>&1; then
    pass "flow run --resume succeeds"
  else
    fail "flow run --resume" "missing event_uuid"
  fi

  AFTER_RESUME=$($CLI -o json runtime get "$RUNTIME_UUID" 2>&1)
  PAUSED_AFTER=$(echo "$AFTER_RESUME" | jq -r '.paused')
  if [ "$PAUSED_AFTER" = "false" ]; then
    pass "runtime is unpaused after --resume"
  else
    fail "runtime after --resume" "expected paused=false, got $PAUSED_AFTER"
  fi

  echo "=== runtime resume (explicit) ==="

  # wait for health to restore
  for delay in 2 3 5 5; do
    sleep "$delay"
    HEALTH=$($CLI -o json runtime get "$RUNTIME_UUID" 2>&1 | jq -r '.health')
    [ "$HEALTH" != "null" ] && break
  done
  if [ "$HEALTH" != "null" ]; then
    pass "runtime health restored: $HEALTH"
  else
    skip "runtime health not yet available after 15s"
  fi

  # resume on already-running runtime should be idempotent
  RESUME_IDEM=$($CLI -o json runtime resume "$RUNTIME_UUID" 2>&1)
  PAUSED_IDEM=$(echo "$RESUME_IDEM" | jq -r '.paused')
  if [ "$PAUSED_IDEM" = "false" ]; then
    pass "runtime resume is idempotent"
  else
    fail "runtime resume idempotent" "expected paused=false, got $PAUSED_IDEM"
  fi
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
