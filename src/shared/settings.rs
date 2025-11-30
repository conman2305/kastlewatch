use serde::Deserialize;
use config::{Config, File, ConfigError};

#[derive(Debug, Deserialize, Clone)]
pub struct Settings {
    pub controller: ControllerSettings,
}

#[derive(Debug, Deserialize, Clone)]
pub struct ControllerSettings {
    pub base_url: String,
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
