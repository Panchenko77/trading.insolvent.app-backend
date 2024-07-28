use crate::utils::crypto::{PrivateKey, PrivateKeyOptions};
use eyre::{bail, Context, Ok};
use serde::Deserialize;

fn read_env_var(var: &mut String, name: &str) -> eyre::Result<()> {
    if var.is_empty() {
        let v = std::env::var(name).with_context(|| format!("reading env var: {}", name))?;
        *var = v;
    }
    Ok(())
}
fn read_env_var_secret(var: &mut PrivateKey, name: &str) -> eyre::Result<()> {
    if var.is_empty() {
        let v = std::env::var(name).with_context(|| format!("reading env var: {}", name))?;
        *var = PrivateKey::new(v, PrivateKeyOptions::ALL)?;
    }
    Ok(())
}

#[derive(Debug, Clone, Deserialize)]
pub struct SigningApiKeySecret {
    pub env: Option<String>,
    #[serde(default)]
    pub api_key: PrivateKey,
    #[serde(default)]
    pub api_secret: PrivateKey,
    #[serde(default)]
    pub passphrase: PrivateKey,

}

impl SigningApiKeySecret {
    pub fn empty() -> Self {
        Self {
            env: None,
            api_key: Default::default(),
            api_secret: Default::default(),
            passphrase: Default::default(),
           // passphrase
        }
    }
    fn name<'a>(&'a self, default_name: &'a str) -> &str {
        self.env.as_deref().unwrap_or(default_name)
    }

    pub fn try_load_from_env(&mut self, default_name: &str) -> eyre::Result<()> {
        let env = self.name(default_name);
        let key_name = format!("{}_API_KEY", env);
        let secret_name = format!("{}_API_SECRET", env);
        let passphrase_name = format!("{}_PASSPHRASE", env);
        read_env_var_secret(&mut self.api_key, &key_name)?;
        read_env_var_secret(&mut self.api_secret, &secret_name)?;
        let _ = read_env_var_secret(&mut self.passphrase, &passphrase_name);
        Ok(())
    }
    pub fn verify(&self, name: &str) -> eyre::Result<()> {
        let name = self.name(name);
        if self.api_key.is_empty() {
            bail!("{}: api_key is empty", name);
        }
        if self.api_secret.is_empty() {
            bail!("{}: api_secret is empty", name);
        }
        Ok(())
    }

    pub fn verify_passphrase(&self, passphrase: &str) -> eyre::Result<()> {
        let passphrase = self.name(passphrase);
        if self.passphrase.is_empty() {
            bail!("{}: passphrase is empty", passphrase);
        }
        Ok(())
    }

    pub fn verify_api_key(&self, name: &str) -> eyre::Result<()> {
        let name = self.name(name);
        if self.api_key.is_empty() {
            bail!("{}: api_key is empty", name);
        }
        Ok(())
    }
    pub fn to_value(&self) -> serde_json::Value {
        serde_json::json!({
            "api_key": self.api_key.expose_secret().unwrap(),
            "api_secret": self.api_secret.expose_secret().unwrap(),
            "passphrase": self.passphrase.expose_secret().unwrap(),
        })
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct SigningAddressPrivateKey {
    pub env: Option<String>,
    #[serde(default)]
    pub address: String,
    #[serde(default)]
    pub private_key: PrivateKey,
}

impl SigningAddressPrivateKey {
    pub fn new(address: String, private_key: PrivateKey) -> Self {
        Self {
            env: None,
            address,
            private_key,
        }
    }
    pub fn empty() -> Self {
        Self {
            env: None,
            address: "".to_string(),
            private_key: Default::default(),
        }
    }
    fn name<'a>(&'a self, default_name: &'a str) -> &str {
        self.env.as_deref().unwrap_or(default_name)
    }
    pub fn try_load_from_env(&mut self, default_name: &str) -> eyre::Result<()> {
        let env = self.name(default_name);
        let address_name = format!("{}_ADDRESS", env);
        let private_key_name = format!("{}_PRIVATE_KEY", env);
        read_env_var(&mut self.address, &address_name)?;
        read_env_var_secret(&mut self.private_key, &private_key_name)?;
        Ok(())
    }
    pub fn verify(&self, name: &str) -> eyre::Result<()> {
        let name = self.name(name);
        if self.address.is_empty() {
            bail!("{}: address is empty", name);
        }
        if self.private_key.is_empty() {
            bail!("{}: private_key is empty", name);
        }
        Ok(())
    }
    pub fn verify_address(&self, name: &str) -> eyre::Result<()> {
        let name = self.name(name);
        if self.address.is_empty() {
            bail!("{}: address is empty", name);
        }
        Ok(())
    }
    pub fn to_value(&self) -> serde_json::Value {
        serde_json::json!({
            "address": self.address,
            "private_key": self.private_key.expose_secret().unwrap(),
        })
    }
}
