#!/usr/bin/env bash
# Prove the maintained public ascend-tools Otto lifecycle surface:
# - progressive open / preview
# - older-history page via --before
# - reopen / catch-up via --after
# - terminal completion via otto run --jsonl
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

CLI="uv run --project ascend-tools ascend-tools"
PASS=0
FAIL=0
SKIP=0

pass() { echo "  PASS: $1"; PASS=$((PASS + 1)); }
fail() { echo "  FAIL: $1 — $2"; FAIL=$((FAIL + 1)); }
skip() { echo "  SKIP: $1"; SKIP=$((SKIP + 1)); }

require_cmd() {
  if command -v "$1" >/dev/null 2>&1; then
    pass "$1 available"
  else
    fail "$1 availability" "command not found"
    print_summary
  fi
}

print_summary() {
  local total=$((PASS + FAIL + SKIP))
  echo
  echo "=== results ==="
  echo "$PASS passed, $FAIL failed, $SKIP skipped (of $total)"
  if [ "$FAIL" -gt 0 ]; then
    exit 1
  fi
  echo "all tests passed"
}

jsonl_terminal_completed() {
  local out="$1"
  echo "$out" | jq -e 'select(.record_type == "terminal" and .stream_status == "completed")' >/dev/null
}

jsonl_request_present() {
  local out="$1"
  echo "$out" | jq -e 'select(.record_type == "request") | .request_body.prompt' >/dev/null
}

jsonl_thread_done_present() {
  local out="$1"
  echo "$out" | jq -e 'select(.record_type == "event" and .event_type == "thread.done")' >/dev/null
}

extract_thread_id() {
  local out="$1"
  echo "$out" | jq -r 'select(.record_type == "event") | .thread_id' | head -n1
}

extract_thread_done_latest() {
  local out="$1"
  echo "$out" | jq -r 'select(.record_type == "event" and .event_type == "thread.done") | .data.latest_message_id // empty' | tail -n1
}

run_otto_jsonl() {
  local prompt="$1"
  shift
  $CLI otto run "$prompt" --jsonl "$@"
}

run_conversation_list() {
  local limit="${1:-5}"
  $CLI otto conversation list --limit "$limit"
}

run_conversation_list_json() {
  local limit="${1:-1}"
  $CLI -o json otto conversation list --limit "$limit"
}

progressive_open() {
  local thread_id="$1"
  shift
  $CLI -o json otto conversation open "$thread_id" --id "$@"
}

history_page() {
  local thread_id="$1"
  local before_id="$2"
  local limit="${3:-5}"
  $CLI -o json otto conversation history "$thread_id" --id --before "$before_id" --limit "$limit"
}

echo "=== preflight ==="
for var in ASCEND_SERVICE_ACCOUNT_ID ASCEND_SERVICE_ACCOUNT_KEY ASCEND_INSTANCE_API_URL; do
  if [ -z "${!var:-}" ]; then
    echo "ERROR: $var is not set" >&2
    exit 1
  fi
done
pass "env vars set"
require_cmd jq

echo "=== conversation list ==="
set +e
LIST_OUT="$(run_conversation_list 5 2>&1)"
LIST_RC=$?
set -e
if [ "$LIST_RC" -eq 0 ]; then
  pass "conversation list command succeeds with repo-local invocation"
else
  fail "conversation list" "$LIST_OUT"
  print_summary
fi

LOCAL_DEV_API_SUFFIX="-instance.api.local.ascend.dev"
LOCAL_DEV_APP_SUFFIX="-instance.app.local.ascend.dev"
if [[ "$ASCEND_INSTANCE_API_URL" == *"$LOCAL_DEV_API_SUFFIX"* ]]; then
  APP_HOST_URL="${ASCEND_INSTANCE_API_URL/$LOCAL_DEV_API_SUFFIX/$LOCAL_DEV_APP_SUFFIX}"
  set +e
  APP_HOST_OUT="$(ASCEND_INSTANCE_API_URL="$APP_HOST_URL" run_conversation_list_json 1 2>&1)"
  APP_HOST_RC=$?
  set -e
  if [ "$APP_HOST_RC" -eq 0 ] && echo "$APP_HOST_OUT" | jq -e '.threads' >/dev/null 2>&1; then
    pass "local-dev app host is normalized to the matching API host"
  else
    fail "local-dev host normalization proof" "$APP_HOST_OUT"
  fi
else
  skip "local-dev host normalization proof not applicable for current ASCEND_INSTANCE_API_URL"
fi

echo "=== otto lifecycle ==="
THREAD_ID="${ASCEND_OTTO_LIFECYCLE_THREAD_ID:-}"
FOLLOWUP_PROVIDER="${ASCEND_OTTO_LIFECYCLE_PROVIDER:-OpenAI}"
FOLLOWUP_MODEL="${ASCEND_OTTO_LIFECYCLE_MODEL:-gpt-5.2}"
FOLLOWUP_PROMPT="${ASCEND_OTTO_LIFECYCLE_PROMPT:-Reply with exactly 'plan-proof'.}"
if [ -n "$THREAD_ID" ]; then
  echo "  reusing thread: $THREAD_ID"
else
  SEED_OUT="$(run_otto_jsonl "Reply with exactly 'iter12-seed'.")"
  if jsonl_request_present "$SEED_OUT"; then
    pass "seed run emitted request provenance"
  else
    fail "seed run request provenance" "missing request record"
  fi
  if jsonl_thread_done_present "$SEED_OUT" && jsonl_terminal_completed "$SEED_OUT"; then
    pass "seed run completed through thread.done"
  else
    fail "seed run completion" "missing thread.done or completed terminal record"
  fi

  THREAD_ID="$(extract_thread_id "$SEED_OUT")"
  if [ -n "$THREAD_ID" ] && [ "$THREAD_ID" != "null" ]; then
    pass "seed run returned thread id $THREAD_ID"
  else
    fail "seed run thread id" "missing thread id in event records"
    print_summary
  fi
fi

PREVIEW_JSON="$(progressive_open "$THREAD_ID")"
PREVIEW_KIND="$(echo "$PREVIEW_JSON" | jq -r '.kind')"
if [ "$PREVIEW_KIND" = "preview" ]; then
  pass "progressive open returns preview"
else
  fail "progressive open kind" "expected preview, got $PREVIEW_KIND"
fi

HAS_MORE="$(echo "$PREVIEW_JSON" | jq -r '.data.has_more')"
TOTAL_COUNT="$(echo "$PREVIEW_JSON" | jq -r '.data.total_message_count')"
MAX_TURNS="${ASCEND_OTTO_LIFECYCLE_MAX_TURNS:-35}"
TURN=0

while [ "$HAS_MORE" != "true" ] && [ "$TURN" -lt "$MAX_TURNS" ]; do
  TURN=$((TURN + 1))
  TURN_OUT="$(run_otto_jsonl "Reply with exactly 'iter12-turn-$TURN'." --conversation "$THREAD_ID")"
  if jsonl_thread_done_present "$TURN_OUT" && jsonl_terminal_completed "$TURN_OUT"; then
    :
  else
    fail "turn $TURN completion" "missing thread.done or completed terminal record"
    break
  fi
  PREVIEW_JSON="$(progressive_open "$THREAD_ID")"
  HAS_MORE="$(echo "$PREVIEW_JSON" | jq -r '.data.has_more')"
  TOTAL_COUNT="$(echo "$PREVIEW_JSON" | jq -r '.data.total_message_count')"
done

if [ "$HAS_MORE" = "true" ]; then
  pass "long-thread preview reached has_more=true at total_message_count=$TOTAL_COUNT"
else
  fail "long-thread preview" "did not reach has_more=true after $TURN turns (total=$TOTAL_COUNT)"
  print_summary
fi

OLDEST_ID="$(echo "$PREVIEW_JSON" | jq -r '.data.oldest_message_id // empty')"
LATEST_BEFORE_FOLLOWUP="$(echo "$PREVIEW_JSON" | jq -r '.data.latest_message_id // empty')"
if [ -n "$OLDEST_ID" ] && [ -n "$LATEST_BEFORE_FOLLOWUP" ]; then
  pass "preview exposed oldest/latest message anchors"
else
  fail "preview anchors" "missing oldest_message_id or latest_message_id"
  print_summary
fi

HISTORY_JSON="$(history_page "$THREAD_ID" "$OLDEST_ID" 5)"
HISTORY_COUNT="$(echo "$HISTORY_JSON" | jq '.messages | length')"
if [ "$HISTORY_COUNT" -gt 0 ]; then
  pass "history page returned $HISTORY_COUNT older messages"
else
  fail "history page" "no older messages returned before $OLDEST_ID"
fi

if echo "$HISTORY_JSON" | jq -e '.oldest_message_id' >/dev/null; then
  pass "history page returned oldest_message_id"
else
  fail "history page anchor" "missing oldest_message_id"
fi

FOLLOW_OUT="$(run_otto_jsonl "$FOLLOWUP_PROMPT" --provider "$FOLLOWUP_PROVIDER" --model "$FOLLOWUP_MODEL" --conversation "$THREAD_ID")"
if jsonl_request_present "$FOLLOW_OUT"; then
  pass "follow-up run emitted request provenance"
else
  fail "follow-up request provenance" "missing request record"
fi
if echo "$FOLLOW_OUT" | jq -e --arg provider "$FOLLOWUP_PROVIDER" --arg model "$FOLLOWUP_MODEL" 'select(.record_type == "request") | .provider == $provider and .model == $model' >/dev/null; then
  pass "follow-up run preserved explicit provider/model provenance"
else
  fail "follow-up provider/model provenance" "missing explicit provider/model in request record"
fi
if jsonl_thread_done_present "$FOLLOW_OUT" && jsonl_terminal_completed "$FOLLOW_OUT"; then
  pass "follow-up run completed through thread.done"
else
  fail "follow-up completion" "missing thread.done or completed terminal record"
fi
LATEST_AFTER_FOLLOWUP="$(extract_thread_done_latest "$FOLLOW_OUT")"
if [ -n "$LATEST_AFTER_FOLLOWUP" ]; then
  pass "follow-up thread.done exposed latest_message_id"
else
  fail "follow-up latest_message_id" "thread.done missing latest_message_id"
fi

DELTA_JSON="$(progressive_open "$THREAD_ID" --after "$LATEST_BEFORE_FOLLOWUP")"
DELTA_KIND="$(echo "$DELTA_JSON" | jq -r '.kind')"
DELTA_COUNT="$(echo "$DELTA_JSON" | jq '.data.messages | length')"
NEW_LATEST="$(echo "$DELTA_JSON" | jq -r '.data.latest_message_id // empty')"

if [ "$DELTA_KIND" = "delta" ]; then
  pass "reopen with --after returned delta"
else
  fail "reopen kind" "expected delta, got $DELTA_KIND"
fi
if [ "$DELTA_COUNT" -gt 0 ]; then
  pass "reopen delta returned $DELTA_COUNT new messages"
else
  fail "reopen delta messages" "no messages returned after $LATEST_BEFORE_FOLLOWUP"
fi
if [ -n "$NEW_LATEST" ] && [ "$NEW_LATEST" = "$LATEST_AFTER_FOLLOWUP" ]; then
  pass "reopen delta latest_message_id matches follow-up thread.done"
else
  fail "reopen latest anchor" "expected $LATEST_AFTER_FOLLOWUP, got ${NEW_LATEST:-<empty>}"
fi

print_summary
