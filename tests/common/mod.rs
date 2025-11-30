use kastlewatch::{controller, shared, worker};
use kube::{Client, Config};
use shared::resources::monitors::http_monitor::v1alpha1::HTTPMonitor;
use shared::resources::monitors::tcp_monitor::v1alpha1::TCPMonitor;
use shared::resources::notifiers::discord_notifier::v1alpha1::DiscordNotifier;
use std::sync::Mutex;
use testcontainers::core::IntoContainerPort;
use testcontainers::{ContainerAsync, ImageExt, runners::AsyncRunner};
use testcontainers_modules::k3s::K3s;
use tokio::net::TcpListener;
use tokio::sync::OnceCell;
use tokio::time::Duration;

// Global static to hold the container and client
pub static K3S_INSTANCE: OnceCell<(Client, Mutex<Option<ContainerAsync<K3s>>>)> =
    OnceCell::const_new();

pub async fn get_k3s_instance() -> &'static Client {
    let (client, _node) = K3S_INSTANCE
        .get_or_init(|| async { setup_k3s().await.expect("Failed to setup K3s") })
        .await;
    client
}

// Returns both Client and the Container handle (wrapped in Mutex).
// IMPORTANT: The Container handle must be kept alive.
async fn setup_k3s() -> anyhow::Result<(Client, Mutex<Option<ContainerAsync<K3s>>>)> {
    // 1. Start K3s
    // Find a free port
    let free_port_listener = TcpListener::bind("127.0.0.1:0").await?;
    let host_port = free_port_listener.local_addr()?.port();
    drop(free_port_listener);

    let node = K3s::default()
        .with_tag("v1.34.2-k3s1")
        .with_mapped_port(host_port, 6443.tcp())
        .with_privileged(true)
        .with_env_var("K3S_KUBECONFIG_MODE", "644")
        .with_cmd(vec!["server", "--snapshotter=native"])
        .with_startup_timeout(Duration::from_secs(300))
        .start()
        .await?;

    // 2. Get Kubeconfig
    use tokio::io::AsyncReadExt;
    let mut exec_res = node
        .exec(testcontainers::core::ExecCommand::new(vec![
            "cat",
            "/etc/rancher/k3s/k3s.yaml",
        ]))
        .await?;
    let mut stdout_reader = exec_res.stdout();
    let mut kubeconfig_bytes = Vec::new();
    stdout_reader.read_to_end(&mut kubeconfig_bytes).await?;
    let kubeconfig_str = String::from_utf8(kubeconfig_bytes)?;

    // Replace the internal IP with localhost and the mapped port
    let kubeconfig_str = kubeconfig_str.replace(
        "server: https://127.0.0.1:6443",
        &format!("server: https://127.0.0.1:{}", host_port),
    );

    // Create a temporary file for the kubeconfig
    let temp_dir = tempfile::tempdir()?;
    let kubeconfig_path = temp_dir.path().join("kubeconfig");
    tokio::fs::write(&kubeconfig_path, kubeconfig_str).await?;

    // 3. Create Client
    let config = Config::from_custom_kubeconfig(
        kube::config::Kubeconfig::read_from(kubeconfig_path.clone())?,
        &kube::config::KubeConfigOptions::default(),
    )
    .await?;

    let client = Client::try_from(config)?;

    // Init CRDs
    controller::crd_manager::init_crds::<TCPMonitor>(client.clone()).await?;
    controller::crd_manager::init_crds::<HTTPMonitor>(client.clone()).await?;
    controller::crd_manager::init_crds::<DiscordNotifier>(client.clone()).await?;

    Ok((client, Mutex::new(Some(node))))
}

pub async fn start_services(client: Client) -> anyhow::Result<String> {
    // 1. Start Worker
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let worker_port = listener.local_addr()?.port();
    let worker_base_url = format!("http://127.0.0.1:{}", worker_port);

    let worker_client = client.clone();
    tokio::spawn(async move {
        if let Err(e) = worker::server::run(worker_client, listener).await {
            eprintln!("Worker failed: {:?}", e);
        }
    });

    // 2. Start Controller
    let controller_client = client.clone();
    let controller_settings = shared::settings::Settings {
        controller: shared::settings::ControllerSettings {
            base_url: worker_base_url.clone()
        },
        worker: shared::settings::WorkerSettings {
            host: "0.0.0.0".to_string(),
            port: 3000,
        },
    };

    tokio::spawn(async move {
        if let Err(e) = controller::controller::run(controller_client, controller_settings).await {
            eprintln!("Controller failed: {:?}", e);
        }
    });

    Ok(worker_base_url)
}

pub fn cleanup() {
    // Check if K3S_INSTANCE has been initialized
    if let Some((_client, node_mutex)) = K3S_INSTANCE.get() {
        println!("Cleaning up K3s container...");

        // Take the container out of the mutex
        let node_opt = node_mutex.lock().unwrap().take();

        if let Some(node) = node_opt {
            // Create a new runtime to execute the async stop command
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async {
                node.rm().await.ok();
            });
            println!("K3s container cleaned up.");
        }
    }
}
