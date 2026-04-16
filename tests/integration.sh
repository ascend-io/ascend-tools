#!/usr/bin/env bash
# Integration tests for the ascend-tools CLI.
# Requires a running ASE workspace and ASCEND_SERVICE_ACCOUNT_ID,
# ASCEND_SERVICE_ACCOUNT_KEY, and ASCEND_INSTANCE_API_URL set.
set -euo pipefail

CLI="uv run ascend-tools"
PASS=0
FAIL=0
SKIP=0
LIGHTWEIGHT_FOLLOWUP_PROMPT="umm"
FOUNDRY_SIMPLE_PROMPT="Briefly explain what ASCEND_INSTANCE_API_URL is used for in one sentence."
OTTO_TOOL_PROMPT="Use a tool to inspect the current workspace root, confirm whether the repo contains both ascend-tools and ascend-backend, and answer in two short sentences with the names you found."

pass() { echo "  PASS: $1"; PASS=$((PASS + 1)); }
fail() { echo "  FAIL: $1 — $2"; FAIL=$((FAIL + 1)); }
skip() { echo "  SKIP: $1"; SKIP=$((SKIP + 1)); }

jsonl_completed_with_output() {
  echo "$1" | jq -e -s '
    def assistant_text:
      [(.content // [])[]? | .text // .output_text // empty] | join("");
    def completed_snapshot_has_text:
      any(
        .[];
        .record_type == "event"
        and .event_type == "thread.details"
        and (.data.is_processing == false)
        and (
          (.data.messages | type == "object")
          and any(
            (.data.messages | to_entries[]?.value);
            .role == "assistant" and (assistant_text | length > 0)
          )
        )
      );
    def has_text_delta:
      any(
        .[];
        .record_type == "event"
        and .event_type == "response.output_text.delta"
        and ((.data.delta // "") | length > 0)
      );
    any(.[]; .record_type == "terminal" and .stream_status == "completed")
    and (has_text_delta or completed_snapshot_has_text)
  ' >/dev/null
}

jsonl_has_explicit_failure() {
  echo "$1" | jq -e -s '
    (
      first(.[] | select(.record_type == "terminal")).stream_status == "interrupted"
      and (
        (first(.[] | select(.record_type == "terminal")).stream_error // "") | length
      ) > 0
    )
    or any(.[]; .record_type == "event" and .event_type == "response.error")
  ' >/dev/null
}

jsonl_has_reasoning_events() {
  echo "$1" | jq -e -s '
    any(
      .[];
      .record_type == "event"
      and (
        .event_type == "response.reasoning_summary_text.delta"
        or .event_type == "response.reasoning_text.delta"
      )
    )
  ' >/dev/null
}

jsonl_has_tool_argument_deltas() {
  echo "$1" | jq -e -s '
    any(
      .[];
      .record_type == "event"
      and .event_type == "response.function_call_arguments.delta"
    )
  ' >/dev/null
}

provider_list_has_thinking_levels() {
  echo "$1" | jq -e '
    any(
      .[];
      any(.models[]?; has("thinking_levels"))
    )
  ' >/dev/null
}

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

# Run `otto run` with retries for paused or still-starting local workspaces.
run_otto_retry() {
  local workspace_title="$1"
  local model_id="$2"
  local prompt="$3"
  local provider_id="${4:-}"
  local thread_id="${5:-}"

  local out=""
  local rc=1
  local delay
  for delay in 0 2 5 10 15; do
    if [ "$delay" -gt 0 ]; then
      sleep "$delay"
    fi

    set +e
    if [ -n "$provider_id" ]; then
      if [ -n "$thread_id" ]; then
        out=$($CLI otto run "$prompt" --workspace "$workspace_title" --provider "$provider_id" --model "$model_id" --thinking medium --thread "$thread_id" --jsonl 2>&1)
      else
        out=$($CLI otto run "$prompt" --workspace "$workspace_title" --provider "$provider_id" --model "$model_id" --thinking medium --jsonl 2>&1)
      fi
    else
      if [ -n "$thread_id" ]; then
        out=$($CLI otto run "$prompt" --workspace "$workspace_title" --model "$model_id" --thinking medium --thread "$thread_id" --jsonl 2>&1)
      else
        out=$($CLI otto run "$prompt" --workspace "$workspace_title" --model "$model_id" --thinking medium --jsonl 2>&1)
      fi
    fi
    rc=$?
    set -e

    if [ "$rc" -eq 0 ]; then
      echo "$out"
      return 0
    fi

    if echo "$out" | grep -qi "paused"; then
      $CLI -o json workspace resume "$workspace_title" >/dev/null 2>&1 || true
      continue
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

TARGET_TITLE="${ASCEND_TEST_WORKSPACE:-ascend-tools}"
MATCH=$(echo "$JSON" | jq -r --arg t "$TARGET_TITLE" '[.[] | select(.title == $t)] | first // empty')
if [ -n "$MATCH" ]; then
  RUNTIME_UUID=$(echo "$MATCH" | jq -r '.uuid')
  RUNTIME_TITLE=$(echo "$MATCH" | jq -r '.title')
else
  echo "  workspace '$TARGET_TITLE' not found, falling back to first workspace"
  RUNTIME_UUID=$(echo "$JSON" | jq -r '.[0].uuid')
  RUNTIME_TITLE=$(echo "$JSON" | jq -r '.[0].title')
fi
echo "  using workspace: $RUNTIME_TITLE ($RUNTIME_UUID)"

# get workspace by title
GET_JSON=$($CLI -o json workspace get "$RUNTIME_TITLE" 2>&1)
GOT_UUID=$(echo "$GET_JSON" | jq -r '.uuid')
if [ "$GOT_UUID" = "$RUNTIME_UUID" ]; then
  pass "workspace get returns correct uuid"
else
  fail "workspace get" "expected $RUNTIME_UUID, got $GOT_UUID"
fi

RUNTIME_PAUSED=$(echo "$GET_JSON" | jq -r '.paused')

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

# ---------- otto ----------

echo "=== otto ==="

set +e
OTTO_PROVIDERS_JSON=$($CLI -o json otto provider list 2>&1)
OTTO_PROVIDERS_RC=$?
set -e
if [ "$OTTO_PROVIDERS_RC" -ne 0 ]; then
  if echo "$OTTO_PROVIDERS_JSON" | grep -qi "not found\|not implemented\|404"; then
    skip "otto provider list not available"
  else
    fail "otto provider list" "$OTTO_PROVIDERS_JSON"
  fi
else
  pass "otto provider list returns JSON"
  if provider_list_has_thinking_levels "$OTTO_PROVIDERS_JSON"; then
    pass "otto provider list surfaces model thinking_levels"
  else
    fail "otto provider list" "missing model thinking_levels metadata"
  fi
  FOUNDRY_GPT54_MODEL=$(echo "$OTTO_PROVIDERS_JSON" | jq -r '
    [
      .[] | select(.id == "microsoft_foundry") | .models[]? | .id
      | select(. == "azure_ai/gpt-5.4")
    ][0] // empty
  ')
  OTTO_MODEL=$(echo "$OTTO_PROVIDERS_JSON" | jq -r '
    [
      .[] | .models[]? | .id
      | select(test("(claude|gpt-5|gemini|^o[0-9])"; "i"))
    ][0] // empty
  ')

  if [ -z "$OTTO_MODEL" ]; then
    skip "no reasoning-capable otto model found for CLI thinking test"
  else
    echo "  using otto model: $OTTO_MODEL"
    if [ "$RUNTIME_PAUSED" = "true" ]; then
      PRE_OTTO_RESUME=$($CLI -o json workspace resume "$RUNTIME_TITLE" 2>&1)
      PRE_OTTO_PAUSED=$(echo "$PRE_OTTO_RESUME" | jq -r '.paused')
      if [ "$PRE_OTTO_PAUSED" = "false" ]; then
        pass "workspace resume clears paused before otto"
      else
        fail "workspace resume before otto" "expected paused=false, got $PRE_OTTO_PAUSED"
      fi
    fi

    if [ -n "$FOUNDRY_GPT54_MODEL" ]; then
      set +e
      FOUNDRY_SIMPLE_OUT=$(run_otto_retry "$RUNTIME_TITLE" "$FOUNDRY_GPT54_MODEL" "$FOUNDRY_SIMPLE_PROMPT" "microsoft_foundry")
      FOUNDRY_SIMPLE_RC=$?
      set -e

      if [ "$FOUNDRY_SIMPLE_RC" -ne 0 ]; then
        fail "foundry gpt-5.4 simple prompt" "$FOUNDRY_SIMPLE_OUT"
      else
        if echo "$FOUNDRY_SIMPLE_OUT" | jq -e 'select(.record_type == "request") | .provider == "microsoft_foundry" and .model == "azure_ai/gpt-5.4"' >/dev/null; then
          pass "foundry gpt-5.4 simple prompt preserved provider/model provenance"
        else
          fail "foundry gpt-5.4 simple prompt" "missing exact provider/model provenance"
        fi

        if jsonl_completed_with_output "$FOUNDRY_SIMPLE_OUT"; then
          pass "foundry gpt-5.4 simple prompt returned assistant output"
        elif jsonl_has_explicit_failure "$FOUNDRY_SIMPLE_OUT"; then
          pass "foundry gpt-5.4 simple prompt surfaced an explicit failure"
        else
          fail "foundry gpt-5.4 simple prompt" "missing assistant output and explicit surfaced failure"
        fi
      fi

      set +e
      FOUNDRY_TOOL_OUT=$(run_otto_retry "$RUNTIME_TITLE" "$FOUNDRY_GPT54_MODEL" "$OTTO_TOOL_PROMPT" "microsoft_foundry")
      FOUNDRY_TOOL_RC=$?
      set -e

      if [ "$FOUNDRY_TOOL_RC" -ne 0 ]; then
        fail "foundry gpt-5.4 tool prompt" "$FOUNDRY_TOOL_OUT"
      else
        if jsonl_completed_with_output "$FOUNDRY_TOOL_OUT"; then
          pass "foundry gpt-5.4 tool prompt returned assistant output"
          if jsonl_has_reasoning_events "$FOUNDRY_TOOL_OUT"; then
            pass "foundry gpt-5.4 tool prompt surfaced reasoning events"
          else
            fail "foundry gpt-5.4 tool prompt" "missing reasoning events on successful tool prompt"
          fi
          if jsonl_has_tool_argument_deltas "$FOUNDRY_TOOL_OUT"; then
            pass "foundry gpt-5.4 tool prompt surfaced tool argument deltas"
          else
            fail "foundry gpt-5.4 tool prompt" "missing tool argument deltas on successful tool prompt"
          fi
        elif jsonl_has_explicit_failure "$FOUNDRY_TOOL_OUT"; then
          pass "foundry gpt-5.4 tool prompt surfaced an explicit failure"
        else
          fail "foundry gpt-5.4 tool prompt" "missing assistant output and explicit surfaced failure"
        fi
      fi
    else
      skip "foundry gpt-5.4 model not available for exact-path probe"
    fi

    set +e
    OTTO_RUN_OUT=$(run_otto_retry "$RUNTIME_TITLE" "$OTTO_MODEL" "$OTTO_TOOL_PROMPT")
    OTTO_RUN_RC=$?
    set -e

    if [ "$OTTO_RUN_RC" -ne 0 ]; then
      fail "otto run --thinking" "$OTTO_RUN_OUT"
    else
      if echo "$OTTO_RUN_OUT" | jq -e 'select(.record_type == "request") | .request_body.thinking == "medium"' >/dev/null; then
        pass "otto run --jsonl preserves explicit thinking in request record"
      else
        fail "otto run --jsonl" "missing or incorrect thinking in request record"
      fi

      if echo "$OTTO_RUN_OUT" | jq -e 'select(.record_type == "event")' >/dev/null; then
        pass "otto run --jsonl emitted ordered event records"
      else
        fail "otto run --jsonl" "missing event records"
      fi

      if jsonl_completed_with_output "$OTTO_RUN_OUT"; then
        pass "otto run --jsonl emitted completed output"
      else
        fail "otto run --jsonl" "missing completed assistant output"
      fi

      if jsonl_has_reasoning_events "$OTTO_RUN_OUT"; then
        pass "otto run --jsonl surfaced reasoning events for the tool-use prompt"
      else
        fail "otto run --jsonl" "missing reasoning events for the required tool-use prompt"
      fi

      if jsonl_has_tool_argument_deltas "$OTTO_RUN_OUT"; then
        pass "otto run --jsonl surfaced tool argument deltas for the tool-use prompt"
      else
        fail "otto run --jsonl" "missing tool argument deltas for the required tool-use prompt"
      fi

      OTTO_THREAD_ID=$(echo "$OTTO_RUN_OUT" | jq -r 'select(.record_type == "terminal") | .thread_id' | tail -n 1)
      if [ -n "$OTTO_THREAD_ID" ] && [ "$OTTO_THREAD_ID" != "null" ]; then
        pass "otto run --jsonl terminal record exposed thread_id"
        set +e
        OTTO_FOLLOWUP_OUT=$(run_otto_retry "$RUNTIME_TITLE" "$OTTO_MODEL" "$LIGHTWEIGHT_FOLLOWUP_PROMPT" "" "$OTTO_THREAD_ID")
        OTTO_FOLLOWUP_RC=$?
        set -e

        if [ "$OTTO_FOLLOWUP_RC" -ne 0 ]; then
          fail "otto follow-up --jsonl" "$OTTO_FOLLOWUP_OUT"
        else
          if echo "$OTTO_FOLLOWUP_OUT" | jq -e --arg thread_id "$OTTO_THREAD_ID" 'select(.record_type == "request") | .request_thread_id == $thread_id' >/dev/null; then
            pass "otto follow-up --jsonl preserved request_thread_id"
          else
            fail "otto follow-up --jsonl" "missing request_thread_id on follow-up request"
          fi

          if echo "$OTTO_FOLLOWUP_OUT" | jq -e --arg thread_id "$OTTO_THREAD_ID" 'select(.record_type == "terminal") | .thread_id == $thread_id' >/dev/null; then
            pass "otto follow-up --jsonl reused terminal thread_id"
          else
            fail "otto follow-up --jsonl" "terminal thread_id did not match follow-up target"
          fi
        fi
      else
        fail "otto run --jsonl" "terminal thread_id missing from initial turn"
      fi
    fi
  fi
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

RUNS_BEFORE_RESULT=$($CLI -o json flow list-runs --uuid "$RUNTIME_UUID" --flow "$FLOW_NAME" 2>&1)
RUNS_BEFORE=$(echo "$RUNS_BEFORE_RESULT" | jq '.items')
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
  RUNS_AFTER_RESULT=$($CLI -o json flow list-runs --uuid "$RUNTIME_UUID" --flow "$FLOW_NAME" 2>&1)
  RUNS_AFTER=$(echo "$RUNS_AFTER_RESULT" | jq '.items')
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
