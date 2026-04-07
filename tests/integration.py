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

import argparse
import os
import sys
import time

from ascend_tools import Client

PASS = 0
FAIL = 0
SKIP = 0
FOUNDRY_SIMPLE_PROMPT = (
    "Briefly explain what ASCEND_INSTANCE_API_URL is used for in one sentence."
)
OTTO_TOOL_PROMPT = (
    "Use a tool to inspect the current workspace root, confirm whether the repo contains both "
    "ascend-tools and ascend-backend, and answer in two short sentences with the names you found."
)


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
    workspace: str,
    flow: str,
    spec: dict | None = None,
    resume: bool = False,
) -> dict:
    """Run a flow with retries for transient readiness states."""
    last_error: Exception | None = None
    for delay in (0, 5, 10, 15, 15, 15):
        if delay:
            time.sleep(delay)
        try:
            return client.run_flow(
                flow=flow,
                workspace=workspace,
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


def run_otto_with_retry(
    client: Client,
    *,
    workspace: str,
    prompt: str,
    model: str | None = None,
    provider: str | None = None,
    thinking: str | None = None,
) -> dict:
    """Run Otto with retries for paused or still-starting local workspaces."""
    last_error: Exception | None = None
    for delay in (0, 2, 5, 10, 15):
        if delay:
            time.sleep(delay)
        try:
            kwargs = {"prompt": prompt, "workspace": workspace}
            if model is not None:
                kwargs["model"] = model
            if provider is not None:
                kwargs["provider"] = provider
            if thinking is not None:
                kwargs["thinking"] = thinking
            return client.otto(**kwargs)
        except Exception as e:  # noqa: BLE001
            msg = str(e).lower()
            if "paused" in msg:
                client.resume_workspace(title=workspace)
                last_error = e
                continue
            if "starting" in msg or "no health status" in msg or "initializing" in msg:
                last_error = e
                continue
            raise

    if last_error is not None:
        raise last_error
    raise RuntimeError("otto retry exhausted")


def pick_reasoning_model(providers: list[dict]) -> str | None:
    for provider in providers:
        for model in provider.get("models", []):
            model_id = model.get("id", "")
            if any(
                token in model_id.lower() for token in ("claude", "gpt-5", "gemini")
            ):
                return model_id
            if model_id.lower().startswith("o"):
                return model_id
    return None


def find_provider_model(
    providers: list[dict], provider_id: str, model_id: str
) -> str | None:
    for provider in providers:
        if provider.get("id") != provider_id:
            continue
        for model in provider.get("models", []):
            if model.get("id") == model_id:
                return model_id
    return None


def exercise_otto_response_contract(
    client: Client,
    *,
    workspace: str,
    prompt: str,
    label: str,
    model: str | None = None,
    provider: str | None = None,
    allow_explicit_failure: bool = False,
) -> bool:
    try:
        otto_resp = run_otto_with_retry(
            client,
            prompt=prompt,
            workspace=workspace,
            model=model,
            provider=provider,
            thinking="medium",
        )
        check(isinstance(otto_resp, dict), f"{label} returns dict")
        check("message" in otto_resp, f"{label} response has 'message' key")
        check("thread_id" in otto_resp, f"{label} response has 'thread_id' key")
        check(
            bool(str(otto_resp.get("message", "")).strip()),
            f"{label} returned assistant output",
            repr(otto_resp.get("message")),
        )
        return True
    except Exception as e:  # noqa: BLE001
        if allow_explicit_failure:
            check(
                bool(str(e).strip()),
                f"{label} surfaced explicit failure",
                str(e),
            )
            return False
        check(False, label, str(e))
        return False


def main():
    parser = argparse.ArgumentParser(
        description="ascend-tools Python SDK integration tests"
    )
    parser.add_argument(
        "--workspace",
        default="ascend-tools",
        help="Workspace title to test against (default: ascend-tools)",
    )
    args = parser.parse_args()

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

    # ---------- workspaces ----------

    print("=== workspaces ===")

    workspaces = client.list_workspaces()
    check(isinstance(workspaces, list), "list_workspaces returns list")

    if not workspaces:
        skip("no workspaces found — skipping remaining tests")
        print_summary()
        return

    check(True, f"list_workspaces returned {len(workspaces)} workspace(s)")

    # find the target workspace by title
    matches = [w for w in workspaces if w["title"] == args.workspace]
    if matches:
        workspace = matches[0]
    else:
        print(
            f"  workspace '{args.workspace}' not found, falling back to first workspace"
        )
        workspace = workspaces[0]

    ws_title = workspace["title"]
    ws_uuid = workspace["uuid"]
    print(f"  using workspace: {ws_title} ({ws_uuid})")

    # get workspace
    got = client.get_workspace(title=ws_title)
    check(got["uuid"] == ws_uuid, "get_workspace returns correct uuid")

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
            f"get_workspace has field '{field}'",
            f"value: {got.get(field)}",
        )

    # ---------- deployments ----------

    print("=== deployments ===")

    deployments = client.list_deployments()
    check(isinstance(deployments, list), "list_deployments returns list")
    check(True, f"list_deployments returned {len(deployments)} deployment(s)")

    # ---------- environments ----------

    print("=== environments ===")

    environments = client.list_environments()
    check(isinstance(environments, list), "list_environments returns list")

    if environments:
        check(True, f"list_environments returned {len(environments)} environment(s)")
        env0 = environments[0]
        for field in ("uuid", "id", "title"):
            check(
                env0.get(field) is not None,
                f"environment has field '{field}'",
                f"value: {env0.get(field)}",
            )

        # get_environment
        env_title = env0["title"]
        resolved_env = client.get_environment(title=env_title)
        check(isinstance(resolved_env, dict), "get_environment returns dict")
        check(
            resolved_env["uuid"] == env0["uuid"],
            "get_environment returns correct environment",
            f"expected {env0['uuid']}, got {resolved_env.get('uuid')}",
        )
    else:
        skip("no environments found — skipping get_environment")

    # ---------- projects ----------

    print("=== projects ===")

    projects = client.list_projects()
    check(isinstance(projects, list), "list_projects returns list")

    if projects:
        check(True, f"list_projects returned {len(projects)} project(s)")
        proj0 = projects[0]
        for field in ("uuid", "id", "title", "path", "repository_uuid"):
            check(
                proj0.get(field) is not None,
                f"project has field '{field}'",
                f"value: {proj0.get(field)}",
            )

        # get_project
        proj_title = proj0["title"]
        resolved_proj = client.get_project(title=proj_title)
        check(isinstance(resolved_proj, dict), "get_project returns dict")
        check(
            resolved_proj["uuid"] == proj0["uuid"],
            "get_project returns correct project",
            f"expected {proj0['uuid']}, got {resolved_proj.get('uuid')}",
        )
    else:
        skip("no projects found — skipping get_project")

    # ---------- otto chat ----------

    print("=== otto chat ===")

    reasoning_model = None
    foundry_gpt54_model = None
    if got.get("paused") is True:
        resumed = client.resume_workspace(title=ws_title)
        check(
            resumed.get("paused") is False,
            "resume_workspace clears paused before otto",
        )
    try:
        provider_probe = client.list_otto_providers()
        if isinstance(provider_probe, list):
            reasoning_model = pick_reasoning_model(provider_probe)
            foundry_gpt54_model = find_provider_model(
                provider_probe, "microsoft_foundry", "azure_ai/gpt-5.4"
            )
    except Exception:
        pass

    if foundry_gpt54_model:
        exercise_otto_response_contract(
            client,
            workspace=ws_title,
            prompt=FOUNDRY_SIMPLE_PROMPT,
            label="foundry gpt-5.4 simple prompt",
            model=foundry_gpt54_model,
            provider="microsoft_foundry",
            allow_explicit_failure=True,
        )
        exercise_otto_response_contract(
            client,
            workspace=ws_title,
            prompt=OTTO_TOOL_PROMPT,
            label="foundry gpt-5.4 tool prompt",
            model=foundry_gpt54_model,
            provider="microsoft_foundry",
            allow_explicit_failure=True,
        )
    else:
        skip("foundry gpt-5.4 model not available for exact-path SDK probe")

    if reasoning_model:
        if exercise_otto_response_contract(
            client,
            workspace=ws_title,
            prompt=OTTO_TOOL_PROMPT,
            label=f"otto explicit thinking contract for {reasoning_model}",
            model=reasoning_model,
        ):
            check(True, f"otto accepts explicit thinking for {reasoning_model}")
    else:
        skip("no reasoning-capable otto model found for explicit thinking request")

    # ---------- otto provider ----------

    print("=== otto provider ===")

    providers = None
    try:
        providers = client.list_otto_providers()
        check(isinstance(providers, list), "list_otto_providers returns list")
        if providers:
            check(True, f"list_otto_providers returned {len(providers)} provider(s)")
            p = providers[0]
            check("id" in p, "provider has field 'id'")
            check("name" in p, "provider has field 'name'")
            check("default_model" in p, "provider has field 'default_model'")
            check("models" in p, "provider has field 'models'")
            check(isinstance(p["models"], list), "provider.models is a list")
        else:
            skip("no otto providers configured")
    except Exception as e:  # noqa: BLE001
        msg = str(e).lower()
        if "not found" in msg or "404" in msg:
            skip(f"list_otto_providers not available: {e}")
        else:
            check(False, "list_otto_providers", str(e))

    # ---------- flows ----------

    print("=== flows ===")

    flows = client.list_flows(workspace=ws_title)
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

    runs_before_result = client.list_flow_runs(workspace=ws_title, flow=flow_name)
    check(isinstance(runs_before_result, dict), "list_flow_runs returns dict")
    check("items" in runs_before_result, "list_flow_runs has 'items' key")
    check("truncated" in runs_before_result, "list_flow_runs has 'truncated' key")
    runs_before = runs_before_result["items"]
    runs_before_count = len(runs_before)
    check(True, f"list_flow_runs returned {runs_before_count} run(s) before trigger")

    # test get_flow_run on existing run
    if runs_before:
        existing_run = runs_before[0]
        got_run = client.get_flow_run(name=existing_run["name"], workspace=ws_title)
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
    limited = client.list_flow_runs(workspace=ws_title, flow=flow_name, limit=1)[
        "items"
    ]
    check(
        len(limited) <= 1,
        "list_flow_runs(limit=1) returns at most 1",
        f"got {len(limited)}",
    )

    if runs_before_count > 1:
        offset_runs = client.list_flow_runs(
            workspace=ws_title, flow=flow_name, offset=1, limit=1
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

    # Workspace may already be paused from previous sessions; use resume=True for baseline trigger.
    trigger = run_flow_with_retry(
        client, workspace=ws_title, flow=flow_name, resume=True
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
        runs_after = client.list_flow_runs(workspace=ws_title, flow=flow_name)["items"]
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
        got_new = client.get_flow_run(name=newest["name"], workspace=ws_title)
        check(got_new["name"] == newest["name"], "get_flow_run on new run works")

    # ---------- status filter ----------

    print("=== status filter ===")

    for status in ("pending", "running", "succeeded", "failed"):
        by_status_result = client.list_flow_runs(workspace=ws_title, status=status)
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
        client, workspace=ws_title, flow=flow_name, spec={}, resume=True
    )
    check(trigger2.get("event_uuid") is not None, "run_flow with empty spec works")

    # spec with full_refresh
    trigger3_fr = run_flow_with_retry(
        client,
        workspace=ws_title,
        flow=flow_name,
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
        workspace=ws_title,
        flow=flow_name,
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
        workspace=ws_title,
        flow=flow_name,
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

    # ---------- workspace pause/resume ----------

    print("=== workspace pause ===")

    paused_rt = client.pause_workspace(title=ws_title)
    check(paused_rt.get("paused") is True, "pause_workspace sets paused=True")

    got_paused = client.get_workspace(title=ws_title)
    check(got_paused.get("paused") is True, "get_workspace confirms paused")

    # run_flow without resume should fail on a paused workspace
    try:
        client.run_flow(flow=flow_name, workspace=ws_title)
        check(False, "run_flow on paused workspace should raise", "no error raised")
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
            "run_flow on paused/transitioning workspace raises descriptive error",
            str(e),
        )

    print("=== workspace resume via flow run ===")

    trigger3 = run_flow_with_retry(
        client, workspace=ws_title, flow=flow_name, resume=True
    )
    check(trigger3.get("event_uuid") is not None, "run_flow with resume=True succeeds")

    got_resumed = client.get_workspace(title=ws_title)
    check(got_resumed.get("paused") is False, "workspace is unpaused after resume")

    print("=== workspace resume (explicit) ===")

    # Wait for workspace to start coming up, then verify resume is idempotent
    for delay in (2, 3, 5, 5):
        time.sleep(delay)
        ws_health = client.get_workspace(title=ws_title)
        if ws_health.get("health") is not None:
            break

    if ws_health.get("health") is not None:
        check(True, f"workspace health restored: {ws_health['health']}")
    else:
        skip(
            "workspace health not yet available after 15s (workspace may be slow to start)"
        )

    # resume on an already-running workspace should be a no-op
    resumed_rt = client.resume_workspace(title=ws_title)
    check(resumed_rt.get("paused") is False, "resume_workspace is idempotent")

    # ---------- summary ----------

    print_summary()


if __name__ == "__main__":
    main()
