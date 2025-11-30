use axum::{
    routing::{get, post},
    Router,
};
use kube::Client;
use crate::shared::resources::monitors::tcp_monitor;
use crate::shared::resources::monitors::http_monitor;
use crate::shared::resources::common::MonitorResource;
use tracing::info;

use crate::shared::context::AppState;

pub async fn run(client: Client, listener: tokio::net::TcpListener) -> anyhow::Result<()> {
    let local_addr = listener.local_addr()?;
    info!("Starting Worker Server on {}", local_addr);
    // client is passed in
    let state = AppState { client };

    let app = Router::new()
        .route("/healthz", get(|| async { "OK" }))
        .route("/readyz", get(|| async { "OK" }))
        .route("/v1alpha1/tcpmonitor", post(tcp_monitor::v1alpha1::TCPMonitor::handle_http))
        .route("/v1alpha1/httpmonitor", post(http_monitor::v1alpha1::HTTPMonitor::handle_http))
        .with_state(state);

    axum::serve(listener, app).await?;

    Ok(())
}

