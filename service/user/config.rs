use std::path::PathBuf;
use std::str::FromStr;

use lib::log::LogLevel;
use lib::ws::WsServerConfig;
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize, Default)]
pub struct DatabaseConfig {
    pub directory: PathBuf,
}
#[derive(Debug, Clone, Deserialize)]
pub struct LogConfig {
    pub level: LogLevel,
    pub file: Option<PathBuf>,
}
#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    pub database: DatabaseConfig,
    pub server: WsServerConfig,
    pub log: LogConfig,
    #[serde(default)]
    pub skip_key: bool,
}

impl FromStr for Config {
    type Err = toml::de::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        toml::from_str(s)
    }
}

impl TryFrom<PathBuf> for Config {
    type Error = eyre::Error;

    fn try_from(path: PathBuf) -> Result<Self, Self::Error> {
        let toml_str = std::fs::read_to_string(path).map_err(|e| eyre::eyre!("{e}"))?;
        Config::from_str(&toml_str).map_err(|e| eyre::eyre!("{e}"))
    }
}
