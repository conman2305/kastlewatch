use kube::{
    api::{Api, PostParams},
};
use kastlewatch::shared::resources::monitors::tcp_monitor::v1alpha1::TCPMonitor;
use kastlewatch::shared::resources::notifiers::discord_notifier::v1alpha1::DiscordNotifier;
use kastlewatch::shared::resources::common::{MonitorConfigSpec, SecretKeySelector};
use k8s_openapi::api::core::v1::Secret;

use std::collections::BTreeMap;
use tokio::time::{sleep, Duration};
use ctor::dtor;
use wiremock::{MockServer, Mock, ResponseTemplate};
use wiremock::matchers::{method, path};

mod common;

#[tokio::test]
async fn test_discord_notification_flow() -> anyhow::Result<()> {

    // Start Mock Discord Server
    let mock_server = MockServer::start().await;
    
    Mock::given(method("POST"))
        .and(path("/"))
        .respond_with(ResponseTemplate::new(204))
        .mount(&mock_server)
        .await;

    // Use the shared K3s instance
    let client = common::get_k3s_instance().await;

    // Start services
    common::start_services(client.clone()).await?;

    // 1. Create Secret with Webhook URL
    let secrets: Api<Secret> = Api::namespaced(client.clone(), "default");
    let secret = Secret {
        metadata: kube::api::ObjectMeta {
            name: Some("discord-webhook".to_string()),
            ..Default::default()
        },
        string_data: Some(BTreeMap::from([
            ("url".to_string(), mock_server.uri())
        ])),
        ..Default::default()
    };
    secrets.create(&PostParams::default(), &secret).await?;

    // 2. Create DiscordNotifier
    let notifiers: Api<DiscordNotifier> = Api::namespaced(client.clone(), "default");
    let notifier = DiscordNotifier::new("test-notifier", kastlewatch::shared::resources::notifiers::discord_notifier::v1alpha1::DiscordNotifierSpec {
        webhook_secret_ref: SecretKeySelector {
            name: "discord-webhook".to_string(),
            key: "url".to_string(),
        },
        message_format: None,
    });
    // Add labels for matching
    let mut notifier = notifier;
    notifier.metadata.labels = Some(BTreeMap::from([
        ("type".to_string(), "discord".to_string())
    ]));
    notifiers.create(&PostParams::default(), &notifier).await?;

    // 3. Create TCPMonitor that matches the notifier
    // We point it to a non-existent host to trigger a failure (Critical state)
    // Or we can point to google.com and expect Healthy.
    // Let's point to google.com:80 to get Healthy state.
    // The worker will transition NoData -> Healthy, which should trigger a notification.
    let monitors: Api<TCPMonitor> = Api::namespaced(client.clone(), "default");
    let monitor = TCPMonitor::new("test-monitor-notify", kastlewatch::shared::resources::monitors::tcp_monitor::v1alpha1::TCPMonitorSpec {
        host: "google.com".to_string(),
        port: 80,
        monitor_config: MonitorConfigSpec {
            polling_frequency: 5,
            timeout: 5,
            retries: 3,
            notifiers_match_labels: Some(BTreeMap::from([
                ("type".to_string(), "discord".to_string())
            ])),
        },
    });
    monitors.create(&PostParams::default(), &monitor).await?;

    // 4. Verify Mock Server received request
    // We wait for the worker to process and send the notification.
    let mut received = false;
    for _ in 0..20 { // Wait up to 20 seconds
        if let Some(reqs) = mock_server.received_requests().await {
            if reqs.len() > 0 {
                received = true;
                break;
            }
        }
        sleep(Duration::from_secs(1)).await;
    }

    assert!(received, "Timed out waiting for Discord notification");
    
    // Optional: Verify payload content
    let requests = mock_server.received_requests().await.unwrap();
    let last_request = requests.last().unwrap();
    let body: serde_json::Value = serde_json::from_slice(&last_request.body)?;
    println!("Received payload: {:?}", body);
    
    // We expect "State changed from NoData to Healthy"
    let description = body["embeds"][0]["description"].as_str().unwrap();
    assert!(description.contains("NoData to Healthy"));
    
    // Verify color for Healthy state (0x00FF00 = 65280)
    let color = body["embeds"][0]["color"].as_u64().unwrap();
    assert_eq!(color, 65280, "Color should be Green (65280) for Healthy state");

    Ok(())
}

#[dtor]
fn cleanup() {
    common::cleanup();
}
