use crate::shared::resources::common::{MonitorResource, MonitorState};
use crate::shared::resources::notifiers;
use kube::{Api, Client, ResourceExt};
use tracing::{error, info};

pub async fn generic_worker_handler<T>(monitor: T, client: Client)
where
    T: MonitorResource + serde::Serialize + serde::de::DeserializeOwned + std::fmt::Debug,
{
    info!("Worker received {}: {}", T::kind(&()), monitor.name_any());

    let name = monitor.name_any();
    let ns = monitor.namespace().unwrap_or_else(|| "default".to_string());
    let api: Api<T> = Api::namespaced(client.clone(), &ns);

    let old_state = match monitor.status() {
        Some(status) => status.state.to_owned(),
        None => MonitorState::NoData,
    };

    let check_result = monitor.check().await;

    let new_state = match check_result {
        Ok(state) => state,
        Err(e) => {
            error!("Check failed for {}: {:?}", monitor.name_any(), e);
            MonitorState::NoData
        }
    };

    // Update Status
    let status = serde_json::json!({
        "status": {
            "last_checked": chrono::Utc::now().to_rfc3339(),
            "state": new_state
        }
    });

    match api
        .patch_status(
            &name,
            &kube::api::PatchParams::default(),
            &kube::api::Patch::Merge(&status),
        )
        .await
    {
        Ok(_) => info!("Successfully updated status for {}", name),
        Err(e) => error!("Failed to update status for {}: {:?}", name, e),
    }

    // Emit event if state changed
    if old_state != new_state {
        let reason = "StateChange";
        let message = format!(
            "Monitor state changed from {:?} to {:?}",
            old_state, new_state
        );
        let type_ = match new_state {
            MonitorState::Healthy => "Normal",
            _ => "Warning",
        };

        let kind = T::kind(&());
        let api_version = T::api_version(&());

        if let Err(e) = crate::shared::resources::common::publish_event(
            client.clone(),
            &name,
            &kind,
            &api_version,
            &ns,
            reason,
            &message,
            type_,
        )
        .await
        {
            error!("Failed to publish event for {}: {:?}", name, e);
        }
    }

    // Process notifications
    let config = monitor.monitor_config();
    notifiers::process_notifications(
        client,
        &name,
        &ns,
        &config.notifiers_match_labels,
        &old_state,
        &new_state,
    )
    .await;
}
