use kastlewatch::shared;
use kastlewatch::shared::resources::common::{MonitorConfigSpec, MonitorState};
use kastlewatch::shared::resources::monitors::tcp_monitor::v1alpha1::TCPMonitor;
use kube::api::{Api, PostParams};

use ctor::dtor;
use tokio::time::{Duration, sleep};

mod common;

#[tokio::test]
async fn test_end_to_end_controller_worker_flow() -> anyhow::Result<()> {
    // Use the shared K3s instance
    let client = common::get_k3s_instance().await;

    // Start services
    common::start_services(client.clone()).await?;

    // Run Test Scenario
    let monitors: Api<TCPMonitor> = Api::namespaced(client.clone(), "default");

    let monitor = TCPMonitor::new(
        "test-monitor",
        shared::resources::monitors::tcp_monitor::v1alpha1::TCPMonitorSpec {
            host: "google.com".to_string(),
            port: 80,
            monitor_config: MonitorConfigSpec {
                polling_frequency: 5,
                timeout: 5,
                retries: 3,
                notifiers_match_labels: None,
            },
        },
    );

    monitors.create(&PostParams::default(), &monitor).await?;

    // Verify initial status is None (effectively NoData)
    let m = monitors.get("test-monitor").await?;
    assert!(m.status.is_none(), "Initial status should be None");

    // Wait for status update
    let mut success = false;
    for _ in 0..20 {
        // Wait up to 20 seconds
        let m = monitors.get("test-monitor").await?;
        if let Some(status) = m.status {
            println!("Got status: {:?}", status);
            if matches!(status.state, MonitorState::Healthy) {
                success = true;
                break;
            }
        }
        sleep(Duration::from_secs(1)).await;
    }

    assert!(
        success,
        "Timed out waiting for monitor status to become Healthy"
    );
    Ok(())
}

#[dtor]
fn cleanup() {
    common::cleanup();
}
