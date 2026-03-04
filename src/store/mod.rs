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
    Json { keys: Vec<String> },
}

impl std::fmt::Display for SecretKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SecretKind::PlainText => write!(f, "plain-text"),
            SecretKind::Json { keys } => write!(f, "json({})", keys.join(", ")),
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
    fn list(&self) -> impl std::future::Future<Output = Result<Vec<SecretMeta>>> + Send;
    fn describe(&self, name: &str) -> impl std::future::Future<Output = Result<SecretMeta>> + Send;
    fn get_value(&self, name: &str) -> impl std::future::Future<Output = Result<SecretValue>> + Send;
    fn put_value(&self, name: &str, value: &SecretValue) -> impl std::future::Future<Output = Result<()>> + Send;
    fn create(&self, name: &str, kind: SecretKind, value: &SecretValue) -> impl std::future::Future<Output = Result<()>> + Send;
    fn delete(&self, name: &str) -> impl std::future::Future<Output = Result<()>> + Send;
    fn set_schema(&self, name: &str, kind: SecretKind) -> impl std::future::Future<Output = Result<()>> + Send;
}

#[cfg(feature = "dev-store")]
pub mod file;
