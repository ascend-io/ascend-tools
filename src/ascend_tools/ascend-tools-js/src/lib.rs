#![deny(unsafe_code)]

use std::sync::Arc;

use ascend_tools::client::AscendClient;
use ascend_tools::config::Config;
use ascend_tools::models;
use napi::bindgen_prelude::*;
use napi::threadsafe_function::{ThreadsafeFunction, ThreadsafeFunctionCallMode, UnknownReturnValue};
use napi::Task;
use napi_derive::napi;

// --- serde_json::Value wrapper that implements TypeName for Task trait ---

pub struct JsonValue(serde_json::Value);

impl TypeName for JsonValue {
    fn type_name() -> &'static str {
        "object"
    }
    fn value_type() -> napi::ValueType {
        napi::ValueType::Object
    }
}

impl ValidateNapiValue for JsonValue {}

impl ToNapiValue for JsonValue {
    #[allow(unsafe_code)]
    unsafe fn to_napi_value(
        env: napi::sys::napi_env,
        val: Self,
    ) -> napi::Result<napi::sys::napi_value> {
        // Safety: delegates to serde_json::Value's implementation
        unsafe { <serde_json::Value as ToNapiValue>::to_napi_value(env, val.0) }
    }
}

// --- Generic task types for libuv thread pool ---

pub struct ValueTask(Option<Box<dyn FnOnce() -> napi::Result<serde_json::Value> + Send>>);

impl ValueTask {
    fn new(f: impl FnOnce() -> napi::Result<serde_json::Value> + Send + 'static) -> Self {
        Self(Some(Box::new(f)))
    }
}

impl Task for ValueTask {
    type Output = serde_json::Value;
    type JsValue = JsonValue;

    fn compute(&mut self) -> napi::Result<Self::Output> {
        (self.0.take().expect("task already consumed"))()
    }

    fn resolve(&mut self, _env: Env, output: Self::Output) -> napi::Result<Self::JsValue> {
        Ok(JsonValue(output))
    }
}

pub struct VoidTask(Option<Box<dyn FnOnce() -> napi::Result<()> + Send>>);

impl VoidTask {
    fn new(f: impl FnOnce() -> napi::Result<()> + Send + 'static) -> Self {
        Self(Some(Box::new(f)))
    }
}

impl Task for VoidTask {
    type Output = ();
    type JsValue = ();

    fn compute(&mut self) -> napi::Result<Self::Output> {
        (self.0.take().expect("task already consumed"))()
    }

    fn resolve(&mut self, _env: Env, _output: Self::Output) -> napi::Result<Self::JsValue> {
        Ok(())
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

    #[napi]
    pub fn list_workspaces(
        &self,
        title: Option<String>,
        project: Option<String>,
        environment: Option<String>,
    ) -> AsyncTask<ValueTask> {
        let client = self.inner.clone();
        AsyncTask::new(ValueTask::new(move || {
            let mut filters = models::RuntimeFilters::default();
            filters.title = title;
            filters.project = project;
            filters.environment = environment;
            let result = client.list_workspaces(filters).map_err(to_napi_err)?;
            serde_json::to_value(&result).map_err(to_napi_err)
        }))
    }

    #[napi]
    pub fn get_workspace(
        &self,
        title: String,
        uuid: Option<String>,
    ) -> AsyncTask<ValueTask> {
        let client = self.inner.clone();
        AsyncTask::new(ValueTask::new(move || {
            let result = client
                .get_workspace(&title, uuid.as_deref())
                .map_err(to_napi_err)?;
            serde_json::to_value(&result).map_err(to_napi_err)
        }))
    }

    #[napi]
    pub fn pause_workspace(
        &self,
        title: String,
        uuid: Option<String>,
    ) -> AsyncTask<ValueTask> {
        let client = self.inner.clone();
        AsyncTask::new(ValueTask::new(move || {
            let result = client
                .pause_workspace(&title, uuid.as_deref())
                .map_err(to_napi_err)?;
            serde_json::to_value(&result).map_err(to_napi_err)
        }))
    }

    #[napi]
    pub fn resume_workspace(
        &self,
        title: String,
        uuid: Option<String>,
    ) -> AsyncTask<ValueTask> {
        let client = self.inner.clone();
        AsyncTask::new(ValueTask::new(move || {
            let result = client
                .resume_workspace(&title, uuid.as_deref())
                .map_err(to_napi_err)?;
            serde_json::to_value(&result).map_err(to_napi_err)
        }))
    }

    #[napi]
    #[allow(clippy::too_many_arguments)]
    pub fn create_workspace(
        &self,
        title: String,
        environment: String,
        project: String,
        profile: String,
        git_branch: String,
        git_branch_base: Option<String>,
        size: Option<String>,
        storage_size: Option<u32>,
        auto_snooze_timeout_minutes: Option<u32>,
    ) -> AsyncTask<ValueTask> {
        let client = self.inner.clone();
        AsyncTask::new(ValueTask::new(move || {
            let mut create = models::RuntimeCreate::new(
                &title,
                &environment,
                &project,
                &profile,
                &git_branch,
            );
            create.base_git_branch = git_branch_base;
            create.size = size;
            create.storage_size = storage_size;
            create.auto_snooze_timeout_minutes = auto_snooze_timeout_minutes;
            let result = client.create_workspace(&create).map_err(to_napi_err)?;
            serde_json::to_value(&result).map_err(to_napi_err)
        }))
    }

    #[napi]
    #[allow(clippy::too_many_arguments)]
    pub fn update_workspace(
        &self,
        title: String,
        uuid: Option<String>,
        new_title: Option<String>,
        git_branch: Option<String>,
        git_branch_base: Option<String>,
        profile: Option<String>,
        size: Option<String>,
        storage_size: Option<u32>,
        auto_snooze_timeout_minutes: Option<u32>,
    ) -> AsyncTask<ValueTask> {
        let client = self.inner.clone();
        AsyncTask::new(ValueTask::new(move || {
            let mut update = models::RuntimeUpdate::default();
            update.title = new_title;
            update.working_git_branch = git_branch;
            update.base_git_branch = git_branch_base;
            update.profile_name = profile;
            update.size = size;
            update.storage_size = storage_size;
            update.auto_snooze_timeout_minutes = auto_snooze_timeout_minutes;
            let result = client
                .update_workspace(&title, uuid.as_deref(), &update)
                .map_err(to_napi_err)?;
            serde_json::to_value(&result).map_err(to_napi_err)
        }))
    }

    #[napi]
    pub fn delete_workspace(
        &self,
        title: String,
        uuid: Option<String>,
    ) -> AsyncTask<VoidTask> {
        let client = self.inner.clone();
        AsyncTask::new(VoidTask::new(move || {
            client
                .delete_workspace(&title, uuid.as_deref())
                .map_err(to_napi_err)
        }))
    }

    // -- Deployment methods --

    #[napi]
    pub fn list_deployments(
        &self,
        title: Option<String>,
        project: Option<String>,
        environment: Option<String>,
    ) -> AsyncTask<ValueTask> {
        let client = self.inner.clone();
        AsyncTask::new(ValueTask::new(move || {
            let mut filters = models::RuntimeFilters::default();
            filters.title = title;
            filters.project = project;
            filters.environment = environment;
            let result = client.list_deployments(filters).map_err(to_napi_err)?;
            serde_json::to_value(&result).map_err(to_napi_err)
        }))
    }

    #[napi]
    pub fn get_deployment(
        &self,
        title: String,
        uuid: Option<String>,
    ) -> AsyncTask<ValueTask> {
        let client = self.inner.clone();
        AsyncTask::new(ValueTask::new(move || {
            let result = client
                .get_deployment(&title, uuid.as_deref())
                .map_err(to_napi_err)?;
            serde_json::to_value(&result).map_err(to_napi_err)
        }))
    }

    #[napi]
    #[allow(clippy::too_many_arguments)]
    pub fn create_deployment(
        &self,
        title: String,
        environment: String,
        project: String,
        profile: String,
        git_branch: String,
        git_branch_base: Option<String>,
        size: Option<String>,
        storage_size: Option<u32>,
        enable_automations: Option<bool>,
    ) -> AsyncTask<ValueTask> {
        let client = self.inner.clone();
        AsyncTask::new(ValueTask::new(move || {
            let mut create = models::RuntimeCreate::new(
                &title,
                &environment,
                &project,
                &profile,
                &git_branch,
            );
            create.base_git_branch = git_branch_base;
            create.size = size;
            create.storage_size = storage_size;
            create.enable_automations = enable_automations;
            let result = client.create_deployment(&create).map_err(to_napi_err)?;
            serde_json::to_value(&result).map_err(to_napi_err)
        }))
    }

    #[napi]
    #[allow(clippy::too_many_arguments)]
    pub fn update_deployment(
        &self,
        title: String,
        uuid: Option<String>,
        new_title: Option<String>,
        git_branch: Option<String>,
        git_branch_base: Option<String>,
        profile: Option<String>,
        size: Option<String>,
        storage_size: Option<u32>,
        enable_automations: Option<bool>,
    ) -> AsyncTask<ValueTask> {
        let client = self.inner.clone();
        AsyncTask::new(ValueTask::new(move || {
            let mut update = models::RuntimeUpdate::default();
            update.title = new_title;
            update.working_git_branch = git_branch;
            update.base_git_branch = git_branch_base;
            update.profile_name = profile;
            update.size = size;
            update.storage_size = storage_size;
            update.enable_automations = enable_automations;
            let result = client
                .update_deployment(&title, uuid.as_deref(), &update)
                .map_err(to_napi_err)?;
            serde_json::to_value(&result).map_err(to_napi_err)
        }))
    }

    #[napi]
    pub fn pause_deployment_automations(
        &self,
        title: String,
        uuid: Option<String>,
    ) -> AsyncTask<ValueTask> {
        let client = self.inner.clone();
        AsyncTask::new(ValueTask::new(move || {
            let result = client
                .pause_deployment_automations(&title, uuid.as_deref())
                .map_err(to_napi_err)?;
            serde_json::to_value(&result).map_err(to_napi_err)
        }))
    }

    #[napi]
    pub fn resume_deployment_automations(
        &self,
        title: String,
        uuid: Option<String>,
    ) -> AsyncTask<ValueTask> {
        let client = self.inner.clone();
        AsyncTask::new(ValueTask::new(move || {
            let result = client
                .resume_deployment_automations(&title, uuid.as_deref())
                .map_err(to_napi_err)?;
            serde_json::to_value(&result).map_err(to_napi_err)
        }))
    }

    #[napi]
    pub fn delete_deployment(
        &self,
        title: String,
        uuid: Option<String>,
    ) -> AsyncTask<VoidTask> {
        let client = self.inner.clone();
        AsyncTask::new(VoidTask::new(move || {
            client
                .delete_deployment(&title, uuid.as_deref())
                .map_err(to_napi_err)
        }))
    }

    // -- Environment methods --

    #[napi]
    pub fn list_environments(&self) -> AsyncTask<ValueTask> {
        let client = self.inner.clone();
        AsyncTask::new(ValueTask::new(move || {
            let result = client.list_environments().map_err(to_napi_err)?;
            serde_json::to_value(&result).map_err(to_napi_err)
        }))
    }

    #[napi]
    pub fn get_environment(&self, title: String) -> AsyncTask<ValueTask> {
        let client = self.inner.clone();
        AsyncTask::new(ValueTask::new(move || {
            let result = client.get_environment(&title).map_err(to_napi_err)?;
            serde_json::to_value(&result).map_err(to_napi_err)
        }))
    }

    // -- Project methods --

    #[napi]
    pub fn list_projects(&self) -> AsyncTask<ValueTask> {
        let client = self.inner.clone();
        AsyncTask::new(ValueTask::new(move || {
            let result = client.list_projects().map_err(to_napi_err)?;
            serde_json::to_value(&result).map_err(to_napi_err)
        }))
    }

    #[napi]
    pub fn get_project(&self, title: String) -> AsyncTask<ValueTask> {
        let client = self.inner.clone();
        AsyncTask::new(ValueTask::new(move || {
            let result = client.get_project(&title).map_err(to_napi_err)?;
            serde_json::to_value(&result).map_err(to_napi_err)
        }))
    }

    // -- Profile methods --

    #[napi]
    pub fn list_profiles(
        &self,
        workspace: Option<String>,
        deployment: Option<String>,
        uuid: Option<String>,
        project: Option<String>,
        branch: Option<String>,
    ) -> AsyncTask<ValueTask> {
        let client = self.inner.clone();
        AsyncTask::new(ValueTask::new(move || {
            let runtime_uuid = client
                .resolve_optional_runtime_target(
                    workspace.as_deref(),
                    deployment.as_deref(),
                    uuid.as_deref(),
                )
                .map_err(to_napi_err)?;
            let result = client
                .list_profiles(runtime_uuid.as_deref(), project.as_deref(), branch.as_deref())
                .map_err(to_napi_err)?;
            serde_json::to_value(&result).map_err(to_napi_err)
        }))
    }

    // -- Flow methods --

    #[napi]
    pub fn list_flows(
        &self,
        workspace: Option<String>,
        deployment: Option<String>,
        uuid: Option<String>,
    ) -> AsyncTask<ValueTask> {
        let client = self.inner.clone();
        AsyncTask::new(ValueTask::new(move || {
            let runtime_uuid = client
                .resolve_runtime_target(
                    workspace.as_deref(),
                    deployment.as_deref(),
                    uuid.as_deref(),
                )
                .map_err(to_napi_err)?;
            let result = client.list_flows(&runtime_uuid).map_err(to_napi_err)?;
            serde_json::to_value(&result).map_err(to_napi_err)
        }))
    }

    #[napi]
    #[allow(clippy::too_many_arguments)]
    pub fn run_flow(
        &self,
        flow_name: String,
        workspace: Option<String>,
        deployment: Option<String>,
        uuid: Option<String>,
        spec: Option<serde_json::Value>,
        resume: Option<bool>,
    ) -> AsyncTask<ValueTask> {
        let client = self.inner.clone();
        AsyncTask::new(ValueTask::new(move || {
            let runtime_uuid = client
                .resolve_runtime_target(
                    workspace.as_deref(),
                    deployment.as_deref(),
                    uuid.as_deref(),
                )
                .map_err(to_napi_err)?;
            let result = client
                .run_flow(
                    &runtime_uuid,
                    &flow_name,
                    spec,
                    resume.unwrap_or(false),
                )
                .map_err(to_napi_err)?;
            serde_json::to_value(&result).map_err(to_napi_err)
        }))
    }

    // -- Flow run methods --

    #[napi]
    #[allow(clippy::too_many_arguments)]
    pub fn list_flow_runs(
        &self,
        workspace: Option<String>,
        deployment: Option<String>,
        uuid: Option<String>,
        status: Option<String>,
        flow_name: Option<String>,
        since: Option<String>,
        until: Option<String>,
        offset: Option<u32>,
        limit: Option<u32>,
    ) -> AsyncTask<ValueTask> {
        let client = self.inner.clone();
        AsyncTask::new(ValueTask::new(move || {
            let runtime_uuid = client
                .resolve_runtime_target(
                    workspace.as_deref(),
                    deployment.as_deref(),
                    uuid.as_deref(),
                )
                .map_err(to_napi_err)?;
            let mut filters = models::FlowRunFilters::default();
            filters.status = status;
            filters.flow = flow_name;
            filters.since = since;
            filters.until = until;
            filters.offset = offset.map(u64::from);
            filters.limit = limit.map(u64::from);
            let result = client
                .list_flow_runs(&runtime_uuid, filters)
                .map_err(to_napi_err)?;
            serde_json::to_value(&result).map_err(to_napi_err)
        }))
    }

    #[napi]
    pub fn get_flow_run(
        &self,
        name: String,
        workspace: Option<String>,
        deployment: Option<String>,
        uuid: Option<String>,
    ) -> AsyncTask<ValueTask> {
        let client = self.inner.clone();
        AsyncTask::new(ValueTask::new(move || {
            let runtime_uuid = client
                .resolve_runtime_target(
                    workspace.as_deref(),
                    deployment.as_deref(),
                    uuid.as_deref(),
                )
                .map_err(to_napi_err)?;
            let result = client
                .get_flow_run(&runtime_uuid, &name)
                .map_err(to_napi_err)?;
            serde_json::to_value(&result).map_err(to_napi_err)
        }))
    }

    // -- Otto methods --

    #[napi]
    pub fn list_otto_providers(&self) -> AsyncTask<ValueTask> {
        let client = self.inner.clone();
        AsyncTask::new(ValueTask::new(move || {
            let result = client.list_otto_providers().map_err(to_napi_err)?;
            serde_json::to_value(&result).map_err(to_napi_err)
        }))
    }

    #[napi]
    #[allow(clippy::too_many_arguments)]
    pub fn otto_chat_streaming(
        &self,
        prompt: String,
        on_delta: ThreadsafeFunction<String, UnknownReturnValue>,
        workspace: Option<String>,
        deployment: Option<String>,
        uuid: Option<String>,
        thread_id: Option<String>,
        model: Option<String>,
        provider: Option<String>,
    ) -> AsyncTask<ValueTask> {
        let client = self.inner.clone();
        AsyncTask::new(ValueTask::new(move || {
            let otto_model = models::OttoModel::from_options(provider.as_deref(), model.as_deref());
            let runtime_id = client
                .resolve_optional_runtime_target(
                    workspace.as_deref(),
                    deployment.as_deref(),
                    uuid.as_deref(),
                )
                .map_err(to_napi_err)?;
            let request = models::OttoChatRequest {
                prompt,
                runtime_id,
                thread_id,
                model: otto_model,
            };
            let response = client
                .otto_chat_streaming(&request, |delta| {
                    on_delta.call(
                        Ok(delta.to_string()),
                        ThreadsafeFunctionCallMode::NonBlocking,
                    );
                })
                .map_err(to_napi_err)?;
            Ok(serde_json::json!({
                "message": response.message,
                "thread_id": response.thread_id,
            }))
        }))
    }

    #[napi]
    #[allow(clippy::too_many_arguments)]
    pub fn otto_chat(
        &self,
        prompt: String,
        workspace: Option<String>,
        deployment: Option<String>,
        uuid: Option<String>,
        thread_id: Option<String>,
        model: Option<String>,
        provider: Option<String>,
    ) -> AsyncTask<ValueTask> {
        let client = self.inner.clone();
        AsyncTask::new(ValueTask::new(move || {
            let otto_model = models::OttoModel::from_options(provider.as_deref(), model.as_deref());
            let runtime_id = client
                .resolve_optional_runtime_target(
                    workspace.as_deref(),
                    deployment.as_deref(),
                    uuid.as_deref(),
                )
                .map_err(to_napi_err)?;
            let request = models::OttoChatRequest {
                prompt,
                runtime_id,
                thread_id,
                model: otto_model,
            };
            let response = client.otto_chat(&request).map_err(to_napi_err)?;
            Ok(serde_json::json!({
                "message": response.message,
                "thread_id": response.thread_id,
            }))
        }))
    }
}

fn to_napi_err(e: impl std::fmt::Display) -> napi::Error {
    napi::Error::from_reason(e.to_string())
}
