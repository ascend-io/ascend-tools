#!/usr/bin/env -S uv run --script
# /// script
# requires-python = ">=3.11"
# dependencies = ["ascend-ops"]
#
# [tool.uv.sources]
# ascend-ops = { path = ".." }
# ///
"""Integration tests for the ascend-ops Python SDK.

Requires a running ASE workspace with ASCEND_SERVICE_ACCOUNT_ID,
ASCEND_SERVICE_ACCOUNT_KEY, and ASCEND_INSTANCE_API_URL set.
"""

import os
import sys
import time

from ascend_ops import Client

PASS = 0
FAIL = 0


def check(condition: bool, label: str, detail: str = ""):
    global PASS, FAIL
    if condition:
        print(f"  PASS: {label}")
        PASS += 1
    else:
        print(f"  FAIL: {label} — {detail}")
        FAIL += 1


def main():
    global PASS, FAIL

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
    check(
        len(runtimes) > 0,
        "list_runtimes returns at least 1 runtime",
        f"got {len(runtimes)}",
    )

    if not runtimes:
        print("ERROR: cannot continue without at least one runtime", file=sys.stderr)
        sys.exit(1)

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
    check(len(flows) > 0, "list_flows returns at least 1 flow", f"got {len(flows)}")

    if not flows:
        print("ERROR: cannot continue without at least one flow", file=sys.stderr)
        sys.exit(1)

    flow_name = flows[0]["name"]
    print(f"  using flow: {flow_name}")

    # verify all flows have name
    check(all("name" in f for f in flows), "all flows have 'name' field")

    # ---------- flow runs (before) ----------

    print("=== flow runs (before trigger) ===")

    runs_before = client.list_flow_runs(runtime_uuid=runtime_uuid, flow_name=flow_name)
    check(isinstance(runs_before, list), "list_flow_runs returns list")
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
    )
    check(
        len(limited) <= 1,
        "list_flow_runs(limit=1) returns at most 1",
        f"got {len(limited)}",
    )

    if runs_before_count > 1:
        offset_runs = client.list_flow_runs(
            runtime_uuid=runtime_uuid, flow_name=flow_name, offset=1, limit=1
        )
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

    trigger = client.run_flow(runtime_uuid=runtime_uuid, flow_name=flow_name)
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

    time.sleep(2)

    runs_after = client.list_flow_runs(runtime_uuid=runtime_uuid, flow_name=flow_name)
    runs_after_count = len(runs_after)
    check(
        runs_after_count > runs_before_count,
        f"flow run count increased: {runs_before_count} -> {runs_after_count}",
        f"expected > {runs_before_count}, got {runs_after_count}",
    )

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
        by_status = client.list_flow_runs(runtime_uuid=runtime_uuid, status=status)
        check(
            isinstance(by_status, list),
            f"list_flow_runs(status={status!r}) returns list",
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

    trigger2 = client.run_flow(runtime_uuid=runtime_uuid, flow_name=flow_name, spec={})
    check(trigger2.get("event_uuid") is not None, "run_flow with empty spec works")

    # ---------- summary ----------

    print()
    print("=== results ===")
    total = PASS + FAIL
    print(f"{PASS}/{total} passed")
    if FAIL > 0:
        print(f"{FAIL} FAILED")
        sys.exit(1)
    print("all tests passed")


if __name__ == "__main__":
    main()
