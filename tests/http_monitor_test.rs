
use kube::{
    api::{Api, PostParams},
};
use kastlewatch::shared::resources::monitors::http_monitor::v1alpha1::{HTTPMonitor, HTTPMonitorSpec, Method};
use kastlewatch::shared::resources::common::{MonitorConfigSpec, MonitorState};
use std::env;
use tokio::time::{sleep, Duration};
use ctor::dtor;

mod common;

#[tokio::test]
async fn test_http_monitor_flow() -> anyhow::Result<()> {
    if env::var("ENABLE_INTEGRATION_TESTS").is_err() {
        println!("Skipping integration test. Set ENABLE_INTEGRATION_TESTS=1 to run.");
        return Ok(());
    }

    // Use the shared K3s instance
    let client = common::get_k3s_instance().await;

    // Start services and get worker URL
    let worker_url = common::start_services(client.clone()).await?;
    println!("Worker running at {}", worker_url);

    // Parse worker URL to get host and port
    let url = reqwest::Url::parse(&worker_url)?;
    let host = url.host_str().unwrap().to_string();
    let port = url.port().unwrap();

    // Run Test Scenario
    let monitors: Api<HTTPMonitor> = Api::namespaced(client.clone(), "default");

    // Monitor the worker's own healthz endpoint
    let monitor = HTTPMonitor::new("http-test-monitor", HTTPMonitorSpec {
        url: format!("http://{}:{}/healthz", host, port),
        monitor_config: MonitorConfigSpec {
            polling_frequency: 5,
            timeout: 5,
            retries: 3,
            notifiers_match_labels: None,
        },
        method: Method::GET,
        status_code: None,
        base64_data: None,
    });

    monitors.create(&PostParams::default(), &monitor).await?;

    // Verify initial status is None
    let m = monitors.get("http-test-monitor").await?;
    assert!(m.status.is_none(), "Initial status should be None");

    // Wait for status update
    let mut success = false;
    for _ in 0..20 { // Wait up to 20 seconds
        let m = monitors.get("http-test-monitor").await?;
        if let Some(status) = m.status {
            println!("Got status: {:?}", status);
            if matches!(status.state, MonitorState::Healthy) {
                success = true;
                break;
            }
        }
        sleep(Duration::from_secs(1)).await;
    }

    assert!(success, "Timed out waiting for monitor status to become Healthy");
    Ok(())
}

#[dtor]
fn cleanup() {
    common::cleanup();
}
