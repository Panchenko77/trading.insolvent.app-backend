use std::{env::current_dir, fmt::Debug, path::PathBuf};

use clap::Parser;
pub use dotenvy::dotenv;
use eyre::{eyre, Result};
use serde::{de::DeserializeOwned, *};
use serde_json::Value;

use crate::env::load_env_recursively;

#[derive(Parser)]
#[clap(author, version, about, long_about = None)]
struct CliArgument {
    /// The path to config file
    #[clap(short, long, value_parser, value_name = "FILE", env = "CONFIG")]
    config: Option<PathBuf>,
    /// The path to config file
    #[clap(long)]
    config_entry: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    #[serde(default)]
    pub name: String,
    pub addr: String,
    #[serde(default)]
    pub pub_certs: Option<Vec<String>>,
    #[serde(default)]
    pub priv_cert: Option<String>,
    #[serde(default)]
    pub debug: bool,
}

pub fn load_config<Config: DeserializeOwned + Debug>(
    default_config_file: impl Into<PathBuf>,
    service_name: impl AsRef<str>,
) -> Result<Config> {
    load_env_recursively()?;
    // print all environment variables
    // for (key, value) in std::env::vars() {
    //     println!("{}: {}", key, value);
    // }
    let args: CliArgument = CliArgument::parse();

    println!("Working directory {}", current_dir()?.display());
    let config_path = args.config.unwrap_or(default_config_file.into());
    println!("Loading config from {}", config_path.display());
    let config = std::fs::read_to_string(&config_path)?;
    let config: Value = serde_json::from_str(&config)?;
    if let Some(entry) = args.config_entry {
        parse_config(config, &entry)
    } else {
        parse_config(config, service_name.as_ref())
    }
}

pub fn parse_config<Config: DeserializeOwned + Debug>(
    mut config: Value,
    service_name: impl AsRef<str>,
) -> Result<Config> {
    let service_name = service_name.as_ref();
    let service_config = config
        .get_mut(&service_name)
        .ok_or_else(|| eyre!("Service {} not found in config", service_name))?
        .clone();
    let root = config.as_object_mut().unwrap();
    for (k, v) in service_config.as_object().unwrap() {
        root.insert(k.clone(), v.clone());
    }
    if service_config.get(service_name).is_none() {
        root.remove(service_name);
    }
    root.insert("name".to_string(), Value::String(service_name.to_string()));
    let config: Config = serde_json::from_value(config)?;
    println!("App config {:#?}", config);
    Ok(config)
}
