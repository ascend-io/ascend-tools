import json

from ascend_ops.core import Client as _RustClient


class Client:
    """Ascend API client.

    Authenticates via service account credentials and provides
    access to the Ascend Instance API.
    """

    def __init__(
        self,
        *,
        service_account_id: str,
        private_key: str,
        instance_api_url: str,
        org_id: str,
        cloud_api_url: str | None = None,
    ):
        self._inner = _RustClient(
            service_account_id=service_account_id,
            private_key=private_key,
            instance_api_url=instance_api_url,
            org_id=org_id,
            cloud_api_url=cloud_api_url,
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

    def get_runtime(self, uuid: str) -> dict:
        return json.loads(self._inner.get_runtime(uuid))

    def run_flow(
        self,
        *,
        runtime_uuid: str,
        flow_name: str,
        spec: dict | None = None,
    ) -> dict:
        spec_json = json.dumps(spec) if spec else None
        return json.loads(self._inner.run_flow(runtime_uuid, flow_name, spec_json))

    def backfill_flow(
        self,
        *,
        runtime_uuid: str,
        flow_name: str,
        spec: dict | None = None,
    ) -> dict:
        spec_json = json.dumps(spec) if spec else None
        return json.loads(self._inner.backfill_flow(runtime_uuid, flow_name, spec_json))

    def list_flow_runs(
        self,
        *,
        runtime_uuid: str,
        status: str | None = None,
        flow: str | None = None,
    ) -> list[dict]:
        return json.loads(self._inner.list_flow_runs(runtime_uuid, status, flow))

    def get_flow_run(self, *, runtime_uuid: str, name: str) -> dict:
        return json.loads(self._inner.get_flow_run(runtime_uuid, name))

    def list_builds(self, *, runtime_uuid: str) -> list[dict]:
        return json.loads(self._inner.list_builds(runtime_uuid))

    def get_build(self, uuid: str) -> dict:
        return json.loads(self._inner.get_build(uuid))


__all__ = ["Client"]
