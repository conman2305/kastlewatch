use kube::Client;
use crate::shared::settings::Settings;

#[derive(Clone)]
pub struct Context {
    pub client: Client,
    pub settings: Settings,
}
#[derive(Clone)]
pub struct AppState {
    pub client: Client,
}
