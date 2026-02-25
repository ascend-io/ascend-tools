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
    #[pyo3(signature = (*, service_account_id=None, service_account_key=None, instance_api_url=None))]
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

    #[pyo3(signature = (*, id=None, kind=None, project_uuid=None, environment_uuid=None))]
    fn list_runtimes(
        &self,
        py: Python<'_>,
        id: Option<&str>,
        kind: Option<&str>,
        project_uuid: Option<&str>,
        environment_uuid: Option<&str>,
    ) -> PyResult<Py<PyAny>> {
        let runtimes = self
            .inner
            .list_runtimes(models::RuntimeFilters {
                id: id.map(String::from),
                kind: kind.map(String::from),
                project_uuid: project_uuid.map(String::from),
                environment_uuid: environment_uuid.map(String::from),
            })
            .map_err(to_py_err)?;
        to_python(py, &runtimes)
    }

    #[pyo3(signature = (*, uuid))]
    fn get_runtime(&self, py: Python<'_>, uuid: &str) -> PyResult<Py<PyAny>> {
        let runtime = self.inner.get_runtime(uuid).map_err(to_py_err)?;
        to_python(py, &runtime)
    }

    #[pyo3(signature = (*, runtime_uuid))]
    fn list_flows(&self, py: Python<'_>, runtime_uuid: &str) -> PyResult<Py<PyAny>> {
        let flows = self.inner.list_flows(runtime_uuid).map_err(to_py_err)?;
        to_python(py, &flows)
    }

    #[pyo3(signature = (*, runtime_uuid, flow_name, spec=None))]
    fn run_flow(
        &self,
        py: Python<'_>,
        runtime_uuid: &str,
        flow_name: &str,
        spec: Option<&Bound<'_, PyAny>>,
    ) -> PyResult<Py<PyAny>> {
        let spec_value: Option<serde_json::Value> = match spec {
            Some(obj) => Some(pythonize::depythonize(obj)?),
            None => None,
        };
        let trigger = self
            .inner
            .run_flow(runtime_uuid, flow_name, spec_value)
            .map_err(to_py_err)?;
        to_python(py, &trigger)
    }

    #[pyo3(signature = (*, runtime_uuid, status=None, flow_name=None, since=None, until=None, offset=None, limit=None))]
    fn list_flow_runs(
        &self,
        py: Python<'_>,
        runtime_uuid: &str,
        status: Option<&str>,
        flow_name: Option<&str>,
        since: Option<&str>,
        until: Option<&str>,
        offset: Option<u64>,
        limit: Option<u64>,
    ) -> PyResult<Py<PyAny>> {
        let runs = self
            .inner
            .list_flow_runs(
                runtime_uuid,
                models::FlowRunFilters {
                    status: status.map(String::from),
                    flow: flow_name.map(String::from),
                    since: since.map(String::from),
                    until: until.map(String::from),
                    offset,
                    limit,
                },
            )
            .map_err(to_py_err)?;
        to_python(py, &runs)
    }

    #[pyo3(signature = (*, runtime_uuid, name))]
    fn get_flow_run(
        &self,
        py: Python<'_>,
        runtime_uuid: &str,
        name: &str,
    ) -> PyResult<Py<PyAny>> {
        let run = self
            .inner
            .get_flow_run(runtime_uuid, name)
            .map_err(to_py_err)?;
        to_python(py, &run)
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

fn to_python(py: Python<'_>, value: &impl serde::Serialize) -> PyResult<Py<PyAny>> {
    pythonize::pythonize(py, value)
        .map(Bound::unbind)
        .map_err(to_py_err)
}

fn to_py_err(e: impl std::fmt::Display) -> PyErr {
    PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string())
}
