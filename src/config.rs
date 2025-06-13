use config::{Config, ConfigError, File};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct ServerConfig {
    pub port: u16,
}

impl ServerConfig {
    pub fn new() -> Result<Self, ConfigError> {
        let settings = Config::builder()
            .add_source(
                File::with_name("config")
                    .format(config::FileFormat::Json)
                    .required(false),
            )
            .set_default("port", 3000)?
            .build()?;

        settings.try_deserialize()
    }
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self { port: 3000 }
    }
}
