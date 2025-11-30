use crate::shared::context::{AppState, Context};
use axum::{
    extract::{Json, State},
    http::StatusCode,
};
use kube::Resource;
use kube::runtime::controller::Action;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// Configuration for the monitoring behavior
#[derive(Deserialize, Serialize, Clone, Debug, JsonSchema)]
pub struct MonitorConfigSpec {
    /// Timeout in seconds for the connection attempt
    pub timeout: u32,
    /// Number of retries before considering the check failed
    pub retries: u32,
    /// Frequency in seconds to poll the target
    pub polling_frequency: u32,
    /// Labels to match notifiers
    pub notifiers_match_labels: Option<std::collections::BTreeMap<String, String>>,
}

/// Reference to a secret key
#[derive(Deserialize, Serialize, Clone, Debug, JsonSchema)]
pub struct SecretKeySelector {
    /// The name of the secret
    pub name: String,
    /// The key of the secret to select
    pub key: String,
}

/// The current state of the monitor
#[derive(Deserialize, Serialize, Clone, Debug, JsonSchema, PartialEq)]
pub enum MonitorState {
    /// The target is reachable and healthy
    Healthy,
    /// The target is reachable but showing signs of issues (not currently used)
    Warning,
    /// The target is unreachable
    Critical,
    /// No check has been performed yet
    NoData,
}

/// The status of the monitor resource
#[derive(Deserialize, Serialize, Clone, Debug, JsonSchema)]
pub struct MonitorStatus {
    /// The timestamp of the last check in RFC3339 format
    pub last_checked: Option<String>,
    /// The current state of the monitor
    pub state: MonitorState,
}

/* Helper functions */
pub fn build_worker_url<T>(base: &str) -> String
where
    T: Resource<DynamicType = ()>,
{
    // Construct URL: {base}/{version}/{kind}
    // dynamic discovery from the object type
    // The &() argument is required because api_version() and kind() take a &Self::DynamicType.
    // For CustomResources derived with #[kube], DynamicType defaults to (), so we pass a reference to unit.
    let api_version = T::api_version(&());
    let kind = T::kind(&());

    // api_version is typically "group/version" (e.g., "kastlewatch.io/v1alpha1")
    // We want just the version.
    let version = api_version.split('/').last().unwrap_or(&api_version);

    let kind_lower = kind.to_lowercase();

    format!("{}/{}/{}", base.trim_end_matches('/'), version, kind_lower)
}

/// Trait for resources that need controller logic (reconciliation policies)
pub trait ControllerResource:
    Resource<DynamicType = (), Scope = kube::core::NamespaceResourceScope>
    + Clone
    + Send
    + Sync
    + 'static
{
    /// Returns the action to take after a successful reconciliation
    fn success_policy(&self) -> Action;

    /// Returns the action to take after a failed reconciliation
    fn error_policy(&self, error: &anyhow::Error, _ctx: Arc<Context>) -> Action;

    /// Validates the resource configuration
    fn validate(&self) -> anyhow::Result<()> {
        Ok(())
    }
}

/// Trait for monitor resources to implement generic controller logic
#[allow(async_fn_in_trait)]
pub trait MonitorResource: ControllerResource {
    /// Performs the check and returns the state
    async fn check(&self) -> anyhow::Result<MonitorState>;

    /// Handles the HTTP request for the resource
    async fn handle_http(state: State<AppState>, monitor: Json<Self>) -> StatusCode;

    /// Returns the monitor configuration
    fn monitor_config(&self) -> &MonitorConfigSpec;

    /// Returns the current status of the monitor
    fn status(&self) -> Option<&MonitorStatus>;
}

pub async fn publish_event(
    client: kube::Client,
    resource_name: &str,
    resource_kind: &str,
    resource_api_version: &str,
    namespace: &str,
    reason: &str,
    message: &str,
    type_: &str,
) -> anyhow::Result<()> {
    use chrono::Utc;
    use k8s_openapi::api::core::v1::{Event, EventSource, ObjectReference};
    use k8s_openapi::apimachinery::pkg::apis::meta::v1::Time;
    use kube::Api;
    use kube::api::{ObjectMeta, PostParams};

    let events: Api<Event> = Api::namespaced(client, namespace);

    let event = Event {
        metadata: ObjectMeta {
            generate_name: Some(format!("{}-", resource_name)),
            namespace: Some(namespace.to_string()),
            ..Default::default()
        },
        involved_object: ObjectReference {
            kind: Some(resource_kind.to_string()),
            name: Some(resource_name.to_string()),
            namespace: Some(namespace.to_string()),
            api_version: Some(resource_api_version.to_string()),
            ..Default::default()
        },
        reason: Some(reason.to_string()),
        message: Some(message.to_string()),
        type_: Some(type_.to_string()),
        first_timestamp: Some(Time(Utc::now())),
        last_timestamp: Some(Time(Utc::now())),
        count: Some(1),
        source: Some(EventSource {
            component: Some("kastlewatch-controller".to_string()),
            ..Default::default()
        }),
        ..Default::default()
    };

    events.create(&PostParams::default(), &event).await?;
    Ok(())
}
