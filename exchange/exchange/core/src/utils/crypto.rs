use aes_gcm::aead::Aead;
use aes_gcm::{AeadCore, Aes256Gcm, Key, KeyInit};
use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;
use base64::Engine;
use dashmap::mapref::entry::Entry;
use dashmap::DashMap;
use eyre::{eyre, Context, ContextCompat, Error, Result};
use rand::rngs::OsRng;
pub use secrecy::ExposeSecret;
pub use secrecy::SecretBytesMut;
pub use secrecy::SecretString;
use serde_with_macros::DeserializeFromStr;
use std::str::FromStr;
use std::sync::OnceLock;

#[derive(Default, Debug, Clone)]
pub enum PrivateKeyKind {
    #[default]
    None,
    String,
    Bytes,
    Hex,
    Base64,
    Aes256,
    Ed25519,
    Rsa,
    Ecdsa,
}
#[derive(Default, Debug, Clone, DeserializeFromStr)]
pub struct PrivateKey {
    /// raw string
    string: Option<SecretString>,
    /// decoded bytes
    decoded: Option<SecretBytesMut>,
    /// decrypted bytes
    decrypted: Option<SecretBytesMut>,
    kind: PrivateKeyKind,
}
impl FromStr for PrivateKey {
    type Err = Error;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        PrivateKey::new(
            s,
            PrivateKeyOptions {
                file: true,
                encryption: true,
            },
        )
    }
}
#[derive(Debug, Clone, Default)]
pub struct PrivateKeyOptions {
    pub file: bool,
    pub encryption: bool,
}
impl PrivateKeyOptions {
    pub const ALL: Self = Self {
        file: true,
        encryption: true,
    };
    pub const NONE: Self = Self {
        file: false,
        encryption: false,
    };
}
impl PrivateKey {
    pub fn new(s: impl Into<String>, opts: PrivateKeyOptions) -> Result<Self> {
        let string = s.into();

        if string.starts_with("base64:") {
            let bytes = BASE64_STANDARD.decode(&string[7..]).context("Invalid base64 string")?;
            return Ok(PrivateKey {
                string: Some(SecretString::new(string)),
                decoded: Some(SecretBytesMut::new(&*bytes)),
                decrypted: None,
                kind: PrivateKeyKind::Base64,
            });
        }
        if let Ok(bytes) = BASE64_STANDARD.decode(&string) {
            return Ok(PrivateKey {
                string: Some(SecretString::new(string)),
                decoded: Some(SecretBytesMut::new(&*bytes)),
                decrypted: None,
                kind: PrivateKeyKind::Base64,
            });
        }
        if string.starts_with("0x") {
            let bytes = hex::decode(&string[2..]).context("Invalid hex string")?;
            return Ok(PrivateKey {
                string: Some(SecretString::new(string)),
                decoded: Some(SecretBytesMut::new(&*bytes)),
                decrypted: None,
                kind: PrivateKeyKind::Hex,
            });
        }
        if let Ok(bytes) = hex::decode(&string) {
            return Ok(PrivateKey {
                string: Some(SecretString::new(string)),
                decoded: Some(SecretBytesMut::new(&*bytes)),
                decrypted: None,
                kind: PrivateKeyKind::Hex,
            });
        }
        if opts.encryption {
            if string.starts_with("aes256:") {
                let bytes = PrivateKey::new(&string[7..], PrivateKeyOptions { file: false, ..opts })?;
                return Ok(PrivateKey {
                    string: Some(SecretString::new(string)),
                    decoded: bytes.as_plain_bytes().cloned(),
                    decrypted: None,
                    kind: PrivateKeyKind::Aes256,
                });
            }
            if string.starts_with("ed25519:") {
                let bytes = PrivateKey::new(&string[8..], PrivateKeyOptions { file: false, ..opts })?;
                return Ok(PrivateKey {
                    string: Some(SecretString::new(string)),
                    decoded: bytes.as_plain_bytes().cloned(),
                    decrypted: None,
                    kind: PrivateKeyKind::Ed25519,
                });
            }

            if string.starts_with("rsa:") {
                let bytes = PrivateKey::new(&string[4..], PrivateKeyOptions { file: false, ..opts })?;
                return Ok(PrivateKey {
                    string: Some(SecretString::new(string)),
                    decoded: bytes.as_plain_bytes().cloned(),
                    decrypted: None,
                    kind: PrivateKeyKind::Rsa,
                });
            }
            if string.starts_with("ecdsa:") {
                let bytes = PrivateKey::new(&string[6..], PrivateKeyOptions { file: false, ..opts })?;
                return Ok(PrivateKey {
                    string: Some(SecretString::new(string)),
                    decoded: bytes.as_plain_bytes().cloned(),
                    decrypted: None,
                    kind: PrivateKeyKind::Ecdsa,
                });
            }
        }
        if opts.file {
            if string.starts_with("file:") {
                let path = &string[5..];
                let bytes = std::fs::read(path)?;
                return Ok(PrivateKey {
                    string: Some(SecretString::new(string)),
                    decoded: Some(SecretBytesMut::new(&*bytes)),
                    decrypted: None,
                    kind: PrivateKeyKind::Bytes,
                });
            }
        }

        Ok(PrivateKey {
            decoded: Some(SecretBytesMut::new(string.as_bytes())),
            string: Some(SecretString::new(string)),
            decrypted: None,
            kind: PrivateKeyKind::String,
        })
    }
    pub fn is_empty(&self) -> bool {
        self.string
            .as_ref()
            .map(|s| s.expose_secret().is_empty())
            .unwrap_or(true)
            && self
                .decoded
                .as_ref()
                .map(|s| s.expose_secret().is_empty())
                .unwrap_or(true)
            && self
                .decrypted
                .as_ref()
                .map(|s| s.expose_secret().is_empty())
                .unwrap_or(true)
    }
    pub fn raw_string(&self) -> Option<&SecretString> {
        self.string.as_ref()
    }
    pub fn expose_secret(&self) -> Option<&str> {
        let s = self.string.as_ref()?;
        let s = s.expose_secret();
        match s.find(':') {
            Some(i) => Some(&s[i + 1..]),
            None => Some(s.as_str()),
        }
    }

    pub fn as_plain_bytes(&self) -> Option<&SecretBytesMut> {
        match self.kind {
            PrivateKeyKind::String => self.decoded.as_ref(),
            PrivateKeyKind::Bytes => self.decoded.as_ref(),
            PrivateKeyKind::Hex => self.decoded.as_ref(),
            PrivateKeyKind::Base64 => self.decoded.as_ref(),
            _ => self.decrypted.as_ref(),
        }
    }

    pub fn decrypt_bytes(&mut self, key: &PrivateKey) -> Result<&SecretBytesMut> {
        if let Some(s) = self.as_plain_bytes() {
            // safety: to workaround lifetime issue by rustc
            let s: &'static SecretBytesMut = unsafe { std::mem::transmute(s) };
            return Ok(s);
        }
        let key = key
            .as_plain_bytes()
            .with_context(|| format!("Password must be unencrypted: {:?}", key))?;
        match self.kind {
            PrivateKeyKind::Aes256 => {
                let key = key.expose_secret();
                let s = self.decoded.as_ref().unwrap().expose_secret();
                let data = decrypt_aes256(s, key)?;
                self.decrypted = Some(SecretBytesMut::new(&*data));
                Ok(self.decrypted.as_ref().unwrap())
            }
            _ => panic!("Not a byte array: {:?}", self),
        }
    }
    pub fn decrypt_string(&mut self, key: &PrivateKey) -> Result<&str> {
        let bytes = self.decrypt_bytes(key)?;
        let s = std::str::from_utf8(bytes.expose_secret())?;
        Ok(s)
    }
    pub fn maybe_decrypt_bytes(&mut self, password: &str) -> Result<&SecretBytesMut> {
        if let Some(s) = self.as_plain_bytes() {
            // safety: to workaround lifetime issue by rustc
            let s: &'static SecretBytesMut = unsafe { std::mem::transmute(s) };
            return Ok(s);
        }
        let key = read_password_from_stdin(password)?;
        self.decrypt_bytes(&key)
    }
    pub fn maybe_decrypt_string(&mut self, password: &str) -> Result<&str> {
        let bytes = self.maybe_decrypt_bytes(password)?;
        let s = std::str::from_utf8(bytes.expose_secret())?;
        Ok(s)
    }
}
static PASSWORD_CACHE: OnceLock<DashMap<String, PrivateKey>> = OnceLock::new();
pub fn read_password_from_stdin(key: &str) -> Result<PrivateKey> {
    let cache = PASSWORD_CACHE.get_or_init(|| DashMap::new());

    match cache.entry(key.to_string()) {
        Entry::Occupied(entry) => Ok(entry.get().clone()),
        Entry::Vacant(entry) => {
            let prompt = format!("Enter password for {}", key);
            let password = PrivateKey::new(
                rpassword::prompt_password(prompt)?,
                PrivateKeyOptions {
                    file: false,
                    encryption: false,
                },
            )?;

            entry.insert(password.clone());
            Ok(password)
        }
    }
}
pub fn decrypt_aes256(data: &[u8], key: &[u8]) -> Result<Vec<u8>> {
    // Transformed from a byte array:
    let key: [u8; 32] = key.try_into()?;
    let key = Key::<Aes256Gcm>::from_slice(&key);
    let cipher = Aes256Gcm::new(&key);
    let nonce = Aes256Gcm::generate_nonce(&mut OsRng); // 96-bits; unique per message
    let plaintext = cipher
        .decrypt(&nonce, data)
        .map_err(|_| eyre!("Failed to decrypt data with AES256-GCM"))?;
    Ok(plaintext.to_vec())
}
