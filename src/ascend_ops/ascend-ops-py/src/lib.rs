use ascend_ops::client::AscendClient;
use ascend_ops::config::Config;
use ascend_ops::models;
use pyo3::prelude::*;

#[pyclass]
struct Client {
    inner: AscendClient,
}

#[pymethods]
impl Client {
    #[new]
    #[pyo3(signature = (service_account_id=None, service_account_key=None, instance_api_url=None))]
    fn new(
        service_account_id: Option<&str>,
        service_account_key: Option<&str>,
        instance_api_url: Option<&str>,
    ) -> PyResult<Self> {
        let config = Config::with_overrides(
            service_account_id,
            service_account_key,
            instance_api_url,
        )
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))?;

        let inner = AscendClient::new(config)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))?;

        Ok(Self { inner })
    }

    #[pyo3(signature = (id=None, kind=None, project_uuid=None, environment_uuid=None))]
    fn list_runtimes(
        &self,
        id: Option<&str>,
        kind: Option<&str>,
        project_uuid: Option<&str>,
        environment_uuid: Option<&str>,
    ) -> PyResult<String> {
        let runtimes = self
            .inner
            .list_runtimes(models::RuntimeFilters {
                id: id.map(String::from),
                kind: kind.map(String::from),
                project_uuid: project_uuid.map(String::from),
                environment_uuid: environment_uuid.map(String::from),
            })
            .map_err(to_py_err)?;
        serde_json::to_string(&runtimes).map_err(to_py_err)
    }

    fn get_runtime(&self, uuid: &str) -> PyResult<String> {
        let runtime = self.inner.get_runtime(uuid).map_err(to_py_err)?;
        serde_json::to_string(&runtime).map_err(to_py_err)
    }

    fn list_flows(&self, runtime_uuid: &str) -> PyResult<String> {
        let flows = self.inner.list_flows(runtime_uuid).map_err(to_py_err)?;
        serde_json::to_string(&flows).map_err(to_py_err)
    }

    #[pyo3(signature = (runtime_uuid, flow_name, spec=None))]
    fn run_flow(
        &self,
        runtime_uuid: &str,
        flow_name: &str,
        spec: Option<&str>,
    ) -> PyResult<String> {
        let spec_value = match spec {
            Some(s) => Some(
                serde_json::from_str(s)
                    .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))?,
            ),
            None => None,
        };
        let trigger = self
            .inner
            .run_flow(runtime_uuid, flow_name, spec_value)
            .map_err(to_py_err)?;
        serde_json::to_string(&trigger).map_err(to_py_err)
    }

    #[pyo3(signature = (runtime_uuid, status=None, flow=None, since=None, until=None, offset=None, limit=None))]
    fn list_flow_runs(
        &self,
        runtime_uuid: &str,
        status: Option<&str>,
        flow: Option<&str>,
        since: Option<&str>,
        until: Option<&str>,
        offset: Option<u64>,
        limit: Option<u64>,
    ) -> PyResult<String> {
        let runs = self
            .inner
            .list_flow_runs(
                runtime_uuid,
                models::FlowRunFilters {
                    status: status.map(String::from),
                    flow: flow.map(String::from),
                    since: since.map(String::from),
                    until: until.map(String::from),
                    offset,
                    limit,
                },
            )
            .map_err(to_py_err)?;
        serde_json::to_string(&runs).map_err(to_py_err)
    }

    fn get_flow_run(&self, runtime_uuid: &str, name: &str) -> PyResult<String> {
        let run = self
            .inner
            .get_flow_run(runtime_uuid, name)
            .map_err(to_py_err)?;
        serde_json::to_string(&run).map_err(to_py_err)
    }

}

#[pyfunction]
fn run(argv: Vec<String>) -> PyResult<()> {
    ascend_ops_cli::run(argv.iter().map(|s| s.as_str()))
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))
}

#[pymodule]
fn core(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<Client>()?;
    m.add_function(wrap_pyfunction!(run, m)?)?;
    Ok(())
}

fn to_py_err(e: impl std::fmt::Display) -> PyErr {
    PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string())
}
