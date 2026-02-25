#!/usr/bin/env bash
# Integration tests for the ascend-ops CLI.
# Requires a running ASE workspace and ASCEND_SERVICE_ACCOUNT_ID,
# ASCEND_SERVICE_ACCOUNT_KEY, and ASCEND_INSTANCE_API_URL set.
set -euo pipefail

CLI="uv run ascend-ops"
PASS=0
FAIL=0

pass() { echo "  PASS: $1"; PASS=$((PASS + 1)); }
fail() { echo "  FAIL: $1 — $2"; FAIL=$((FAIL + 1)); }

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
  fail "runtime list (json)" "no runtimes found"
  echo "ERROR: cannot continue without at least one runtime" >&2
  exit 1
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
  fail "flow list" "no flows found"
  echo "ERROR: cannot continue without at least one flow" >&2
  exit 1
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

# wait briefly for the run to appear
sleep 2

RUNS_AFTER=$($CLI -o json flow list-runs -r "$RUNTIME_UUID" -f "$FLOW_NAME" 2>&1)
RUNS_AFTER_COUNT=$(echo "$RUNS_AFTER" | jq 'length')

if [ "$RUNS_AFTER_COUNT" -gt "$RUNS_BEFORE_COUNT" ]; then
  pass "flow run count increased: $RUNS_BEFORE_COUNT -> $RUNS_AFTER_COUNT"
else
  fail "flow run count" "expected > $RUNS_BEFORE_COUNT, got $RUNS_AFTER_COUNT"
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

# ---------- summary ----------

echo ""
echo "=== results ==="
TOTAL=$((PASS + FAIL))
echo "$PASS/$TOTAL passed"
if [ "$FAIL" -gt 0 ]; then
  echo "$FAIL FAILED"
  exit 1
fi
echo "all tests passed"
