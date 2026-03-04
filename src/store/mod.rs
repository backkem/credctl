use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use zeroize::Zeroize;

// -- SecretValue newtype --

/// Wraps a secret string. Zeroized on drop, never printed.
pub struct SecretValue {
    inner: String,
}

impl SecretValue {
    pub fn new(s: String) -> Self {
        Self { inner: s }
    }

    pub fn expose(&self) -> &str {
        &self.inner
    }
}

impl Drop for SecretValue {
    fn drop(&mut self) {
        self.inner.zeroize();
    }
}

impl std::fmt::Debug for SecretValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("SecretValue(****)")
    }
}

// -- Secret schema types --

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum SecretKind {
    PlainText,
    #[serde(alias = "Json")]
    Structured { keys: Vec<String> },
}

impl std::fmt::Display for SecretKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SecretKind::PlainText => write!(f, "plain-text"),
            SecretKind::Structured { keys } => write!(f, "structured({})", keys.join(", ")),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecretMeta {
    pub name: String,
    pub kind: SecretKind,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

// -- SecretStore trait --

pub trait SecretStore {
    /// If the store manages a single implicit secret, return its name.
    fn default_name(&self) -> Option<String> { None }
    fn list(&self) -> impl std::future::Future<Output = Result<Vec<SecretMeta>>> + Send;
    fn describe(&self, name: &str) -> impl std::future::Future<Output = Result<SecretMeta>> + Send;
    fn get_value(&self, name: &str) -> impl std::future::Future<Output = Result<SecretValue>> + Send;
    fn put_value(&self, name: &str, value: &SecretValue) -> impl std::future::Future<Output = Result<()>> + Send;
    fn create(&self, name: &str, kind: SecretKind, value: &SecretValue) -> impl std::future::Future<Output = Result<()>> + Send;
    fn delete(&self, name: &str) -> impl std::future::Future<Output = Result<()>> + Send;
}

pub mod aws;
pub mod env;
#[cfg(feature = "dev-store")]
pub mod file;
