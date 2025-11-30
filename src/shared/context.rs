use crate::shared::settings::Settings;
use kube::Client;

#[derive(Clone)]
pub struct Context {
    pub client: Client,
    pub settings: Settings,
}
#[derive(Clone)]
pub struct AppState {
    pub client: Client,
}
