import json

from ascend_ops.core import Client as _RustClient


class Client:
    """Ascend API client.

    Authenticates via service account credentials and provides
    access to the Ascend Instance API.

    All parameters are optional — if not provided, they are resolved
    from environment variables:
      - ASCEND_SERVICE_ACCOUNT_ID
      - ASCEND_SERVICE_ACCOUNT_KEY
      - ASCEND_INSTANCE_API_URL
    """

    def __init__(
        self,
        *,
        service_account_id: str | None = None,
        service_account_key: str | None = None,
        instance_api_url: str | None = None,
    ):
        self._inner = _RustClient(
            service_account_id=service_account_id,
            service_account_key=service_account_key,
            instance_api_url=instance_api_url,
        )

    def list_runtimes(
        self,
        *,
        id: str | None = None,
        kind: str | None = None,
        project_uuid: str | None = None,
        environment_uuid: str | None = None,
    ) -> list[dict]:
        return json.loads(
            self._inner.list_runtimes(id, kind, project_uuid, environment_uuid)
        )

    def get_runtime(self, *, uuid: str) -> dict:
        return json.loads(self._inner.get_runtime(uuid))

    def list_flows(self, *, runtime_uuid: str) -> list[dict]:
        return json.loads(self._inner.list_flows(runtime_uuid))

    def run_flow(
        self,
        *,
        runtime_uuid: str,
        flow_name: str,
        spec: dict | None = None,
    ) -> dict:
        spec_json = json.dumps(spec) if spec is not None else None
        return json.loads(self._inner.run_flow(runtime_uuid, flow_name, spec_json))

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
    ) -> list[dict]:
        return json.loads(
            self._inner.list_flow_runs(
                runtime_uuid, status, flow_name, since, until, offset, limit
            )
        )

    def get_flow_run(self, *, runtime_uuid: str, name: str) -> dict:
        return json.loads(self._inner.get_flow_run(runtime_uuid, name))
