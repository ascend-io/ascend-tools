#![deny(unsafe_code)]

use std::sync::Arc;

use ascend_tools::client::AscendClient;
use ascend_tools::config::Config;
use ascend_tools::models;
use napi::bindgen_prelude::*;
use napi::threadsafe_function::{
    ThreadsafeFunction, ThreadsafeFunctionCallMode, UnknownReturnValue,
};
use napi::Task;
use napi_derive::napi;

// --- Generic task for libuv thread pool ---

pub struct TypedTask<T>(Option<Box<dyn FnOnce() -> napi::Result<T> + Send>>);

impl<T> TypedTask<T> {
    fn new(f: impl FnOnce() -> napi::Result<T> + Send + 'static) -> Self {
        Self(Some(Box::new(f)))
    }
}

impl<T: ToNapiValue + TypeName + Send + 'static> Task for TypedTask<T> {
    type Output = T;
    type JsValue = T;

    fn compute(&mut self) -> napi::Result<Self::Output> {
        let f = self
            .0
            .take()
            .ok_or_else(|| napi::Error::from_reason("task already consumed"))?;
        f()
    }

    fn resolve(&mut self, _env: Env, output: Self::Output) -> napi::Result<Self::JsValue> {
        Ok(output)
    }
}

// --- Client ---

#[napi]
pub struct Client {
    inner: Arc<AscendClient>,
}

#[napi]
impl Client {
    #[napi(constructor)]
    pub fn new(
        service_account_id: Option<String>,
        service_account_key: Option<String>,
        instance_api_url: Option<String>,
    ) -> napi::Result<Self> {
        let config = Config::with_overrides(
            service_account_id.as_deref(),
            service_account_key.as_deref(),
            instance_api_url.as_deref(),
        )
        .map_err(to_napi_err)?;
        let inner = AscendClient::new(config).map_err(to_napi_err)?;
        Ok(Self {
            inner: Arc::new(inner),
        })
    }

    // -- Workspace methods --

    #[napi(ts_return_type = "Promise<Runtime[]>")]
    pub fn list_workspaces(
        &self,
        title: Option<String>,
        project: Option<String>,
        environment: Option<String>,
    ) -> AsyncTask<TypedTask<Vec<models::Runtime>>> {
        let client = self.inner.clone();
        AsyncTask::new(TypedTask::new(move || {
            let mut filters = models::RuntimeFilters::default();
            filters.title = title;
            filters.project = project;
            filters.environment = environment;
            let result = client.list_workspaces(filters).map_err(to_napi_err)?;
            Ok(result.into_iter().map(|w| w.0).collect())
        }))
    }

    #[napi(ts_return_type = "Promise<Runtime>")]
    pub fn get_workspace(&self, title: String, uuid: Option<String>) -> AsyncTask<TypedTask<models::Runtime>> {
        let client = self.inner.clone();
        AsyncTask::new(TypedTask::new(move || {
            Ok(client.get_workspace(&title, uuid.as_deref()).map_err(to_napi_err)?.0)
        }))
    }

    #[napi(ts_return_type = "Promise<Runtime>")]
    pub fn pause_workspace(&self, title: String, uuid: Option<String>) -> AsyncTask<TypedTask<models::Runtime>> {
        let client = self.inner.clone();
        AsyncTask::new(TypedTask::new(move || {
            Ok(client.pause_workspace(&title, uuid.as_deref()).map_err(to_napi_err)?.0)
        }))
    }

    #[napi(ts_return_type = "Promise<Runtime>")]
    pub fn resume_workspace(&self, title: String, uuid: Option<String>) -> AsyncTask<TypedTask<models::Runtime>> {
        let client = self.inner.clone();
        AsyncTask::new(TypedTask::new(move || {
            Ok(client.resume_workspace(&title, uuid.as_deref()).map_err(to_napi_err)?.0)
        }))
    }

    #[napi(ts_return_type = "Promise<Runtime>")]
    #[allow(clippy::too_many_arguments)]
    pub fn create_workspace(&self, title: String, environment: String, project: String, profile: String, git_branch: String, git_branch_base: Option<String>, size: Option<String>, storage_size: Option<u32>, auto_snooze_timeout_minutes: Option<u32>) -> AsyncTask<TypedTask<models::Runtime>> {
        let client = self.inner.clone();
        AsyncTask::new(TypedTask::new(move || {
            let mut create = models::RuntimeCreate::new(&title, &environment, &project, &profile, &git_branch);
            create.git_branch_base = git_branch_base;
            create.size = size;
            create.storage_size = storage_size;
            create.auto_snooze_timeout_minutes = auto_snooze_timeout_minutes;
            Ok(client.create_workspace(&create).map_err(to_napi_err)?.0)
        }))
    }

    #[napi(ts_return_type = "Promise<Runtime>")]
    #[allow(clippy::too_many_arguments)]
    pub fn update_workspace(&self, title: String, uuid: Option<String>, new_title: Option<String>, git_branch: Option<String>, git_branch_base: Option<String>, profile: Option<String>, size: Option<String>, storage_size: Option<u32>, auto_snooze_timeout_minutes: Option<u32>) -> AsyncTask<TypedTask<models::Runtime>> {
        let client = self.inner.clone();
        AsyncTask::new(TypedTask::new(move || {
            let mut update = models::RuntimeUpdate::default();
            update.title = new_title;
            update.git_branch = git_branch;
            update.git_branch_base = git_branch_base;
            update.profile = profile;
            update.size = size;
            update.storage_size = storage_size;
            update.auto_snooze_timeout_minutes = auto_snooze_timeout_minutes;
            Ok(client.update_workspace(&title, uuid.as_deref(), &update).map_err(to_napi_err)?.0)
        }))
    }

    #[napi(ts_return_type = "Promise<void>")]
    pub fn delete_workspace(&self, title: String, uuid: Option<String>) -> AsyncTask<TypedTask<()>> {
        let client = self.inner.clone();
        AsyncTask::new(TypedTask::new(move || client.delete_workspace(&title, uuid.as_deref()).map_err(to_napi_err)))
    }

    // -- Deployment methods --

    #[napi(ts_return_type = "Promise<Runtime[]>")]
    pub fn list_deployments(&self, title: Option<String>, project: Option<String>, environment: Option<String>) -> AsyncTask<TypedTask<Vec<models::Runtime>>> {
        let client = self.inner.clone();
        AsyncTask::new(TypedTask::new(move || {
            let mut filters = models::RuntimeFilters::default();
            filters.title = title;
            filters.project = project;
            filters.environment = environment;
            let result = client.list_deployments(filters).map_err(to_napi_err)?;
            Ok(result.into_iter().map(|d| d.0).collect())
        }))
    }

    #[napi(ts_return_type = "Promise<Runtime>")]
    pub fn get_deployment(&self, title: String, uuid: Option<String>) -> AsyncTask<TypedTask<models::Runtime>> {
        let client = self.inner.clone();
        AsyncTask::new(TypedTask::new(move || Ok(client.get_deployment(&title, uuid.as_deref()).map_err(to_napi_err)?.0)))
    }

    #[napi(ts_return_type = "Promise<Runtime>")]
    #[allow(clippy::too_many_arguments)]
    pub fn create_deployment(&self, title: String, environment: String, project: String, profile: String, git_branch: String, git_branch_base: Option<String>, size: Option<String>, storage_size: Option<u32>, enable_automations: Option<bool>) -> AsyncTask<TypedTask<models::Runtime>> {
        let client = self.inner.clone();
        AsyncTask::new(TypedTask::new(move || {
            let mut create = models::RuntimeCreate::new(&title, &environment, &project, &profile, &git_branch);
            create.git_branch_base = git_branch_base;
            create.size = size;
            create.storage_size = storage_size;
            create.enable_automations = enable_automations;
            Ok(client.create_deployment(&create).map_err(to_napi_err)?.0)
        }))
    }

    #[napi(ts_return_type = "Promise<Runtime>")]
    #[allow(clippy::too_many_arguments)]
    pub fn update_deployment(&self, title: String, uuid: Option<String>, new_title: Option<String>, git_branch: Option<String>, git_branch_base: Option<String>, profile: Option<String>, size: Option<String>, storage_size: Option<u32>, enable_automations: Option<bool>) -> AsyncTask<TypedTask<models::Runtime>> {
        let client = self.inner.clone();
        AsyncTask::new(TypedTask::new(move || {
            let mut update = models::RuntimeUpdate::default();
            update.title = new_title;
            update.git_branch = git_branch;
            update.git_branch_base = git_branch_base;
            update.profile = profile;
            update.size = size;
            update.storage_size = storage_size;
            update.enable_automations = enable_automations;
            Ok(client.update_deployment(&title, uuid.as_deref(), &update).map_err(to_napi_err)?.0)
        }))
    }

    #[napi(ts_return_type = "Promise<Runtime>")]
    pub fn pause_deployment_automations(&self, title: String, uuid: Option<String>) -> AsyncTask<TypedTask<models::Runtime>> {
        let client = self.inner.clone();
        AsyncTask::new(TypedTask::new(move || Ok(client.pause_deployment_automations(&title, uuid.as_deref()).map_err(to_napi_err)?.0)))
    }

    #[napi(ts_return_type = "Promise<Runtime>")]
    pub fn resume_deployment_automations(&self, title: String, uuid: Option<String>) -> AsyncTask<TypedTask<models::Runtime>> {
        let client = self.inner.clone();
        AsyncTask::new(TypedTask::new(move || Ok(client.resume_deployment_automations(&title, uuid.as_deref()).map_err(to_napi_err)?.0)))
    }

    #[napi(ts_return_type = "Promise<void>")]
    pub fn delete_deployment(&self, title: String, uuid: Option<String>) -> AsyncTask<TypedTask<()>> {
        let client = self.inner.clone();
        AsyncTask::new(TypedTask::new(move || client.delete_deployment(&title, uuid.as_deref()).map_err(to_napi_err)))
    }

    // -- Environment methods --

    #[napi(ts_return_type = "Promise<Environment[]>")]
    pub fn list_environments(&self) -> AsyncTask<TypedTask<Vec<models::Environment>>> {
        let client = self.inner.clone();
        AsyncTask::new(TypedTask::new(move || client.list_environments().map_err(to_napi_err)))
    }

    #[napi(ts_return_type = "Promise<Environment>")]
    pub fn get_environment(&self, title: String) -> AsyncTask<TypedTask<models::Environment>> {
        let client = self.inner.clone();
        AsyncTask::new(TypedTask::new(move || client.get_environment(&title).map_err(to_napi_err)))
    }

    // -- Project methods --

    #[napi(ts_return_type = "Promise<Project[]>")]
    pub fn list_projects(&self) -> AsyncTask<TypedTask<Vec<models::Project>>> {
        let client = self.inner.clone();
        AsyncTask::new(TypedTask::new(move || client.list_projects().map_err(to_napi_err)))
    }

    #[napi(ts_return_type = "Promise<Project>")]
    pub fn get_project(&self, title: String) -> AsyncTask<TypedTask<models::Project>> {
        let client = self.inner.clone();
        AsyncTask::new(TypedTask::new(move || client.get_project(&title).map_err(to_napi_err)))
    }

    // -- Profile methods --

    #[napi(ts_return_type = "Promise<string[]>")]
    pub fn list_profiles(&self, workspace: Option<String>, deployment: Option<String>, uuid: Option<String>, project: Option<String>, branch: Option<String>) -> AsyncTask<TypedTask<Vec<String>>> {
        let client = self.inner.clone();
        AsyncTask::new(TypedTask::new(move || {
            let runtime_uuid = client.resolve_optional_runtime_target(workspace.as_deref(), deployment.as_deref(), uuid.as_deref()).map_err(to_napi_err)?;
            client.list_profiles(runtime_uuid.as_deref(), project.as_deref(), branch.as_deref()).map_err(to_napi_err)
        }))
    }

    // -- Flow methods --

    #[napi(ts_return_type = "Promise<Flow[]>")]
    pub fn list_flows(&self, workspace: Option<String>, deployment: Option<String>, uuid: Option<String>) -> AsyncTask<TypedTask<Vec<models::Flow>>> {
        let client = self.inner.clone();
        AsyncTask::new(TypedTask::new(move || {
            let runtime_uuid = client.resolve_runtime_target(workspace.as_deref(), deployment.as_deref(), uuid.as_deref()).map_err(to_napi_err)?;
            client.list_flows(&runtime_uuid).map_err(to_napi_err)
        }))
    }

    #[napi(ts_return_type = "Promise<FlowRunTrigger>")]
    #[allow(clippy::too_many_arguments)]
    pub fn run_flow(&self, flow: String, workspace: Option<String>, deployment: Option<String>, uuid: Option<String>, spec: Option<serde_json::Value>, resume: Option<bool>) -> AsyncTask<TypedTask<models::FlowRunTrigger>> {
        let client = self.inner.clone();
        AsyncTask::new(TypedTask::new(move || {
            let runtime_uuid = client.resolve_runtime_target(workspace.as_deref(), deployment.as_deref(), uuid.as_deref()).map_err(to_napi_err)?;
            client.run_flow(&runtime_uuid, &flow, spec, resume.unwrap_or(false)).map_err(to_napi_err)
        }))
    }

    // -- Flow run methods --

    #[napi(ts_return_type = "Promise<FlowRunList>")]
    #[allow(clippy::too_many_arguments)]
    pub fn list_flow_runs(&self, workspace: Option<String>, deployment: Option<String>, uuid: Option<String>, status: Option<String>, flow: Option<String>, since: Option<String>, until: Option<String>, offset: Option<u32>, limit: Option<u32>) -> AsyncTask<TypedTask<models::FlowRunList>> {
        let client = self.inner.clone();
        AsyncTask::new(TypedTask::new(move || {
            let runtime_uuid = client.resolve_runtime_target(workspace.as_deref(), deployment.as_deref(), uuid.as_deref()).map_err(to_napi_err)?;
            let mut filters = models::FlowRunFilters::default();
            filters.status = status;
            filters.flow = flow;
            filters.since = since;
            filters.until = until;
            filters.offset = offset.map(u64::from);
            filters.limit = limit.map(u64::from);
            client.list_flow_runs(&runtime_uuid, filters).map_err(to_napi_err)
        }))
    }

    #[napi(ts_return_type = "Promise<FlowRun>")]
    pub fn get_flow_run(&self, name: String, workspace: Option<String>, deployment: Option<String>, uuid: Option<String>) -> AsyncTask<TypedTask<models::FlowRun>> {
        let client = self.inner.clone();
        AsyncTask::new(TypedTask::new(move || {
            let runtime_uuid = client.resolve_runtime_target(workspace.as_deref(), deployment.as_deref(), uuid.as_deref()).map_err(to_napi_err)?;
            client.get_flow_run(&runtime_uuid, &name).map_err(to_napi_err)
        }))
    }

    // -- Otto methods --

    #[napi(ts_return_type = "Promise<OttoProvider[]>")]
    pub fn list_otto_providers(&self) -> AsyncTask<TypedTask<Vec<models::OttoProvider>>> {
        let client = self.inner.clone();
        AsyncTask::new(TypedTask::new(move || client.list_otto_providers().map_err(to_napi_err)))
    }

    #[napi(ts_return_type = "Promise<OttoChatResponse>")]
    #[allow(clippy::too_many_arguments)]
    pub fn otto(&self, prompt: String, workspace: Option<String>, deployment: Option<String>, uuid: Option<String>, thread_id: Option<String>, model: Option<String>, provider: Option<String>) -> AsyncTask<TypedTask<models::OttoChatResponse>> {
        let client = self.inner.clone();
        AsyncTask::new(TypedTask::new(move || {
            let otto_model = models::OttoModel::from_options(provider.as_deref(), model.as_deref());
            let runtime_uuid = client.resolve_optional_runtime_target(workspace.as_deref(), deployment.as_deref(), uuid.as_deref()).map_err(to_napi_err)?;
            let request = models::OttoChatRequest { prompt, runtime_uuid, thread_id, model: otto_model };
            client.otto(&request).map_err(to_napi_err)
        }))
    }

    #[napi(ts_return_type = "Promise<OttoChatResponse>")]
    #[allow(clippy::too_many_arguments)]
    pub fn otto_streaming(&self, prompt: String, on_delta: ThreadsafeFunction<String, UnknownReturnValue>, workspace: Option<String>, deployment: Option<String>, uuid: Option<String>, thread_id: Option<String>, model: Option<String>, provider: Option<String>) -> AsyncTask<TypedTask<models::OttoChatResponse>> {
        let client = self.inner.clone();
        AsyncTask::new(TypedTask::new(move || {
            let otto_model = models::OttoModel::from_options(provider.as_deref(), model.as_deref());
            let runtime_uuid = client.resolve_optional_runtime_target(workspace.as_deref(), deployment.as_deref(), uuid.as_deref()).map_err(to_napi_err)?;
            let request = models::OttoChatRequest { prompt, runtime_uuid, thread_id, model: otto_model };
            client.otto_streaming(&request, |event| { if let models::StreamEvent::TextDelta(delta) = event { on_delta.call(Ok(delta), ThreadsafeFunctionCallMode::NonBlocking); } std::ops::ControlFlow::Continue(()) }, |_| {}).map_err(to_napi_err)
        }))
    }
}

fn to_napi_err(e: impl std::fmt::Display) -> napi::Error {
    napi::Error::from_reason(e.to_string())
}
