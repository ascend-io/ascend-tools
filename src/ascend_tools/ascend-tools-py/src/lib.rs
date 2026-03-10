#![forbid(unsafe_code)]

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

    #[pyo3(signature = (*, id=None, title=None, kind=None, project=None, environment=None))]
    fn list_runtimes(
        &self,
        py: Python<'_>,
        id: Option<&str>,
        title: Option<&str>,
        kind: Option<&str>,
        project: Option<&str>,
        environment: Option<&str>,
    ) -> PyResult<Py<PyAny>> {
        let runtimes = py
            .detach(|| {
                let mut filters = models::RuntimeFilters::default();
                filters.id = id.map(String::from);
                filters.title = title.map(String::from);
                filters.kind = kind.map(String::from);
                filters.project = project.map(String::from);
                filters.environment = environment.map(String::from);
                self.inner.list_runtimes(filters)
            })
            .map_err(to_py_err)?;
        to_python(py, &runtimes)
    }

    // -- Workspace convenience methods --

    #[pyo3(signature = (*, title=None, project=None, environment=None))]
    fn list_workspaces(
        &self,
        py: Python<'_>,
        title: Option<&str>,
        project: Option<&str>,
        environment: Option<&str>,
    ) -> PyResult<Py<PyAny>> {
        let runtimes = py
            .detach(|| {
                let mut filters = models::RuntimeFilters::default();
                filters.title = title.map(String::from);
                filters.project = project.map(String::from);
                filters.environment = environment.map(String::from);
                self.inner.list_workspaces(filters)
            })
            .map_err(to_py_err)?;
        to_python(py, &runtimes)
    }

    #[pyo3(signature = (*, title, uuid=None))]
    fn get_workspace(
        &self,
        py: Python<'_>,
        title: &str,
        uuid: Option<&str>,
    ) -> PyResult<Py<PyAny>> {
        let runtime = py
            .detach(|| self.inner.get_workspace(title, uuid))
            .map_err(to_py_err)?;
        to_python(py, &runtime)
    }

    #[pyo3(signature = (*, title, uuid=None))]
    fn pause_workspace(&self, py: Python<'_>, title: &str, uuid: Option<&str>) -> PyResult<Py<PyAny>> {
        let runtime = py
            .detach(|| self.inner.pause_workspace(title, uuid))
            .map_err(to_py_err)?;
        to_python(py, &runtime)
    }

    #[pyo3(signature = (*, title, uuid=None))]
    fn resume_workspace(&self, py: Python<'_>, title: &str, uuid: Option<&str>) -> PyResult<Py<PyAny>> {
        let runtime = py
            .detach(|| self.inner.resume_workspace(title, uuid))
            .map_err(to_py_err)?;
        to_python(py, &runtime)
    }

    #[pyo3(signature = (*, title, environment, project, profile, git_branch, git_branch_base=None, size=None, storage_size=None, auto_snooze_timeout_minutes=None))]
    #[allow(clippy::too_many_arguments)]
    fn create_workspace(
        &self,
        py: Python<'_>,
        title: &str,
        environment: &str,
        project: &str,
        profile: &str,
        git_branch: &str,
        git_branch_base: Option<&str>,
        size: Option<&str>,
        storage_size: Option<u32>,
        auto_snooze_timeout_minutes: Option<u32>,
    ) -> PyResult<Py<PyAny>> {
        let create = models::RuntimeCreate {
            title: title.into(),
            environment: environment.into(),
            project: project.into(),
            profile_name: profile.into(),
            working_git_branch: git_branch.into(),
            base_git_branch: git_branch_base.map(String::from),
            size: size.map(String::from),
            storage_size,
            enable_automations: None,
            auto_snooze_timeout_minutes,
        };
        let runtime = py.detach(|| self.inner.create_workspace(&create)).map_err(to_py_err)?;
        to_python(py, &runtime)
    }

    #[pyo3(signature = (*, title, uuid=None, new_title=None, git_branch=None, git_branch_base=None, profile=None, size=None, storage_size=None, auto_snooze_timeout_minutes=None))]
    #[allow(clippy::too_many_arguments)]
    fn update_workspace(
        &self,
        py: Python<'_>,
        title: &str,
        uuid: Option<&str>,
        new_title: Option<&str>,
        git_branch: Option<&str>,
        git_branch_base: Option<&str>,
        profile: Option<&str>,
        size: Option<&str>,
        storage_size: Option<u32>,
        auto_snooze_timeout_minutes: Option<u32>,
    ) -> PyResult<Py<PyAny>> {
        let update = models::RuntimeUpdate {
            title: new_title.map(String::from), working_git_branch: git_branch.map(String::from),
            base_git_branch: git_branch_base.map(String::from), profile_name: profile.map(String::from),
            size: size.map(String::from), storage_size, enable_automations: None, auto_snooze_timeout_minutes,
        };
        let runtime = py.detach(|| self.inner.update_workspace(title, uuid, &update)).map_err(to_py_err)?;
        to_python(py, &runtime)
    }

    #[pyo3(signature = (*, title, uuid=None))]
    fn delete_workspace(&self, py: Python<'_>, title: &str, uuid: Option<&str>) -> PyResult<()> {
        py.detach(|| self.inner.delete_workspace(title, uuid))
            .map_err(to_py_err)?;
        Ok(())
    }

    // -- Deployment convenience methods --

    #[pyo3(signature = (*, title=None, project=None, environment=None))]
    fn list_deployments(
        &self,
        py: Python<'_>,
        title: Option<&str>,
        project: Option<&str>,
        environment: Option<&str>,
    ) -> PyResult<Py<PyAny>> {
        let runtimes = py
            .detach(|| {
                let mut filters = models::RuntimeFilters::default();
                filters.title = title.map(String::from);
                filters.project = project.map(String::from);
                filters.environment = environment.map(String::from);
                self.inner.list_deployments(filters)
            })
            .map_err(to_py_err)?;
        to_python(py, &runtimes)
    }

    #[pyo3(signature = (*, title, uuid=None))]
    fn get_deployment(
        &self,
        py: Python<'_>,
        title: &str,
        uuid: Option<&str>,
    ) -> PyResult<Py<PyAny>> {
        let runtime = py
            .detach(|| self.inner.get_deployment(title, uuid))
            .map_err(to_py_err)?;
        to_python(py, &runtime)
    }

    #[pyo3(signature = (*, title, environment, project, profile, git_branch, git_branch_base=None, size=None, storage_size=None, enable_automations=None))]
    #[allow(clippy::too_many_arguments)]
    fn create_deployment(
        &self,
        py: Python<'_>,
        title: &str,
        environment: &str,
        project: &str,
        profile: &str,
        git_branch: &str,
        git_branch_base: Option<&str>,
        size: Option<&str>,
        storage_size: Option<u32>,
        enable_automations: Option<bool>,
    ) -> PyResult<Py<PyAny>> {
        let create = models::RuntimeCreate {
            title: title.into(),
            environment: environment.into(),
            project: project.into(),
            profile_name: profile.into(),
            working_git_branch: git_branch.into(),
            base_git_branch: git_branch_base.map(String::from),
            size: size.map(String::from),
            storage_size,
            enable_automations,
            auto_snooze_timeout_minutes: None,
        };
        let runtime = py.detach(|| self.inner.create_deployment(&create)).map_err(to_py_err)?;
        to_python(py, &runtime)
    }

    #[pyo3(signature = (*, title, uuid=None, new_title=None, git_branch=None, git_branch_base=None, profile=None, size=None, storage_size=None, enable_automations=None))]
    #[allow(clippy::too_many_arguments)]
    fn update_deployment(
        &self,
        py: Python<'_>,
        title: &str,
        uuid: Option<&str>,
        new_title: Option<&str>,
        git_branch: Option<&str>,
        git_branch_base: Option<&str>,
        profile: Option<&str>,
        size: Option<&str>,
        storage_size: Option<u32>,
        enable_automations: Option<bool>,
    ) -> PyResult<Py<PyAny>> {
        let update = models::RuntimeUpdate {
            title: new_title.map(String::from), working_git_branch: git_branch.map(String::from),
            base_git_branch: git_branch_base.map(String::from), profile_name: profile.map(String::from),
            size: size.map(String::from), storage_size, enable_automations, auto_snooze_timeout_minutes: None,
        };
        let runtime = py.detach(|| self.inner.update_deployment(title, uuid, &update)).map_err(to_py_err)?;
        to_python(py, &runtime)
    }

    #[pyo3(signature = (*, title, uuid=None))]
    fn pause_deployment_automations(&self, py: Python<'_>, title: &str, uuid: Option<&str>) -> PyResult<Py<PyAny>> {
        let update = models::RuntimeUpdate {
            enable_automations: Some(false), ..Default::default()
        };
        let runtime = py.detach(|| self.inner.update_deployment(title, uuid, &update)).map_err(to_py_err)?;
        to_python(py, &runtime)
    }

    #[pyo3(signature = (*, title, uuid=None))]
    fn resume_deployment_automations(&self, py: Python<'_>, title: &str, uuid: Option<&str>) -> PyResult<Py<PyAny>> {
        let update = models::RuntimeUpdate {
            enable_automations: Some(true), ..Default::default()
        };
        let runtime = py.detach(|| self.inner.update_deployment(title, uuid, &update)).map_err(to_py_err)?;
        to_python(py, &runtime)
    }

    #[pyo3(signature = (*, title, uuid=None))]
    fn delete_deployment(&self, py: Python<'_>, title: &str, uuid: Option<&str>) -> PyResult<()> {
        py.detach(|| self.inner.delete_deployment(title, uuid))
            .map_err(to_py_err)?;
        Ok(())
    }

    // -- Runtime primitives (low-level) --

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

    #[pyo3(signature = (*, title, kind))]
    fn resolve_runtime_by_title(
        &self,
        py: Python<'_>,
        title: &str,
        kind: &str,
    ) -> PyResult<Py<PyAny>> {
        let runtime = py
            .detach(|| self.inner.resolve_runtime_by_title(title, kind))
            .map_err(to_py_err)?;
        to_python(py, &runtime)
    }

    #[pyo3(signature = (*, title, kind, environment, project, profile_name, working_git_branch, base_git_branch=None, size=None, storage_size=None, enable_automations=None, auto_snooze_timeout_minutes=None))]
    #[allow(clippy::too_many_arguments)]
    fn create_runtime(
        &self,
        py: Python<'_>,
        title: &str,
        kind: &str,
        environment: &str,
        project: &str,
        profile_name: &str,
        working_git_branch: &str,
        base_git_branch: Option<&str>,
        size: Option<&str>,
        storage_size: Option<u32>,
        enable_automations: Option<bool>,
        auto_snooze_timeout_minutes: Option<u32>,
    ) -> PyResult<Py<PyAny>> {
        let create = models::RuntimeCreate {
            title: title.to_string(),
            environment: environment.to_string(),
            project: project.to_string(),
            profile_name: profile_name.to_string(),
            working_git_branch: working_git_branch.to_string(),
            base_git_branch: base_git_branch.map(String::from),
            size: size.map(String::from),
            storage_size,
            enable_automations,
            auto_snooze_timeout_minutes,
        };
        let runtime = py
            .detach(|| match kind {
                "workspace" => self.inner.create_workspace(&create),
                "deployment" => self.inner.create_deployment(&create),
                _ => Err(ascend_tools::Error::MissingField {
                    context: "create_runtime",
                    field: "kind must be 'workspace' or 'deployment'",
                }),
            })
            .map_err(to_py_err)?;
        to_python(py, &runtime)
    }

    #[pyo3(signature = (*, uuid, title=None, working_git_branch=None, base_git_branch=None, profile_name=None, size=None, storage_size=None, enable_automations=None, auto_snooze_timeout_minutes=None))]
    fn update_runtime(
        &self,
        py: Python<'_>,
        uuid: &str,
        title: Option<&str>,
        working_git_branch: Option<&str>,
        base_git_branch: Option<&str>,
        profile_name: Option<&str>,
        size: Option<&str>,
        storage_size: Option<u32>,
        enable_automations: Option<bool>,
        auto_snooze_timeout_minutes: Option<u32>,
    ) -> PyResult<Py<PyAny>> {
        let update = models::RuntimeUpdate {
            title: title.map(String::from),
            working_git_branch: working_git_branch.map(String::from),
            base_git_branch: base_git_branch.map(String::from),
            profile_name: profile_name.map(String::from),
            size: size.map(String::from),
            storage_size,
            enable_automations,
            auto_snooze_timeout_minutes,
        };
        let runtime = py
            .detach(|| self.inner.update_runtime(uuid, &update))
            .map_err(to_py_err)?;
        to_python(py, &runtime)
    }

    #[pyo3(signature = (*, uuid))]
    fn delete_runtime(&self, py: Python<'_>, uuid: &str) -> PyResult<()> {
        py.detach(|| self.inner.delete_runtime(uuid))
            .map_err(to_py_err)?;
        Ok(())
    }

    #[pyo3(signature = ())]
    fn list_environments(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        let environments = py
            .detach(|| self.inner.list_environments())
            .map_err(to_py_err)?;
        to_python(py, &environments)
    }

    #[pyo3(signature = (*, title))]
    fn get_environment(&self, py: Python<'_>, title: &str) -> PyResult<Py<PyAny>> {
        let env = py
            .detach(|| self.inner.get_environment(title))
            .map_err(to_py_err)?;
        to_python(py, &env)
    }

    #[pyo3(signature = ())]
    fn list_projects(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        let projects = py
            .detach(|| self.inner.list_projects())
            .map_err(to_py_err)?;
        to_python(py, &projects)
    }

    #[pyo3(signature = (*, title))]
    fn get_project(&self, py: Python<'_>, title: &str) -> PyResult<Py<PyAny>> {
        let proj = py
            .detach(|| self.inner.get_project(title))
            .map_err(to_py_err)?;
        to_python(py, &proj)
    }

    #[pyo3(signature = (*, runtime_uuid=None, project=None, branch=None))]
    fn list_profiles(
        &self,
        py: Python<'_>,
        runtime_uuid: Option<&str>,
        project: Option<&str>,
        branch: Option<&str>,
    ) -> PyResult<Py<PyAny>> {
        let profiles = py
            .detach(|| self.inner.list_profiles(runtime_uuid, project, branch))
            .map_err(to_py_err)?;
        to_python(py, &profiles)
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
        let result = py
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
        to_python(py, &result)
    }

    #[pyo3(signature = (*, runtime_uuid, name))]
    fn get_flow_run(&self, py: Python<'_>, runtime_uuid: &str, name: &str) -> PyResult<Py<PyAny>> {
        let run = py
            .detach(|| self.inner.get_flow_run(runtime_uuid, name))
            .map_err(to_py_err)?;
        to_python(py, &run)
    }

    #[pyo3(signature = ())]
    fn list_otto_providers(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        let providers = py
            .detach(|| self.inner.list_otto_providers())
            .map_err(to_py_err)?;
        to_python(py, &providers)
    }

    #[pyo3(signature = (*, prompt, runtime_uuid=None, thread_id=None, model=None, provider=None))]
    fn otto_chat(
        &self,
        py: Python<'_>,
        prompt: &str,
        runtime_uuid: Option<&str>,
        thread_id: Option<&str>,
        model: Option<&str>,
        provider: Option<&str>,
    ) -> PyResult<Py<PyAny>> {
        let otto_model = models::OttoModel::from_options(provider, model);
        let request = models::OttoChatRequest {
            prompt: prompt.to_string(),
            runtime_id: runtime_uuid.map(String::from),
            thread_id: thread_id.map(String::from),
            model: otto_model,
        };
        let response = py
            .detach(|| self.inner.otto_chat(&request))
            .map_err(to_py_err)?;
        let result = serde_json::json!({
            "message": response.message,
            "thread_id": response.thread_id,
        });
        to_python(py, &result)
    }
}

#[pyfunction]
fn run(py: Python<'_>, argv: Vec<String>) -> PyResult<()> {
    py.detach(|| {
        ascend_tools_cli::run(argv.iter().map(|s| s.as_str()))
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))
    })
}

/// Start the MCP HTTP server. Blocks until the server is shut down.
///
/// Call from a background thread (e.g. `asyncio.to_thread(run_mcp_http, "127.0.0.1:4201")`)
/// since it blocks the calling thread.
#[pyfunction]
#[pyo3(signature = (bind_addr, *, service_account_id=None, service_account_key=None, instance_api_url=None))]
fn run_mcp_http(
    py: Python<'_>,
    bind_addr: String,
    service_account_id: Option<&str>,
    service_account_key: Option<&str>,
    instance_api_url: Option<&str>,
) -> PyResult<()> {
    let config = Config::with_overrides(service_account_id, service_account_key, instance_api_url);
    py.detach(move || {
        let rt = tokio::runtime::Runtime::new()
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))?;
        rt.block_on(ascend_tools_mcp::run_http(config, &bind_addr))
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))
    })
}

#[pymodule]
fn core(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<Client>()?;
    m.add_function(wrap_pyfunction!(run, m)?)?;
    m.add_function(wrap_pyfunction!(run_mcp_http, m)?)?;
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
