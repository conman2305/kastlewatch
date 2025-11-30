use crate::controller::common;
use crate::shared::resources::monitors::http_monitor::v1alpha1::HTTPMonitor;
use crate::shared::resources::monitors::tcp_monitor::v1alpha1::TCPMonitor;
use crate::shared::resources::notifiers::discord_notifier::v1alpha1::DiscordNotifier;
use kube::Client;
use tracing::info;

use crate::shared::settings::Settings;

pub async fn run(client: Client, settings: Settings) -> anyhow::Result<()> {
    info!("Starting TCPMonitor and HTTPMonitor controllers");

    let tcp_fut = common::run_monitor_controller::<TCPMonitor>(client.clone(), settings.clone());
    let http_fut = common::run_monitor_controller::<HTTPMonitor>(client.clone(), settings.clone());
    let discord_fut =
        common::run_notifier_controller::<DiscordNotifier>(client.clone(), settings.clone());

    futures::future::join3(tcp_fut, http_fut, discord_fut).await;

    Ok(())
}
