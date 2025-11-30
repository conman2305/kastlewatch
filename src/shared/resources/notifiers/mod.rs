use crate::shared::resources::common::{ControllerResource, MonitorState, SecretKeySelector};
use kube::{Client, Api, ResourceExt};
use std::collections::BTreeMap;
use tracing::{info, error};
use k8s_openapi::api::core::v1::Secret;

pub mod discord_notifier;

/// Trait for notifier resources
#[allow(async_fn_in_trait)]
pub trait NotifierResource: ControllerResource {
    /// Sends a notification
    async fn notify(&self, client: Client, monitor_name: &str, old_state: &MonitorState, new_state: &MonitorState) -> anyhow::Result<()>;
}

/// Helper to get a secret value
pub async fn get_secret_value(client: Client, namespace: &str, secret_ref: &SecretKeySelector) -> anyhow::Result<String> {
    let secrets: Api<Secret> = Api::namespaced(client, namespace);
    let secret = secrets.get(&secret_ref.name).await?;
    
    if let Some(data) = secret.data {
        if let Some(byte_string) = data.get(&secret_ref.key) {
            // ByteString is a wrapper around Vec<u8> that decodes from base64 when deserialized from k8s json
            // but here we are accessing the decoded bytes directly from the ByteString
            return Ok(std::str::from_utf8(&byte_string.0)?.to_string());
        }
    }
    
    Err(anyhow::anyhow!("Secret key {} not found in secret {}", secret_ref.key, secret_ref.name))
}

/// Process notifications for a monitor state change
pub async fn process_notifications(
    client: Client,
    monitor_name: &str,
    monitor_namespace: &str,
    match_labels: &Option<BTreeMap<String, String>>,
    old_state: &MonitorState,
    new_state: &MonitorState
) {
    if old_state == new_state {
        return;
    }

    if let Some(labels) = match_labels {
        // TODO: This currently only supports DiscordNotifier. 
        // In the future we might want a way to discover ALL types of notifiers.
        // For now, we will just query for DiscordNotifiers.
        use discord_notifier::v1alpha1::DiscordNotifier;
        
        let api: Api<DiscordNotifier> = Api::namespaced(client.clone(), monitor_namespace);
        let lp = kube::api::ListParams::default().labels(&labels.iter().map(|(k, v)| format!("{}={}", k, v)).collect::<Vec<_>>().join(","));
        
        match api.list(&lp).await {
            Ok(notifiers) => {
                for notifier in notifiers {
                    let notifier_name = notifier.name_any();
                    info!("Sending notification to {} for {}", notifier_name, monitor_name);
                    if let Err(e) = notifier.notify(client.clone(), monitor_name, old_state, new_state).await {
                        error!("Failed to notify {}: {:?}", notifier_name, e);
                    }
                }
            },
            Err(e) => error!("Failed to list notifiers: {:?}", e),
        }
    }
}
