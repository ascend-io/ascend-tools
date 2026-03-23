#![forbid(unsafe_code)]

use ascend_tools::Error;
use ascend_tools::client::AscendClient;
use ascend_tools::config::Config;
use ascend_tools::models;
use pyo3::prelude::*;

#[pyclass(module = "ascend_tools.core")]
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
                .map_err(to_py_err)?;

        let inner = AscendClient::new(config).map_err(to_py_err)?;

        Ok(Self { inner })
    }

    fn __repr__(&self) -> String {
        format!(
            "Client(instance_api_url='{}', service_account_id='{}')",
            self.inner.instance_api_url(),
            self.inner.service_account_id(),
        )
    }

    // -- Workspace methods --

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
    fn pause_workspace(
        &self,
        py: Python<'_>,
        title: &str,
        uuid: Option<&str>,
    ) -> PyResult<Py<PyAny>> {
        let runtime = py
            .detach(|| self.inner.pause_workspace(title, uuid))
            .map_err(to_py_err)?;
        to_python(py, &runtime)
    }

    #[pyo3(signature = (*, title, uuid=None))]
    fn resume_workspace(
        &self,
        py: Python<'_>,
        title: &str,
        uuid: Option<&str>,
    ) -> PyResult<Py<PyAny>> {
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
        let mut create =
            models::RuntimeCreate::new(title, environment, project, profile, git_branch);
        create.git_branch_base = git_branch_base.map(String::from);
        create.size = size.map(String::from);
        create.storage_size = storage_size;
        create.auto_snooze_timeout_minutes = auto_snooze_timeout_minutes;
        let runtime = py
            .detach(|| self.inner.create_workspace(&create))
            .map_err(to_py_err)?;
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
        let mut update = models::RuntimeUpdate::default();
        update.title = new_title.map(String::from);
        update.git_branch = git_branch.map(String::from);
        update.git_branch_base = git_branch_base.map(String::from);
        update.profile = profile.map(String::from);
        update.size = size.map(String::from);
        update.storage_size = storage_size;
        update.auto_snooze_timeout_minutes = auto_snooze_timeout_minutes;
        let runtime = py
            .detach(|| self.inner.update_workspace(title, uuid, &update))
            .map_err(to_py_err)?;
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
        let mut create =
            models::RuntimeCreate::new(title, environment, project, profile, git_branch);
        create.git_branch_base = git_branch_base.map(String::from);
        create.size = size.map(String::from);
        create.storage_size = storage_size;
        create.enable_automations = enable_automations;
        let runtime = py
            .detach(|| self.inner.create_deployment(&create))
            .map_err(to_py_err)?;
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
        let mut update = models::RuntimeUpdate::default();
        update.title = new_title.map(String::from);
        update.git_branch = git_branch.map(String::from);
        update.git_branch_base = git_branch_base.map(String::from);
        update.profile = profile.map(String::from);
        update.size = size.map(String::from);
        update.storage_size = storage_size;
        update.enable_automations = enable_automations;
        let runtime = py
            .detach(|| self.inner.update_deployment(title, uuid, &update))
            .map_err(to_py_err)?;
        to_python(py, &runtime)
    }

    #[pyo3(signature = (*, title, uuid=None))]
    fn pause_deployment_automations(
        &self,
        py: Python<'_>,
        title: &str,
        uuid: Option<&str>,
    ) -> PyResult<Py<PyAny>> {
        let runtime = py
            .detach(|| self.inner.pause_deployment_automations(title, uuid))
            .map_err(to_py_err)?;
        to_python(py, &runtime)
    }

    #[pyo3(signature = (*, title, uuid=None))]
    fn resume_deployment_automations(
        &self,
        py: Python<'_>,
        title: &str,
        uuid: Option<&str>,
    ) -> PyResult<Py<PyAny>> {
        let runtime = py
            .detach(|| self.inner.resume_deployment_automations(title, uuid))
            .map_err(to_py_err)?;
        to_python(py, &runtime)
    }

    #[pyo3(signature = (*, title, uuid=None))]
    fn delete_deployment(&self, py: Python<'_>, title: &str, uuid: Option<&str>) -> PyResult<()> {
        py.detach(|| self.inner.delete_deployment(title, uuid))
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

    #[pyo3(signature = (*, workspace=None, deployment=None, uuid=None, project=None, branch=None))]
    fn list_profiles(
        &self,
        py: Python<'_>,
        workspace: Option<&str>,
        deployment: Option<&str>,
        uuid: Option<&str>,
        project: Option<&str>,
        branch: Option<&str>,
    ) -> PyResult<Py<PyAny>> {
        let profiles = py
            .detach(|| {
                let runtime_uuid = self
                    .inner
                    .resolve_optional_runtime_target(workspace, deployment, uuid)?;
                if project.is_some() && branch.is_none() {
                    return Err(ascend_tools::Error::MissingField {
                        context: "list_profiles",
                        field: "branch (required with project)",
                    });
                }
                if runtime_uuid.is_none() && project.is_none() {
                    return Err(ascend_tools::Error::MissingField {
                        context: "list_profiles",
                        field: "workspace, deployment, uuid, or project",
                    });
                }
                self.inner
                    .list_profiles(runtime_uuid.as_deref(), project, branch)
            })
            .map_err(to_py_err)?;
        to_python(py, &profiles)
    }

    #[pyo3(signature = (*, workspace=None, deployment=None, uuid=None))]
    fn list_flows(
        &self,
        py: Python<'_>,
        workspace: Option<&str>,
        deployment: Option<&str>,
        uuid: Option<&str>,
    ) -> PyResult<Py<PyAny>> {
        let flows = py
            .detach(|| {
                let runtime_uuid = self
                    .inner
                    .resolve_runtime_target(workspace, deployment, uuid)?;
                self.inner.list_flows(&runtime_uuid)
            })
            .map_err(to_py_err)?;
        to_python(py, &flows)
    }

    #[pyo3(signature = (*, flow, workspace=None, deployment=None, uuid=None, spec=None, resume=false))]
    #[allow(clippy::too_many_arguments)]
    fn run_flow(
        &self,
        py: Python<'_>,
        flow: &str,
        workspace: Option<&str>,
        deployment: Option<&str>,
        uuid: Option<&str>,
        spec: Option<&Bound<'_, PyAny>>,
        resume: bool,
    ) -> PyResult<Py<PyAny>> {
        let spec_value: Option<serde_json::Value> = match spec {
            Some(obj) => Some(pythonize::depythonize(obj)?),
            None => None,
        };
        let trigger = py
            .detach(|| {
                let runtime_uuid = self
                    .inner
                    .resolve_runtime_target(workspace, deployment, uuid)?;
                self.inner
                    .run_flow(&runtime_uuid, flow, spec_value, resume)
            })
            .map_err(to_py_err)?;
        to_python(py, &trigger)
    }

    #[pyo3(signature = (*, workspace=None, deployment=None, uuid=None, status=None, flow=None, since=None, until=None, offset=None, limit=None))]
    #[allow(clippy::too_many_arguments)]
    fn list_flow_runs(
        &self,
        py: Python<'_>,
        workspace: Option<&str>,
        deployment: Option<&str>,
        uuid: Option<&str>,
        status: Option<&str>,
        flow: Option<&str>,
        since: Option<&str>,
        until: Option<&str>,
        offset: Option<u64>,
        limit: Option<u64>,
    ) -> PyResult<Py<PyAny>> {
        let result = py
            .detach(|| {
                let runtime_uuid = self
                    .inner
                    .resolve_runtime_target(workspace, deployment, uuid)?;
                let mut filters = models::FlowRunFilters::default();
                filters.status = status.map(String::from);
                filters.flow = flow.map(String::from);
                filters.since = since.map(String::from);
                filters.until = until.map(String::from);
                filters.offset = offset;
                filters.limit = limit;
                self.inner.list_flow_runs(&runtime_uuid, filters)
            })
            .map_err(to_py_err)?;
        to_python(py, &result)
    }

    #[pyo3(signature = (*, name, workspace=None, deployment=None, uuid=None))]
    fn get_flow_run(
        &self,
        py: Python<'_>,
        name: &str,
        workspace: Option<&str>,
        deployment: Option<&str>,
        uuid: Option<&str>,
    ) -> PyResult<Py<PyAny>> {
        let run = py
            .detach(|| {
                let runtime_uuid = self
                    .inner
                    .resolve_runtime_target(workspace, deployment, uuid)?;
                self.inner.get_flow_run(&runtime_uuid, name)
            })
            .map_err(to_py_err)?;
        to_python(py, &run)
    }

    // -- Secret methods --

    #[pyo3(signature = (*, environment=None))]
    fn list_secrets(
        &self,
        py: Python<'_>,
        environment: Option<&str>,
    ) -> PyResult<Py<PyAny>> {
        let secrets = py
            .detach(|| self.inner.list_secrets(environment))
            .map_err(to_py_err)?;
        to_python(py, &secrets)
    }

    #[pyo3(signature = (*, name, environment=None))]
    fn get_secret(
        &self,
        py: Python<'_>,
        name: &str,
        environment: Option<&str>,
    ) -> PyResult<Py<PyAny>> {
        let secret = py
            .detach(|| self.inner.get_secret(name, environment))
            .map_err(to_py_err)?;
        to_python(py, &secret)
    }

    #[pyo3(signature = (*, name, value=None, generate_ssh_key=false, algorithm=None, format=None, environment=None))]
    #[allow(clippy::too_many_arguments)]
    fn set_secret(
        &self,
        py: Python<'_>,
        name: &str,
        value: Option<&str>,
        generate_ssh_key: bool,
        algorithm: Option<&str>,
        format: Option<&str>,
        environment: Option<&str>,
    ) -> PyResult<Py<PyAny>> {
        let secret_value = if generate_ssh_key {
            models::SecretValue::GenerateSshKey {
                algorithm: algorithm.map(String::from),
                format: format.map(String::from),
            }
        } else if let Some(v) = value {
            models::SecretValue::Value(v.to_string())
        } else {
            return Err(pyo3::exceptions::PyValueError::new_err(
                "either value or generate_ssh_key=True is required",
            ));
        };
        let result = py
            .detach(|| self.inner.set_secret(name, &secret_value, environment))
            .map_err(to_py_err)?;
        to_python(py, &result)
    }

    #[pyo3(signature = (*, name, environment=None))]
    fn delete_secret(
        &self,
        py: Python<'_>,
        name: &str,
        environment: Option<&str>,
    ) -> PyResult<Py<PyAny>> {
        let result = py
            .detach(|| self.inner.delete_secret(name, environment))
            .map_err(to_py_err)?;
        to_python(py, &result)
    }

    #[pyo3(signature = (*, name))]
    fn get_secret_ssh_public_key(
        &self,
        py: Python<'_>,
        name: &str,
    ) -> PyResult<Py<PyAny>> {
        let key = py
            .detach(|| self.inner.get_secret_ssh_public_key(name))
            .map_err(to_py_err)?;
        to_python(py, &key)
    }

    // -- Otto methods --

    #[pyo3(signature = ())]
    fn list_otto_providers(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        let providers = py
            .detach(|| self.inner.list_otto_providers())
            .map_err(to_py_err)?;
        to_python(py, &providers)
    }

    #[pyo3(signature = (*, prompt, workspace=None, deployment=None, uuid=None, thread_id=None, model=None, provider=None))]
    #[allow(clippy::too_many_arguments)]
    fn otto(
        &self,
        py: Python<'_>,
        prompt: &str,
        workspace: Option<&str>,
        deployment: Option<&str>,
        uuid: Option<&str>,
        thread_id: Option<&str>,
        model: Option<&str>,
        provider: Option<&str>,
    ) -> PyResult<Py<PyAny>> {
        let response = py
            .detach(|| {
                let runtime_uuid = self
                    .inner
                    .resolve_optional_runtime_target(workspace, deployment, uuid)?;
                let otto_model = self.inner.resolve_otto_model(provider, model)?;
                let request = models::OttoChatRequest {
                    prompt: prompt.to_string(),
                    runtime_uuid,
                    thread_id: thread_id.map(String::from),
                    model: otto_model,
                };
                self.inner.otto(&request)
            })
            .map_err(to_py_err)?;
        to_python(py, &response)
    }
}

#[pyfunction]
fn run_cli(py: Python<'_>, argv: Vec<String>) -> PyResult<()> {
    py.detach(|| {
        ascend_tools_cli::run_cli(argv.iter().map(|s| s.as_str()))
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

const VERSION: &str = env!("CARGO_PKG_VERSION");

#[pymodule]
fn core(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add("__version__", VERSION)?;
    m.add_class::<Client>()?;
    m.add_function(wrap_pyfunction!(run_cli, m)?)?;
    m.add_function(wrap_pyfunction!(run_mcp_http, m)?)?;
    Ok(())
}

fn to_python(py: Python<'_>, value: &impl serde::Serialize) -> PyResult<Py<PyAny>> {
    pythonize::pythonize(py, value)
        .map(Bound::unbind)
        .map_err(to_py_err)
}

fn to_py_err(e: impl std::fmt::Display + AsAscendError) -> PyErr {
    e.to_py_err()
}

/// Maps `ascend_tools::Error` variants to appropriate Python exception types.
trait AsAscendError: std::fmt::Display {
    fn to_py_err(&self) -> PyErr {
        PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(self.to_string())
    }
}

impl AsAscendError for Error {
    fn to_py_err(&self) -> PyErr {
        let msg = self.to_string();
        match self {
            // Config / validation errors → ValueError
            Error::MissingConfig { .. }
            | Error::InvalidServiceAccountKeyEncoding
            | Error::InvalidServiceAccountKeyLength { .. }
            | Error::InvalidEd25519SeedLength { .. }
            | Error::AmbiguousTitle { .. } => {
                PyErr::new::<pyo3::exceptions::PyValueError, _>(msg)
            }

            // Not found → LookupError
            Error::NotFound { .. } => PyErr::new::<pyo3::exceptions::PyLookupError, _>(msg),

            // Auth errors → PermissionError
            Error::ApiError { status, .. } if *status == 401 || *status == 403 => {
                PyErr::new::<pyo3::exceptions::PyPermissionError, _>(msg)
            }

            // HTTP 404 → LookupError
            Error::ApiError { status, .. } if *status == 404 => {
                PyErr::new::<pyo3::exceptions::PyLookupError, _>(msg)
            }

            // Network / request failures → ConnectionError
            Error::RequestFailed { .. } | Error::ResponseReadFailed { .. } => {
                PyErr::new::<pyo3::exceptions::PyConnectionError, _>(msg)
            }

            // Everything else → RuntimeError
            _ => PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(msg),
        }
    }
}

// Fallback for non-Error types (pythonize, serde, etc.)
impl AsAscendError for pythonize::PythonizeError {}
impl AsAscendError for serde_json::Error {}
