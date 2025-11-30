use clap::{Parser, Subcommand};
use kastlewatch::{controller, shared};
use kube::Client;
use tracing::{error, info};
use tracing_subscriber::FmtSubscriber;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Runs the controller
    Controller,
    /// Runs the worker
    Worker,
    /// Generates CRD YAML
    Crdgen,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize tracing
    let subscriber = FmtSubscriber::builder()
        .with_max_level(tracing::Level::INFO)
        .finish();
    tracing::subscriber::set_global_default(subscriber).expect("setting default subscriber failed");

    let cli = Cli::parse();
    let settings = shared::settings::Settings::new()?;

    match &cli.command {
        Commands::Controller => {
            info!("Starting KastleWatch Controller");
            let client = Client::try_default().await?;

            // Initialize CRDs
            if let Err(e) = controller::crd_manager::init_crds::<
                shared::resources::monitors::tcp_monitor::v1alpha1::TCPMonitor,
            >(client.clone())
            .await
            {
                error!("Failed to initialize TCPMonitor CRD: {:?}", e);
                return Err(e);
            }
            if let Err(e) = controller::crd_manager::init_crds::<
                shared::resources::monitors::http_monitor::v1alpha1::HTTPMonitor,
            >(client.clone())
            .await
            {
                error!("Failed to initialize HTTPMonitor CRD: {:?}", e);
                return Err(e);
            }
            if let Err(e) = controller::crd_manager::init_crds::<
                shared::resources::notifiers::discord_notifier::v1alpha1::DiscordNotifier,
            >(client.clone())
            .await
            {
                error!("Failed to initialize DiscordNotifier CRD: {:?}", e);
                return Err(e);
            }

            // Run Controller
            if let Err(e) = controller::controller::run(client, settings).await {
                error!("Controller failed: {:?}", e);
                return Err(e);
            }
        }
        Commands::Worker => {
            info!("Starting KastleWatch Worker");
            let client = Client::try_default().await?;
            let addr = format!("{}:{}", settings.worker.host, settings.worker.port);
            let listener = tokio::net::TcpListener::bind(&addr).await?;
            if let Err(e) = kastlewatch::worker::server::run(client, listener).await {
                error!("Worker failed: {:?}", e);
                return Err(e);
            }
        }
        Commands::Crdgen => {
            use kube::CustomResourceExt;
            println!(
                "{}",
                serde_yaml::to_string(
                    &shared::resources::monitors::tcp_monitor::v1alpha1::TCPMonitor::crd()
                )?
            );
            println!(
                "---\n{}",
                serde_yaml::to_string(
                    &shared::resources::monitors::http_monitor::v1alpha1::HTTPMonitor::crd()
                )?
            );
            println!(
                "---\n{}",
                serde_yaml::to_string(
                    &shared::resources::notifiers::discord_notifier::v1alpha1::DiscordNotifier::crd(
                    )
                )?
            );
        }
    }

    Ok(())
}
