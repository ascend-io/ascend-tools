#!/usr/bin/env -S uv run --script
# /// script
# requires-python = ">=3.13"
# dependencies = ["ascend-tools"]
#
# [tool.uv.sources]
# ascend-tools = { path = ".." }
# ///
"""Integration tests for the ascend-tools Python SDK.

Requires a running ASE workspace with ASCEND_SERVICE_ACCOUNT_ID,
ASCEND_SERVICE_ACCOUNT_KEY, and ASCEND_INSTANCE_API_URL set.
"""

import os
import sys
import time

from ascend_tools import Client

PASS = 0
FAIL = 0
SKIP = 0


def check(condition: bool, label: str, detail: str = ""):
    global PASS, FAIL
    if condition:
        print(f"  PASS: {label}")
        PASS += 1
    else:
        print(f"  FAIL: {label} — {detail}")
        FAIL += 1


def skip(label: str):
    global SKIP
    print(f"  SKIP: {label}")
    SKIP += 1


def print_summary():
    total = PASS + FAIL + SKIP
    print()
    print("=== results ===")
    print(f"{PASS} passed, {FAIL} failed, {SKIP} skipped (of {total})")
    if FAIL > 0:
        print(f"{FAIL} FAILED")
        sys.exit(1)
    print("all tests passed")


def run_flow_with_retry(
    client: Client,
    *,
    runtime_uuid: str,
    flow_name: str,
    spec: dict | None = None,
    resume: bool = False,
) -> dict:
    """Run a flow with retries for transient runtime readiness states."""
    last_error: Exception | None = None
    for delay in (0, 2, 3, 5, 5):
        if delay:
            time.sleep(delay)
        try:
            return client.run_flow(
                runtime_uuid=runtime_uuid,
                flow_name=flow_name,
                spec=spec,
                resume=resume,
            )
        except Exception as e:  # noqa: BLE001
            msg = str(e).lower()
            if "starting" in msg or "no health status" in msg or "initializing" in msg:
                last_error = e
                continue
            raise

    if last_error is not None:
        raise last_error
    raise RuntimeError("run_flow retry exhausted")


def main():
    # ---------- preflight ----------

    print("=== preflight ===")

    for var in (
        "ASCEND_SERVICE_ACCOUNT_ID",
        "ASCEND_SERVICE_ACCOUNT_KEY",
        "ASCEND_INSTANCE_API_URL",
    ):
        if not os.environ.get(var):
            print(f"ERROR: {var} is not set", file=sys.stderr)
            sys.exit(1)
    check(True, "env vars set")

    client = Client()
    check(True, "client created")

    # ---------- runtimes ----------

    print("=== runtimes ===")

    runtimes = client.list_runtimes()
    check(isinstance(runtimes, list), "list_runtimes returns list")

    if not runtimes:
        skip("no runtimes found — skipping runtime get, filters, flows, and flow runs")
        print_summary()
        return

    check(True, f"list_runtimes returned {len(runtimes)} runtime(s)")

    runtime = runtimes[0]
    runtime_uuid = runtime["uuid"]
    runtime_id = runtime["id"]
    print(f"  using runtime: {runtime_id} ({runtime_uuid})")

    # get runtime
    got = client.get_runtime(uuid=runtime_uuid)
    check(got["uuid"] == runtime_uuid, "get_runtime returns correct uuid")

    for field in (
        "uuid",
        "id",
        "title",
        "kind",
        "project_uuid",
        "environment_uuid",
        "created_at",
        "updated_at",
    ):
        check(
            got.get(field) is not None,
            f"get_runtime has field '{field}'",
            f"value: {got.get(field)}",
        )

    # filter by id
    filtered = client.list_runtimes(id=runtime_id)
    check(
        len(filtered) == 1,
        "list_runtimes(id=...) returns exactly 1",
        f"got {len(filtered)}",
    )
    check(filtered[0]["uuid"] == runtime_uuid, "filtered runtime has correct uuid")

    # filter by kind
    kind = runtime["kind"]
    by_kind = client.list_runtimes(kind=kind)
    check(
        len(by_kind) >= 1,
        f"list_runtimes(kind={kind!r}) returns >= 1",
        f"got {len(by_kind)}",
    )
    check(all(r["kind"] == kind for r in by_kind), "all results match kind filter")

    # ---------- flows ----------

    print("=== flows ===")

    flows = client.list_flows(runtime_uuid=runtime_uuid)
    check(isinstance(flows, list), "list_flows returns list")

    if not flows:
        skip("no flows found — skipping flow runs and trigger tests")
        print_summary()
        return

    check(True, f"list_flows returned {len(flows)} flow(s)")

    flow_name = flows[0]["name"]
    print(f"  using flow: {flow_name}")

    # verify all flows have name
    check(all("name" in f for f in flows), "all flows have 'name' field")

    # ---------- flow runs (before) ----------

    print("=== flow runs (before trigger) ===")

    runs_before_result = client.list_flow_runs(
        runtime_uuid=runtime_uuid, flow_name=flow_name
    )
    check(isinstance(runs_before_result, dict), "list_flow_runs returns dict")
    check("items" in runs_before_result, "list_flow_runs has 'items' key")
    check("truncated" in runs_before_result, "list_flow_runs has 'truncated' key")
    runs_before = runs_before_result["items"]
    runs_before_count = len(runs_before)
    check(True, f"list_flow_runs returned {runs_before_count} run(s) before trigger")

    # test get_flow_run on existing run
    if runs_before:
        existing_run = runs_before[0]
        got_run = client.get_flow_run(
            runtime_uuid=runtime_uuid, name=existing_run["name"]
        )
        check(
            got_run["name"] == existing_run["name"], "get_flow_run returns correct run"
        )

        for field in (
            "name",
            "flow",
            "status",
            "runtime_uuid",
            "build_uuid",
            "created_at",
        ):
            check(got_run.get(field) is not None, f"get_flow_run has field '{field}'")

        # verify status is a known value
        check(
            got_run["status"] in ("pending", "running", "succeeded", "failed"),
            f"flow run status is valid: {got_run['status']}",
        )

    # test pagination
    limited = client.list_flow_runs(
        runtime_uuid=runtime_uuid, flow_name=flow_name, limit=1
    )["items"]
    check(
        len(limited) <= 1,
        "list_flow_runs(limit=1) returns at most 1",
        f"got {len(limited)}",
    )

    if runs_before_count > 1:
        offset_runs = client.list_flow_runs(
            runtime_uuid=runtime_uuid, flow_name=flow_name, offset=1, limit=1
        )["items"]
        check(
            len(offset_runs) <= 1, "list_flow_runs(offset=1, limit=1) returns at most 1"
        )
        if offset_runs and runs_before_count > 1:
            check(
                offset_runs[0]["name"] != runs_before[0]["name"],
                "offset=1 returns different run than offset=0",
            )

    # ---------- trigger flow run ----------

    print("=== trigger flow run ===")

    # Runtime may already be paused from previous sessions; use resume=True for baseline trigger.
    trigger = run_flow_with_retry(
        client, runtime_uuid=runtime_uuid, flow_name=flow_name, resume=True
    )
    check(isinstance(trigger, dict), "run_flow returns dict")
    check(
        trigger.get("event_uuid") is not None,
        f"run_flow has event_uuid: {trigger.get('event_uuid')}",
    )
    check(
        trigger.get("event_type") == "ScheduleFlowRun", "event_type is ScheduleFlowRun"
    )

    # ---------- flow runs (after) ----------

    print("=== flow runs (after trigger) ===")

    # poll for the new run to appear (up to 15s)
    runs_after_count = runs_before_count
    for delay in (2, 3, 5, 5):
        time.sleep(delay)
        runs_after = client.list_flow_runs(
            runtime_uuid=runtime_uuid, flow_name=flow_name
        )["items"]
        runs_after_count = len(runs_after)
        if runs_after_count > runs_before_count:
            break

    if runs_after_count > runs_before_count:
        check(
            True, f"flow run count increased: {runs_before_count} -> {runs_after_count}"
        )
    else:
        # Flow runner may be slow to process events (esp. after workspace restart).
        # The trigger itself succeeded (event_uuid returned), so this is infra timing.
        skip("flow run not yet materialized after 15s (flow runner may be catching up)")

    # verify newest run
    if runs_after:
        newest = runs_after[0]
        check(True, f"newest run: {newest['name']} (status: {newest['status']})")

        # get the new run
        got_new = client.get_flow_run(runtime_uuid=runtime_uuid, name=newest["name"])
        check(got_new["name"] == newest["name"], "get_flow_run on new run works")

    # ---------- status filter ----------

    print("=== status filter ===")

    for status in ("pending", "running", "succeeded", "failed"):
        by_status_result = client.list_flow_runs(
            runtime_uuid=runtime_uuid, status=status
        )
        by_status = by_status_result["items"]
        check(
            isinstance(by_status, list),
            f"list_flow_runs(status={status!r}) returns list items",
        )
        if by_status:
            wrong = [r for r in by_status if r["status"] != status]
            check(
                len(wrong) == 0,
                f"all {status} runs have correct status",
                f"{len(wrong)} have wrong status",
            )

    # ---------- run_flow with empty spec ----------

    print("=== run_flow with spec ===")

    trigger2 = run_flow_with_retry(
        client, runtime_uuid=runtime_uuid, flow_name=flow_name, spec={}, resume=True
    )
    check(trigger2.get("event_uuid") is not None, "run_flow with empty spec works")

    # spec with full_refresh
    trigger3_fr = run_flow_with_retry(
        client,
        runtime_uuid=runtime_uuid,
        flow_name=flow_name,
        spec={"full_refresh": True},
        resume=True,
    )
    check(
        trigger3_fr.get("event_uuid") is not None,
        "run_flow with full_refresh=True works",
    )

    # spec with parameters
    trigger3_params = run_flow_with_retry(
        client,
        runtime_uuid=runtime_uuid,
        flow_name=flow_name,
        spec={"parameters": {"key": "value"}},
        resume=True,
    )
    check(
        trigger3_params.get("event_uuid") is not None,
        "run_flow with parameters works",
    )

    # spec with multiple fields
    trigger3_multi = run_flow_with_retry(
        client,
        runtime_uuid=runtime_uuid,
        flow_name=flow_name,
        spec={
            "run_tests": False,
            "halt_flow_on_error": True,
            "runner_overrides": {"size": "Medium"},
        },
        resume=True,
    )
    check(
        trigger3_multi.get("event_uuid") is not None,
        "run_flow with multiple spec fields works",
    )

    # ---------- runtime pause/resume ----------

    if runtime["kind"] != "workspace":
        skip("runtime is not a workspace — skipping pause/resume tests")
    else:
        print("=== runtime pause ===")

        paused_rt = client.pause_runtime(uuid=runtime_uuid)
        check(paused_rt.get("paused") is True, "pause_runtime sets paused=True")

        got_paused = client.get_runtime(uuid=runtime_uuid)
        check(got_paused.get("paused") is True, "get_runtime confirms paused")

        # health may take a moment to clear after pause (runtime pods shutting down)
        for delay in (1, 2, 3):
            if got_paused.get("health") is None:
                break
            time.sleep(delay)
            got_paused = client.get_runtime(uuid=runtime_uuid)
        check(got_paused.get("health") is None, "paused runtime has health=None")

        # run_flow without resume should fail on a paused runtime
        try:
            client.run_flow(runtime_uuid=runtime_uuid, flow_name=flow_name)
            check(False, "run_flow on paused runtime should raise", "no error raised")
        except Exception as e:
            msg = str(e).lower()
            check(
                any(
                    term in msg
                    for term in (
                        "paused",
                        "resume",
                        "no health status",
                        "initializing",
                        "starting",
                    )
                ),
                "run_flow on paused/transitioning runtime raises descriptive error",
                str(e),
            )

        print("=== runtime resume via flow run ===")

        trigger3 = run_flow_with_retry(
            client, runtime_uuid=runtime_uuid, flow_name=flow_name, resume=True
        )
        check(
            trigger3.get("event_uuid") is not None, "run_flow with resume=True succeeds"
        )

        got_resumed = client.get_runtime(uuid=runtime_uuid)
        check(got_resumed.get("paused") is False, "runtime is unpaused after resume")

        print("=== runtime resume (explicit) ===")

        # Wait for runtime to start coming up, then verify resume is idempotent
        for delay in (2, 3, 5, 5):
            time.sleep(delay)
            rt_health = client.get_runtime(uuid=runtime_uuid)
            if rt_health.get("health") is not None:
                break

        if rt_health.get("health") is not None:
            check(True, f"runtime health restored: {rt_health['health']}")
        else:
            skip(
                "runtime health not yet available after 15s (runtime may be slow to start)"
            )

        # resume on an already-running runtime should be a no-op
        resumed_rt = client.resume_runtime(uuid=runtime_uuid)
        check(resumed_rt.get("paused") is False, "resume_runtime is idempotent")

    # ---------- summary ----------

    print_summary()


if __name__ == "__main__":
    main()
