use ascend_tools::client::AscendClient;
use ascend_tools::config::Config;
use ascend_tools::models;
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
        let config =
            Config::with_overrides(service_account_id, service_account_key, instance_api_url)
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
        let runtimes = py
            .detach(|| {
                let mut filters = models::RuntimeFilters::default();
                filters.id = id.map(String::from);
                filters.kind = kind.map(String::from);
                filters.project_uuid = project_uuid.map(String::from);
                filters.environment_uuid = environment_uuid.map(String::from);
                self.inner.list_runtimes(filters)
            })
            .map_err(to_py_err)?;
        to_python(py, &runtimes)
    }

    #[pyo3(signature = (*, uuid))]
    fn get_runtime(&self, py: Python<'_>, uuid: &str) -> PyResult<Py<PyAny>> {
        let runtime = py
            .detach(|| self.inner.get_runtime(uuid))
            .map_err(to_py_err)?;
        to_python(py, &runtime)
    }

    #[pyo3(signature = (*, uuid))]
    fn resume_runtime(&self, py: Python<'_>, uuid: &str) -> PyResult<Py<PyAny>> {
        let runtime = py
            .detach(|| self.inner.resume_runtime(uuid))
            .map_err(to_py_err)?;
        to_python(py, &runtime)
    }

    #[pyo3(signature = (*, uuid))]
    fn pause_runtime(&self, py: Python<'_>, uuid: &str) -> PyResult<Py<PyAny>> {
        let runtime = py
            .detach(|| self.inner.pause_runtime(uuid))
            .map_err(to_py_err)?;
        to_python(py, &runtime)
    }

    #[pyo3(signature = (*, runtime_uuid))]
    fn list_flows(&self, py: Python<'_>, runtime_uuid: &str) -> PyResult<Py<PyAny>> {
        let flows = py
            .detach(|| self.inner.list_flows(runtime_uuid))
            .map_err(to_py_err)?;
        to_python(py, &flows)
    }

    #[pyo3(signature = (*, runtime_uuid, flow_name, spec=None, resume=false))]
    fn run_flow(
        &self,
        py: Python<'_>,
        runtime_uuid: &str,
        flow_name: &str,
        spec: Option<&Bound<'_, PyAny>>,
        resume: bool,
    ) -> PyResult<Py<PyAny>> {
        let spec_value: Option<serde_json::Value> = match spec {
            Some(obj) => Some(pythonize::depythonize(obj)?),
            None => None,
        };
        let trigger = py
            .detach(|| {
                self.inner
                    .run_flow(runtime_uuid, flow_name, spec_value, resume)
            })
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
        let runs = py
            .detach(|| {
                let mut filters = models::FlowRunFilters::default();
                filters.status = status.map(String::from);
                filters.flow = flow_name.map(String::from);
                filters.since = since.map(String::from);
                filters.until = until.map(String::from);
                filters.offset = offset;
                filters.limit = limit;
                self.inner.list_flow_runs(runtime_uuid, filters)
            })
            .map_err(to_py_err)?;
        to_python(py, &runs)
    }

    #[pyo3(signature = (*, runtime_uuid, name))]
    fn get_flow_run(&self, py: Python<'_>, runtime_uuid: &str, name: &str) -> PyResult<Py<PyAny>> {
        let run = py
            .detach(|| self.inner.get_flow_run(runtime_uuid, name))
            .map_err(to_py_err)?;
        to_python(py, &run)
    }
}

#[pyfunction]
fn run(py: Python<'_>, argv: Vec<String>) -> PyResult<()> {
    py.detach(|| {
        ascend_tools_cli::run(argv.iter().map(|s| s.as_str()))
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))
    })
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
