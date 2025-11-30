use config::{Config, ConfigError, File};
use serde::Deserialize;

#[derive(Debug, Deserialize, Clone)]
pub struct Settings {
    pub controller: ControllerSettings,
    pub worker: WorkerSettings,
}

#[derive(Debug, Deserialize, Clone)]
pub struct ControllerSettings {
    pub base_url: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct WorkerSettings {
    pub port: u16,
    pub host: String,
}

impl Settings {
    pub fn new() -> Result<Self, ConfigError> {
        let s = Config::builder()
            .add_source(File::with_name("config"))
            .build()?;

        s.try_deserialize()
    }
}
