use kube::{CustomResource, runtime::controller::Action, ResourceExt};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use crate::shared::resources::common::{MonitorConfigSpec, MonitorStatus, MonitorState, ControllerResource, self};
use crate::shared::resources::worker;
use crate::shared::context::Context;
use std::sync::Arc;
use tokio::time::Duration;
use tracing::{info, error};
use axum::{extract::{State, Json}, http::StatusCode};
use crate::shared::context::AppState;
use base64::prelude::*;

#[derive(Serialize, Deserialize, Clone, Debug, JsonSchema, PartialEq)]
pub enum Method {
    GET,
    POST,
}

/// Specification for the HTTPMonitor resource
#[derive(CustomResource, Deserialize, Serialize, Clone, Debug, JsonSchema)]
#[kube(
    group = "kastlewatch.io",
    version = "v1alpha1",
    kind = "HTTPMonitor",
    namespaced
)]
#[kube(status = "MonitorStatus")]
pub struct HTTPMonitorSpec {
    /// The URL to check
    pub url: String,
    /// Configuration for the monitoring behavior
    pub monitor_config: MonitorConfigSpec,
    /// Support GET or POST
    pub method: Method,
    /// An array of HTTP status codes that are allowed for success. Optional. If not defined, allow any 2XX status code.
    pub status_code: Option<Vec<u16>>,
    /// A base64 string of data to use as the body when method is POST. Optional. Must be base64 encoded.
    pub base64_data: Option<String>,
}


impl ControllerResource for HTTPMonitor {
    fn success_policy(&self) -> Action {
        Action::requeue(Duration::from_secs(self.spec.monitor_config.polling_frequency as u64))
    }

    fn error_policy(&self, error: &anyhow::Error, _ctx: Arc<Context>) -> Action {
        let name = self.name_any();
        error!("Reconciliation error for \"{}\": {:?}", name, error);
        Action::requeue(Duration::from_secs(5))
    }

    fn validate(&self) -> anyhow::Result<()> {
        if let Some(data) = &self.spec.base64_data {
            BASE64_STANDARD.decode(data).map_err(|e| anyhow::anyhow!("Invalid base64 data: {}", e))?;
        }
        Ok(())
    }
}

impl common::MonitorResource for HTTPMonitor {
    async fn check(&self) -> anyhow::Result<MonitorState> {
        let url = &self.spec.url;
        info!("Checking {}", url);
        
        let timeout = Duration::from_secs(self.spec.monitor_config.timeout as u64);
        let http_client = reqwest::Client::builder()
            .timeout(timeout)
            .build()
            .unwrap_or_default();

        let mut req_builder = match self.spec.method {
            Method::GET => http_client.get(url),
            Method::POST => http_client.post(url),
        };

        if let Some(base64_data) = &self.spec.base64_data {
            if let Ok(decoded) = BASE64_STANDARD.decode(base64_data) {
                req_builder = req_builder.body(decoded);
            } else {
                error!("Failed to decode base64 data for check, skipping body");
            }
        }

        let result = req_builder.send().await;
        
        let is_healthy = match result {
            Ok(response) => {
                let status = response.status().as_u16();
                if let Some(allowed_codes) = &self.spec.status_code {
                    allowed_codes.contains(&status)
                } else {
                    status >= 200 && status < 300
                }
            }
            Err(e) => {
                info!("Check failed: {:?}", e);
                false
            }
        };
        
        let new_state = if is_healthy {
            MonitorState::Healthy
        } else {
            MonitorState::Critical
        };
        info!("Check complete: {:?} (Healthy: {})", new_state, is_healthy);
        Ok(new_state)
    }

    async fn handle_http(
        State(state): State<AppState>,
        Json(monitor): Json<HTTPMonitor>
    ) -> StatusCode {
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