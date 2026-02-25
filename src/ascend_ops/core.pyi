class Client:
    def __init__(
        self,
        service_account_id: str | None = None,
        service_account_key: str | None = None,
        instance_api_url: str | None = None,
    ) -> None: ...
    def list_runtimes(
        self,
        id: str | None = None,
        kind: str | None = None,
        project_uuid: str | None = None,
        environment_uuid: str | None = None,
    ) -> str: ...
    def get_runtime(self, uuid: str) -> str: ...
    def list_flows(self, runtime_uuid: str) -> str: ...
    def run_flow(
        self,
        runtime_uuid: str,
        flow_name: str,
        spec: str | None = None,
    ) -> str: ...
    def list_flow_runs(
        self,
        runtime_uuid: str,
        status: str | None = None,
        flow: str | None = None,
        since: str | None = None,
        until: str | None = None,
        offset: int | None = None,
        limit: int | None = None,
    ) -> str: ...
    def get_flow_run(self, runtime_uuid: str, name: str) -> str: ...

def run(argv: list[str]) -> None: ...
