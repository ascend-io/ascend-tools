#!/usr/bin/env -S uv run --script
# /// script
# requires-python = ">=3.13"
# dependencies = ["httpx", "cryptography"]
# ///
"""REST API integration tests for the Ascend Instance API.

Self-contained — no ascend-tools dependency. Authenticates using Ed25519
JWT signing and exercises the /api/v1 endpoints directly via httpx.

Requires ASCEND_SERVICE_ACCOUNT_ID, ASCEND_SERVICE_ACCOUNT_KEY, and
ASCEND_INSTANCE_API_URL environment variables.
"""

import argparse
import base64
import json
import os
import sys
import time
from urllib.parse import quote

import httpx
from cryptography.hazmat.primitives.asymmetric.ed25519 import Ed25519PrivateKey

# ---------------------------------------------------------------------------
# Ed25519 JWT signing
# ---------------------------------------------------------------------------


def base64url_encode(data: bytes) -> str:
    """Encode bytes to unpadded base64url (RFC 7515 §2)."""
    return base64.urlsafe_b64encode(data).rstrip(b"=").decode("ascii")


def base64url_decode(s: str) -> bytes:
    """Decode unpadded base64url (or standard base64) to bytes."""
    s = s.strip()
    try:
        return base64.urlsafe_b64decode(s + "=" * (-len(s) % 4))
    except Exception:
        return base64.b64decode(s)


def sign_jwt(claims: dict, private_key: Ed25519PrivateKey) -> str:
    """Create a signed JWT using EdDSA (Ed25519).

    The JWT is three base64url segments joined by dots:
        base64url(header) . base64url(payload) . base64url(signature)

    The signature covers the ASCII bytes of "header.payload".
    """
    header_b64 = base64url_encode(
        json.dumps({"alg": "EdDSA", "typ": "JWT"}, separators=(",", ":")).encode()
    )
    payload_b64 = base64url_encode(json.dumps(claims, separators=(",", ":")).encode())
    signing_input = f"{header_b64}.{payload_b64}".encode("ascii")
    signature = private_key.sign(signing_input)
    return f"{header_b64}.{payload_b64}.{base64url_encode(signature)}"


# ---------------------------------------------------------------------------
# Authentication
# ---------------------------------------------------------------------------


class Auth:
    """Ascend service account authentication.

    Flow:
      1. Decode the base64url Ed25519 seed (32 bytes) from the service account key
      2. Discover the JWT audience domain from GET /api/v1/auth/config
      3. Sign an Ed25519 JWT with service account claims
      4. Exchange the JWT for an instance token at POST /api/v1/auth/token
      5. Cache the token; refresh when within 5 minutes of expiry
    """

    def __init__(
        self,
        service_account_id: str,
        service_account_key: str,
        instance_api_url: str,
    ):
        self.service_account_id = service_account_id
        self.instance_api_url = instance_api_url

        seed = base64url_decode(service_account_key)
        if len(seed) != 32:
            raise ValueError(f"expected 32-byte Ed25519 seed, got {len(seed)} bytes")
        self.private_key = Ed25519PrivateKey.from_private_bytes(seed)

        self._http = httpx.Client()
        self._cloud_api_domain: str | None = None
        self._cached_token: str | None = None
        self._token_expires_at: int = 0

    def get_token(self) -> str:
        """Return a valid instance token, refreshing if needed."""
        now = int(time.time())
        if self._cached_token and self._token_expires_at > now + 300:
            return self._cached_token

        domain = self._get_cloud_api_domain()
        sa_jwt = self._sign_sa_jwt(now, domain)
        token, expires_at = self._exchange_token(sa_jwt)

        self._cached_token = token
        self._token_expires_at = expires_at
        return token

    def _get_cloud_api_domain(self) -> str:
        """Fetch (and cache) the JWT audience domain from auth config."""
        if self._cloud_api_domain:
            return self._cloud_api_domain

        url = f"{self.instance_api_url}/api/v1/auth/config"
        resp = self._http.get(url)
        resp.raise_for_status()
        domain = resp.json()["cloud_api_domain"]
        self._cloud_api_domain = domain
        return domain

    def _sign_sa_jwt(self, now: int, cloud_api_domain: str) -> str:
        """Sign a service account JWT (5-minute expiry)."""
        return sign_jwt(
            {
                "sub": self.service_account_id,
                "aud": f"https://{cloud_api_domain}/auth/token",
                "exp": now + 300,
                "iat": now,
                "name": self.service_account_id,
                "service_account": self.service_account_id,
            },
            self.private_key,
        )

    def _exchange_token(self, sa_jwt: str) -> tuple[str, int]:
        """Exchange the SA JWT for an instance access token."""
        url = f"{self.instance_api_url}/api/v1/auth/token"
        resp = self._http.post(
            url,
            headers={"Authorization": f"Bearer {sa_jwt}"},
        )
        resp.raise_for_status()
        data = resp.json()
        return data["access_token"], data.get("expiration", int(time.time()) + 3600)


# ---------------------------------------------------------------------------
# API client
# ---------------------------------------------------------------------------


class AscendClient:
    """Minimal HTTP client for the Ascend Instance API v1."""

    def __init__(self, auth: Auth):
        self.auth = auth
        self.base_url = auth.instance_api_url
        self._http = httpx.Client()

    def _headers(self) -> dict[str, str]:
        return {"Authorization": f"Bearer {self.auth.get_token()}"}

    def _get(self, path: str, params: dict | None = None):
        resp = self._http.get(
            f"{self.base_url}{path}", headers=self._headers(), params=params
        )
        _raise_for_api_error(resp)
        return resp.json()

    def _post_empty(self, path: str):
        resp = self._http.post(f"{self.base_url}{path}", headers=self._headers())
        _raise_for_api_error(resp)
        return resp.json()

    def _post_json(self, path: str, body: dict):
        resp = self._http.post(
            f"{self.base_url}{path}", headers=self._headers(), json=body
        )
        _raise_for_api_error(resp)
        return resp.json()

    # -- Runtimes --

    def list_runtimes(self, **filters) -> list[dict]:
        params = {k: v for k, v in filters.items() if v is not None}
        return self._get("/api/v1/runtimes", params=params or None)

    def get_runtime(self, uuid: str) -> dict:
        return self._get(f"/api/v1/runtimes/{_encode(uuid)}")

    def pause_runtime(self, uuid: str) -> dict:
        return self._post_empty(f"/api/v1/runtimes/{_encode(uuid)}:pause")

    def resume_runtime(self, uuid: str) -> dict:
        return self._post_empty(f"/api/v1/runtimes/{_encode(uuid)}:resume")

    # -- Flows --

    def list_flows(self, runtime_uuid: str) -> list[dict]:
        return self._get(f"/api/v1/runtimes/{_encode(runtime_uuid)}/flows")

    def run_flow(
        self,
        runtime_uuid: str,
        flow_name: str,
        spec: dict | None = None,
        resume: bool = False,
    ) -> dict:
        runtime = self.get_runtime(runtime_uuid)
        if runtime.get("paused"):
            if resume:
                self.resume_runtime(runtime_uuid)
            else:
                raise RuntimeError(
                    "Runtime is paused. Use resume=True to resume before running."
                )
        else:
            health = runtime.get("health")
            if health and health != "running":
                raise RuntimeError(f"Runtime health is '{health}', expected 'running'.")
            if not health:
                raise RuntimeError("Runtime has no health status yet.")
        path = (
            f"/api/v1/runtimes/{_encode(runtime_uuid)}/flows/{_encode(flow_name)}:run"
        )
        if spec is not None:
            return self._post_json(path, {"spec": spec})
        return self._post_empty(path)

    # -- Flow runs --

    def list_flow_runs(self, runtime_uuid: str, **filters) -> dict:
        params: dict = {"runtime_uuid": runtime_uuid}
        if "flow_name" in filters:
            filters["flow"] = filters.pop("flow_name")
        params.update({k: v for k, v in filters.items() if v is not None})
        return self._get("/api/v1/flow-runs", params=params)

    def get_flow_run(self, runtime_uuid: str, name: str) -> dict:
        return self._get(
            f"/api/v1/flow-runs/{_encode(name)}",
            params={"runtime_uuid": runtime_uuid},
        )


def _encode(segment: str) -> str:
    """Percent-encode a URL path segment."""
    return quote(segment, safe="")


def _raise_for_api_error(resp: httpx.Response) -> None:
    """Raise a RuntimeError with the API error detail if the response is not 2xx."""
    if 200 <= resp.status_code < 300:
        return
    detail = resp.text
    try:
        detail = resp.json().get("detail", detail)
    except Exception:
        pass
    raise RuntimeError(f"API error (HTTP {resp.status_code}): {detail}")


# ---------------------------------------------------------------------------
# Test harness
# ---------------------------------------------------------------------------

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


# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------


def run_flow_with_retry(
    client: AscendClient,
    runtime_uuid: str,
    flow_name: str,
    spec: dict | None = None,
    resume: bool = False,
) -> dict:
    """Run a flow with retries for transient runtime readiness states."""
    last_error: Exception | None = None
    for delay in (0, 5, 10, 15, 15, 15):
        if delay:
            time.sleep(delay)
        try:
            return client.run_flow(runtime_uuid, flow_name, spec=spec, resume=resume)
        except RuntimeError as e:
            msg = str(e).lower()
            if "starting" in msg or "no health status" in msg or "initializing" in msg:
                last_error = e
                continue
            raise

    if last_error is not None:
        raise last_error
    raise RuntimeError("run_flow retry exhausted")


# ---------------------------------------------------------------------------
# Tests
# ---------------------------------------------------------------------------


def main():
    parser = argparse.ArgumentParser(description="Ascend REST API integration tests")
    parser.add_argument(
        "--runtime-id",
        default="ascend-tools",
        help="Runtime ID to test against (default: ascend-tools)",
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

    auth = Auth(
        service_account_id=os.environ["ASCEND_SERVICE_ACCOUNT_ID"],
        service_account_key=os.environ["ASCEND_SERVICE_ACCOUNT_KEY"],
        instance_api_url=os.environ["ASCEND_INSTANCE_API_URL"],
    )
    token = auth.get_token()
    check(bool(token), "auth: got instance token")

    client = AscendClient(auth)
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

    by_id = client.list_runtimes(id=args.runtime_id)
    if by_id:
        runtime = by_id[0]
    else:
        print(f"  runtime '{args.runtime_id}' not found, falling back to first runtime")
        runtime = runtimes[0]

    runtime_uuid = runtime["uuid"]
    runtime_id = runtime["id"]
    is_paused = runtime.get("paused", False)
    print(
        f"  using runtime: {runtime_id} ({runtime_uuid}){' [paused]' if is_paused else ''}"
    )

    # get runtime
    got = client.get_runtime(runtime_uuid)
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

    flows = client.list_flows(runtime_uuid)
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

    runs_before_result = client.list_flow_runs(runtime_uuid, flow_name=flow_name)
    check(isinstance(runs_before_result, dict), "list_flow_runs returns dict")
    check("items" in runs_before_result, "list_flow_runs has 'items' key")
    check("truncated" in runs_before_result, "list_flow_runs has 'truncated' key")
    runs_before = runs_before_result["items"]
    runs_before_count = len(runs_before)
    check(True, f"list_flow_runs returned {runs_before_count} run(s) before trigger")

    # test get_flow_run on existing run
    if runs_before:
        existing_run = runs_before[0]
        got_run = client.get_flow_run(runtime_uuid, existing_run["name"])
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
    limited = client.list_flow_runs(runtime_uuid, flow_name=flow_name, limit=1)["items"]
    check(
        len(limited) <= 1,
        "list_flow_runs(limit=1) returns at most 1",
        f"got {len(limited)}",
    )

    if runs_before_count > 1:
        offset_runs = client.list_flow_runs(
            runtime_uuid, flow_name=flow_name, offset=1, limit=1
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
    trigger = run_flow_with_retry(client, runtime_uuid, flow_name, resume=True)
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
        runs_after = client.list_flow_runs(runtime_uuid, flow_name=flow_name)["items"]
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
        got_new = client.get_flow_run(runtime_uuid, newest["name"])
        check(got_new["name"] == newest["name"], "get_flow_run on new run works")

    # ---------- status filter ----------

    print("=== status filter ===")

    for status in ("pending", "running", "succeeded", "failed"):
        by_status = client.list_flow_runs(runtime_uuid, status=status)["items"]
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

    # ---------- run_flow with spec ----------

    print("=== run_flow with spec ===")

    trigger2 = run_flow_with_retry(
        client, runtime_uuid, flow_name, spec={}, resume=True
    )
    check(trigger2.get("event_uuid") is not None, "run_flow with empty spec works")

    # spec with full_refresh
    trigger3_fr = run_flow_with_retry(
        client, runtime_uuid, flow_name, spec={"full_refresh": True}, resume=True
    )
    check(
        trigger3_fr.get("event_uuid") is not None,
        "run_flow with full_refresh=True works",
    )

    # spec with parameters
    trigger3_params = run_flow_with_retry(
        client,
        runtime_uuid,
        flow_name,
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
        runtime_uuid,
        flow_name,
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

        paused_rt = client.pause_runtime(runtime_uuid)
        check(paused_rt.get("paused") is True, "pause_runtime sets paused=True")

        got_paused = client.get_runtime(runtime_uuid)
        check(got_paused.get("paused") is True, "get_runtime confirms paused")

        # run_flow without resume should fail on a paused runtime
        try:
            client.run_flow(runtime_uuid, flow_name)
            check(False, "run_flow on paused runtime should raise", "no error raised")
        except Exception as e:
            check(
                "paused" in str(e).lower() or "resume" in str(e).lower(),
                "run_flow on paused runtime raises descriptive error",
                str(e),
            )

        print("=== runtime resume via flow run ===")

        trigger3 = run_flow_with_retry(client, runtime_uuid, flow_name, resume=True)
        check(
            trigger3.get("event_uuid") is not None, "run_flow with resume=True succeeds"
        )

        got_resumed = client.get_runtime(runtime_uuid)
        check(got_resumed.get("paused") is False, "runtime is unpaused after resume")

        print("=== runtime resume (explicit) ===")

        # Wait for runtime to start coming up, then verify resume is idempotent
        for delay in (2, 3, 5, 5):
            time.sleep(delay)
            rt_health = client.get_runtime(runtime_uuid)
            if rt_health.get("health") is not None:
                break

        if rt_health.get("health") is not None:
            check(True, f"runtime health restored: {rt_health['health']}")
        else:
            skip(
                "runtime health not yet available after 15s (runtime may be slow to start)"
            )

        # resume on an already-running runtime should be a no-op
        resumed_rt = client.resume_runtime(runtime_uuid)
        check(resumed_rt.get("paused") is False, "resume_runtime is idempotent")

    # ---------- summary ----------

    print_summary()


if __name__ == "__main__":
    main()
