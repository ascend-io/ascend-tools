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
        title: str | None = None,
        kind: str | None = None,
        project: str | None = None,
        environment: str | None = None,
    ) -> list[dict[str, Any]]:
        """List runtimes, optionally filtered. Project/environment accept names or UUIDs."""
        ...
    # -- Workspaces --
    def list_workspaces(
        self,
        *,
        title: str | None = None,
        project: str | None = None,
        environment: str | None = None,
    ) -> list[dict[str, Any]]:
        """List workspaces. Accepts project/environment names or UUIDs."""
        ...
    def get_workspace(self, *, title: str, uuid: str | None = None) -> dict[str, Any]:
        """Get a workspace by title (or UUID)."""
        ...
    def create_workspace(
        self,
        *,
        title: str,
        environment: str,
        project: str,
        profile: str,
        git_branch: str,
        git_branch_base: str | None = None,
        size: str | None = None,
        storage_size: int | None = None,
        auto_snooze_timeout_minutes: int | None = None,
    ) -> dict[str, Any]:
        """Create a workspace. Accepts environment/project names or UUIDs."""
        ...
    def update_workspace(
        self,
        *,
        title: str,
        uuid: str | None = None,
        new_title: str | None = None,
        git_branch: str | None = None,
        git_branch_base: str | None = None,
        profile: str | None = None,
        size: str | None = None,
        storage_size: int | None = None,
        auto_snooze_timeout_minutes: int | None = None,
    ) -> dict[str, Any]:
        """Update a workspace by title. Only provided fields are changed."""
        ...
    def pause_workspace(self, *, title: str, uuid: str | None = None) -> dict[str, Any]:
        """Pause a workspace."""
        ...
    def resume_workspace(
        self, *, title: str, uuid: str | None = None
    ) -> dict[str, Any]:
        """Resume a paused workspace."""
        ...
    def delete_workspace(self, *, title: str, uuid: str | None = None) -> None:
        """Delete a workspace."""
        ...

    # -- Deployments --

    def list_deployments(
        self,
        *,
        title: str | None = None,
        project: str | None = None,
        environment: str | None = None,
    ) -> list[dict[str, Any]]:
        """List deployments. Accepts project/environment names or UUIDs."""
        ...
    def get_deployment(self, *, title: str, uuid: str | None = None) -> dict[str, Any]:
        """Get a deployment by title (or UUID)."""
        ...
    def create_deployment(
        self,
        *,
        title: str,
        environment: str,
        project: str,
        profile: str,
        git_branch: str,
        git_branch_base: str | None = None,
        size: str | None = None,
        storage_size: int | None = None,
        enable_automations: bool | None = None,
    ) -> dict[str, Any]:
        """Create a deployment. Accepts environment/project names or UUIDs."""
        ...
    def update_deployment(
        self,
        *,
        title: str,
        uuid: str | None = None,
        new_title: str | None = None,
        git_branch: str | None = None,
        git_branch_base: str | None = None,
        profile: str | None = None,
        size: str | None = None,
        storage_size: int | None = None,
        enable_automations: bool | None = None,
    ) -> dict[str, Any]:
        """Update a deployment by title. Only provided fields are changed."""
        ...
    def pause_deployment_automations(
        self, *, title: str, uuid: str | None = None
    ) -> dict[str, Any]:
        """Pause automations on a deployment."""
        ...
    def resume_deployment_automations(
        self, *, title: str, uuid: str | None = None
    ) -> dict[str, Any]:
        """Resume automations on a deployment."""
        ...
    def delete_deployment(self, *, title: str, uuid: str | None = None) -> None:
        """Delete a deployment."""
        ...

    # -- Runtime primitives (low-level) --

    def get_runtime(self, *, uuid: str) -> dict[str, Any]:
        """Get a runtime by UUID."""
        ...
    def resume_runtime(self, *, uuid: str) -> dict[str, Any]:
        """Resume a paused runtime."""
        ...
    def pause_runtime(self, *, uuid: str) -> dict[str, Any]:
        """Pause a running runtime."""
        ...
    def resolve_runtime_by_title(self, *, title: str, kind: str) -> dict[str, Any]:
        """Resolve a runtime by title and kind. Returns exactly one match."""
        ...
    def create_runtime(
        self,
        *,
        title: str,
        kind: str,
        environment: str,
        project: str,
        profile_name: str,
        working_git_branch: str,
        base_git_branch: str | None = None,
        size: str | None = None,
        storage_size: int | None = None,
        enable_automations: bool | None = None,
        auto_snooze_timeout_minutes: int | None = None,
    ) -> dict[str, Any]:
        """Create a runtime (workspace or deployment). Accepts environment/project names or UUIDs."""
        ...
    def update_runtime(
        self,
        *,
        uuid: str,
        title: str | None = None,
        working_git_branch: str | None = None,
        base_git_branch: str | None = None,
        profile_name: str | None = None,
        size: str | None = None,
        storage_size: int | None = None,
        enable_automations: bool | None = None,
        auto_snooze_timeout_minutes: int | None = None,
    ) -> dict[str, Any]:
        """Update a runtime. Only provided fields are changed (PATCH semantics)."""
        ...
    def delete_runtime(self, *, uuid: str) -> None:
        """Delete a runtime."""
        ...
    def list_environments(self) -> list[dict[str, Any]]:
        """List all environments."""
        ...
    def get_environment(self, *, title: str) -> dict[str, Any]:
        """Get an environment by title. Errors if zero or multiple matches."""
        ...
    def list_projects(self) -> list[dict[str, Any]]:
        """List all projects."""
        ...
    def get_project(self, *, title: str) -> dict[str, Any]:
        """Get a project by title. Errors if zero or multiple matches."""
        ...
    def list_profiles(
        self,
        *,
        runtime_uuid: str | None = None,
        project: str | None = None,
        branch: str | None = None,
    ) -> list[str]:
        """List available profiles. Provide runtime_uuid or project+branch (name or UUID)."""
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
    def list_otto_providers(self) -> list[dict[str, Any]]:
        """List available Otto providers and their enabled models."""
        ...
    def otto_chat(
        self,
        *,
        prompt: str,
        runtime_uuid: str | None = None,
        thread_id: str | None = None,
        model: str | None = None,
        provider: str | None = None,
    ) -> dict[str, Any]:
        """Chat with Otto AI assistant. Returns {"message": str, "thread_id": str | None}."""
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
