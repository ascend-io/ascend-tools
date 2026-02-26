from typing import Any

class Client:
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
    ) -> list[dict[str, Any]]: ...
    def get_runtime(self, *, uuid: str) -> dict[str, Any]: ...
    def resume_runtime(self, *, uuid: str) -> dict[str, Any]: ...
    def pause_runtime(self, *, uuid: str) -> dict[str, Any]: ...
    def list_flows(self, *, runtime_uuid: str) -> list[dict[str, Any]]: ...
    def run_flow(
        self,
        *,
        runtime_uuid: str,
        flow_name: str,
        spec: dict[str, Any] | None = None,
        resume: bool = False,
    ) -> dict[str, Any]: ...
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
    ) -> list[dict[str, Any]]: ...
    def get_flow_run(self, *, runtime_uuid: str, name: str) -> dict[str, Any]: ...

def run(argv: list[str]) -> None: ...
