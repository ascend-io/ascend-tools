from typing import Any

class Client:
    """Client for the Ascend Instance API.

    Authenticates via service account credentials (Ed25519 JWT → instance token).
    All parameters are optional and fall back to environment variables
    (ASCEND_SERVICE_ACCOUNT_ID, ASCEND_SERVICE_ACCOUNT_KEY, ASCEND_INSTANCE_API_URL).
    """

    def __init__(
        self,
        *,
        service_account_id: str | None = None,
        service_account_key: str | None = None,
        instance_api_url: str | None = None,
    ) -> None: ...
    def list_runtimes(
        self,
        *,
        id: str | None = None,
        kind: str | None = None,
        project_uuid: str | None = None,
        environment_uuid: str | None = None,
    ) -> list[dict[str, Any]]:
        """List runtimes, optionally filtered by id, kind, project, or environment."""
        ...
    def get_runtime(self, *, uuid: str) -> dict[str, Any]:
        """Get a runtime by UUID."""
        ...
    def resume_runtime(self, *, uuid: str) -> dict[str, Any]:
        """Resume a paused runtime."""
        ...
    def pause_runtime(self, *, uuid: str) -> dict[str, Any]:
        """Pause a running runtime."""
        ...
    def list_flows(self, *, runtime_uuid: str) -> list[dict[str, Any]]:
        """List flows in a runtime."""
        ...
    def run_flow(
        self,
        *,
        runtime_uuid: str,
        flow_name: str,
        spec: dict[str, Any] | None = None,
        resume: bool = False,
    ) -> dict[str, Any]:
        """Trigger a flow run. Set resume=True to resume a paused runtime first."""
        ...
    def list_flow_runs(
        self,
        *,
        runtime_uuid: str,
        status: str | None = None,
        flow_name: str | None = None,
        since: str | None = None,
        until: str | None = None,
        offset: int | None = None,
        limit: int | None = None,
    ) -> dict[str, Any]:
        """List flow runs, optionally filtered by status, flow name, or time range.

        Returns ``{"items": [...], "truncated": bool}``. The ``truncated`` flag
        indicates the server-side row limit was reached and results may be incomplete.
        """
        ...
    def get_flow_run(self, *, runtime_uuid: str, name: str) -> dict[str, Any]:
        """Get a flow run by name."""
        ...

def run(argv: list[str]) -> None:
    """Run the CLI with the given arguments."""
    ...

def run_mcp_http(
    bind_addr: str,
    *,
    service_account_id: str | None = None,
    service_account_key: str | None = None,
    instance_api_url: str | None = None,
) -> None:
    """Start the MCP HTTP server. Blocks until shut down.

    Call from a background thread (e.g. ``asyncio.to_thread(run_mcp_http, "127.0.0.1:4201")``)
    since it blocks the calling thread.
    """
    ...
