use http::{Request, Response};
use kastlewatch::controller::common;
use kastlewatch::shared::context::Context;
use kastlewatch::shared::resources::common::MonitorConfigSpec;
use kastlewatch::shared::resources::monitors::http_monitor::v1alpha1::{
    HTTPMonitor, HTTPMonitorSpec, Method,
};
use kastlewatch::shared::resources::monitors::tcp_monitor::v1alpha1::{TCPMonitor, TCPMonitorSpec};
use kastlewatch::shared::settings::{ControllerSettings, Settings, WorkerSettings};
use kube::Client;
use kube::client::Body;
use kube::runtime::controller::Action;
use std::sync::Arc;
use tokio::time::Duration;
use tower_test::mock;
use chrono::Utc;
use kastlewatch::shared::resources::common::{MonitorState, MonitorStatus};

#[tokio::test]
async fn test_tcp_reconcile_success() {
    let (mock_service, _handle) = mock::pair::<Request<Body>, Response<Body>>();
    let client = Client::new(mock_service, "default");

    let settings = Settings {
        controller: ControllerSettings {
            base_url: "http://worker:3000".to_string(),
        },
        worker: WorkerSettings {
            host: "0.0.0.0".to_string(),
            port: 3000,
        },
    };
    let ctx = Arc::new(Context { client, settings });

    let monitor = TCPMonitor::new(
        "test-monitor",
        TCPMonitorSpec {
            host: "localhost".to_string(),
            port: 8080,
            monitor_config: MonitorConfigSpec {
                timeout: 5,
                retries: 3,
                polling_frequency: 10,
                notifiers_match_labels: None,
            },
        },
    );

    let result = common::reconcile(Arc::new(monitor), ctx).await;

    assert!(result.is_ok());
    let action = result.unwrap();
    assert_eq!(action, Action::requeue(Duration::from_secs(10)));
}

#[tokio::test]
async fn test_http_reconcile_success() {
    let (mock_service, _handle) = mock::pair::<Request<Body>, Response<Body>>();
    let client = Client::new(mock_service, "default");

    let settings = Settings {
        controller: ControllerSettings {
            base_url: "http://worker:3000".to_string(),
        },
        worker: WorkerSettings {
            host: "0.0.0.0".to_string(),
            port: 3000,
        },
    };
    let ctx = Arc::new(Context { client, settings });

    let monitor = HTTPMonitor::new(
        "test-monitor",
        HTTPMonitorSpec {
            url: "http://example.com".to_string(),
            method: Method::GET,
            status_code: None,
            base64_data: None,
            monitor_config: MonitorConfigSpec {
                timeout: 5,
                retries: 3,
                polling_frequency: 10,
                notifiers_match_labels: None,
            },
        },
    );

    let result = common::reconcile(Arc::new(monitor), ctx).await;

    assert!(result.is_ok());
    let action = result.unwrap();
    assert_eq!(action, Action::requeue(Duration::from_secs(10)));
}

#[tokio::test]
async fn test_http_reconcile_invalid_base64() {
    let (mock_service, _handle) = mock::pair::<Request<Body>, Response<Body>>();
    let client = Client::new(mock_service, "default");

    let settings = Settings {
        controller: ControllerSettings {
            base_url: "http://worker:3000".to_string(),
        },
        worker: WorkerSettings {
            host: "0.0.0.0".to_string(),
            port: 3000,
        },
    };
    let ctx = Arc::new(Context { client, settings });

    let monitor = HTTPMonitor::new(
        "test-monitor",
        HTTPMonitorSpec {
            url: "http://example.com".to_string(),
            method: Method::POST,
            status_code: None,
            base64_data: Some("invalid-base64!".to_string()),
            monitor_config: MonitorConfigSpec {
                timeout: 5,
                retries: 3,
                polling_frequency: 10,
                notifiers_match_labels: None,
            },
        },
    );

    let result = common::reconcile(Arc::new(monitor), ctx).await;

    assert!(result.is_err());
}

#[tokio::test]
async fn test_reconcile_skips_recent_check() {
    let (mock_service, _handle) = mock::pair::<Request<Body>, Response<Body>>();
    let client = Client::new(mock_service, "default");

    let settings = Settings {
        controller: ControllerSettings {
            base_url: "http://worker:3000".to_string(),
        },
        worker: WorkerSettings {
            host: "0.0.0.0".to_string(),
            port: 3000,
        },
    };
    let ctx = Arc::new(Context { client, settings });

    let mut monitor = TCPMonitor::new(
        "test-monitor",
        TCPMonitorSpec {
            host: "localhost".to_string(),
            port: 8080,
            monitor_config: MonitorConfigSpec {
                timeout: 5,
                retries: 3,
                polling_frequency: 30,
                notifiers_match_labels: None,
            },
        },
    );

    // Simulate a check that happened 1 second ago
    let last_checked = Utc::now() - chrono::Duration::seconds(1);
    monitor.status = Some(MonitorStatus {
        last_checked: Some(last_checked.to_rfc3339()),
        state: MonitorState::Healthy,
    });

    let result = common::reconcile(Arc::new(monitor), ctx).await;

    assert!(result.is_ok());
    let action = result.unwrap();
    
    // Should requeue for roughly 29 seconds (30 - 1)
    // We allow a small margin of error for execution time
    let expected_29 = Action::requeue(Duration::from_secs(29));
    let expected_28 = Action::requeue(Duration::from_secs(28));

    assert!(action == expected_29 || action == expected_28, "Expected requeue of 28s or 29s, got {:?}", action);
}
