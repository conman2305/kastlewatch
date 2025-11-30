use kube::{CustomResource, runtime::controller::Action, ResourceExt, Client};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use crate::shared::resources::common::{ControllerResource, MonitorState, SecretKeySelector};
use crate::shared::resources::notifiers::{self, NotifierResource};
use crate::shared::context::Context;
use std::sync::Arc;
use tokio::time::Duration;
use tracing::error;

/// Specification for the DiscordNotifier resource
#[derive(CustomResource, Deserialize, Serialize, Clone, Debug, JsonSchema)]
#[kube(
    group = "kastlewatch.io",
    version = "v1alpha1",
    kind = "DiscordNotifier",
    namespaced
)]
pub struct DiscordNotifierSpec {
    /// Reference to the secret containing the webhook URL
    pub webhook_secret_ref: SecretKeySelector,
    /// Optional message format (not currently used, but good for future proofing)
    pub message_format: Option<String>,
}

impl ControllerResource for DiscordNotifier {
    fn success_policy(&self) -> Action {
        // Notifiers don't need regular reconciliation unless we want to validate the webhook?
        // Let's just requeue every 60 minutes.
        Action::requeue(Duration::from_secs(3600))
    }

    fn error_policy(&self, error: &anyhow::Error, _ctx: Arc<Context>) -> Action {
        let name = self.name_any();
        error!("Reconciliation error for \"{}\": {:?}", name, error);
        Action::requeue(Duration::from_secs(60))
    }
}

impl NotifierResource for DiscordNotifier {
    async fn notify(&self, client: Client, monitor_name: &str, old_state: &MonitorState, new_state: &MonitorState) -> anyhow::Result<()> {
        // Get webhook URL
        let ns = self.namespace().unwrap_or_else(|| "default".to_string());
        let webhook_url = notifiers::get_secret_value(client, &ns, &self.spec.webhook_secret_ref).await?;
        
        self.send_discord_notification(&webhook_url, monitor_name, old_state, new_state).await
    }
}

impl DiscordNotifier {
    async fn send_discord_notification(&self, webhook_url: &str, monitor_name: &str, old_state: &MonitorState, new_state: &MonitorState) -> anyhow::Result<()> {
        // Build Discord payload
        let color = match new_state {
            MonitorState::Healthy => 0x00FF00, // Green
            MonitorState::Warning => 0xFFFF00, // Yellow
            MonitorState::Critical => 0xFF0000, // Red
            MonitorState::NoData => 0x808080, // Gray
        };
        
        let title = format!("Monitor {} is {:?}", monitor_name, new_state);
        let description = format!("State changed from {:?} to {:?}", old_state, new_state);
        
        let payload = serde_json::json!({
            "embeds": [{
                "title": title,
                "description": description,
                "color": color,
                "timestamp": chrono::Utc::now().to_rfc3339()
            }]
        });
        
        let http_client = reqwest::Client::new();
        let res = http_client.post(webhook_url)
            .json(&payload)
            .send()
            .await?;
            
        if !res.status().is_success() {
            return Err(anyhow::anyhow!("Discord API returned {}", res.status()));
        }
        
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wiremock::{MockServer, Mock, ResponseTemplate};
    use wiremock::matchers::{method, path, body_partial_json};

    #[tokio::test]
    async fn test_send_discord_notification() {
        let mock_server = MockServer::start().await;
        
        let notifier = DiscordNotifier::new("test-notifier", DiscordNotifierSpec {
            webhook_secret_ref: SecretKeySelector {
                name: "test-secret".to_string(),
                key: "url".to_string(),
            },
            message_format: None,
        });

        Mock::given(method("POST"))
            .and(path("/"))
            .and(body_partial_json(serde_json::json!({
                "embeds": [{
                    "title": "Monitor test-monitor is Critical",
                    "description": "State changed from Healthy to Critical",
                    "color": 0xFF0000
                }]
            })))
            .respond_with(ResponseTemplate::new(204))
            .mount(&mock_server)
            .await;

        let result = notifier.send_discord_notification(
            &mock_server.uri(),
            "test-monitor",
            &MonitorState::Healthy,
            &MonitorState::Critical
        ).await;

        assert!(result.is_ok());
    }
}
