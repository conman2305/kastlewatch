use kube::{Client, Api, ResourceExt};
use crate::shared::resources::common::{MonitorResource, MonitorState};
use crate::shared::resources::notifiers;
use tracing::{info, error};

pub async fn generic_worker_handler<T>(monitor: T, client: Client)
where
    T: MonitorResource + serde::Serialize + serde::de::DeserializeOwned + std::fmt::Debug,
{
    info!("Worker received {}: {}", T::kind(&()), monitor.name_any());
    
    // We need to get the OLD state to compare.
    // However, the monitor object passed in IS the current state from K8s, 
    // but its status might be stale if we are just about to check.
    // Actually, the `monitor` object here comes from the HTTP request body, which is the full CRD.
    // So `monitor.status` contains the last known state.
    
    // We can't easily access the status field generically because it's not part of the MonitorResource trait directly,
    // but the struct T has it. However, to access it generically we might need to adjust the trait or use dynamic dispatch if we want to be clean.
    // BUT, we are deserializing T. 
    // Let's assume for now we can't easily get the old state without querying or changing the trait.
    // Wait, the `MonitorStatus` is standard.
    // Let's check `common.rs`. `MonitorStatus` is a struct.
    // The `MonitorResource` trait doesn't expose status.
    // But `T` implements `Deserialize`.
    // The `monitor` variable IS the deserialized object.
    // If T is `HTTPMonitor`, it has a `status` field.
    // But we can't access `monitor.status` unless `T` is known to have it or we have a trait method.
    
    // For now, let's just assume NoData if we can't get it, or maybe we should query the API to be sure?
    // Querying the API is safer anyway.
    

    
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

    match api.patch_status(
        &name,
        &kube::api::PatchParams::default(),
        &kube::api::Patch::Merge(&status)
    ).await {
        Ok(_) => info!("Successfully updated status for {}", name),
        Err(e) => error!("Failed to update status for {}: {:?}", name, e),
    }
    
    // Process notifications
    let config = monitor.monitor_config();
    notifiers::process_notifications(
        client,
        &name,
        &ns,
        &config.notifiers_match_labels,
        &old_state,
        &new_state
    ).await;
}
