use crate::shared::context::AppState;
use crate::shared::context::Context;
use crate::shared::resources::common::{
    self, ControllerResource, MonitorConfigSpec, MonitorState, MonitorStatus,
};
use crate::shared::resources::monitors::tcp_monitor::check_tcp_connection;
use crate::shared::resources::worker;
use axum::{
    extract::{Json, State},
    http::StatusCode,
};
use kube::{CustomResource, ResourceExt, runtime::controller::Action};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::time::Duration;
use tracing::{error, info};

/// Specification for the TCPMonitor resource
#[derive(CustomResource, Deserialize, Serialize, Clone, Debug, JsonSchema)]
#[kube(
    group = "kastlewatch.io",
    version = "v1alpha1",
    kind = "TCPMonitor",
    namespaced
)]
#[kube(status = "MonitorStatus")]
pub struct TCPMonitorSpec {
    /// The hostname or IP address of the target
    pub host: String,
    /// The port number to check
    pub port: u16,
    /// Configuration for the monitoring behavior
    pub monitor_config: MonitorConfigSpec,
}

impl ControllerResource for TCPMonitor {
    fn success_policy(&self) -> Action {
        Action::requeue(Duration::from_secs(
            self.spec.monitor_config.polling_frequency as u64,
        ))
    }

    fn error_policy(&self, error: &anyhow::Error, _ctx: Arc<Context>) -> Action {
        let name = self.name_any();
        error!("Reconciliation error for \"{}\": {:?}", name, error);
        Action::requeue(Duration::from_secs(5))
    }
}

impl common::MonitorResource for TCPMonitor {
    async fn check(&self) -> anyhow::Result<MonitorState> {
        let host = &self.spec.host;
        let port = self.spec.port;
        info!("Checking {}:{}", host, port);

        let timeout = Duration::from_secs(self.spec.monitor_config.timeout as u64);
        let is_open = check_tcp_connection(host, port, timeout).await;

        let new_state = if is_open {
            MonitorState::Healthy
        } else {
            MonitorState::Critical
        };
        info!("Check complete: {:?} (Open: {})", new_state, is_open);
        Ok(new_state)
    }

    async fn handle_http(State(state): State<AppState>, Json(monitor): Json<Self>) -> StatusCode {
        tokio::spawn(async move {
            worker::generic_worker_handler(monitor, state.client).await;
        });
        StatusCode::OK
    }

    fn monitor_config(&self) -> &MonitorConfigSpec {
        &self.spec.monitor_config
    }

    fn status(&self) -> Option<&MonitorStatus> {
        self.status.as_ref()
    }
}
