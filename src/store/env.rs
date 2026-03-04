use std::collections::HashMap;
use std::path::PathBuf;

use anyhow::{Context, bail};
use chrono::{DateTime, TimeZone, Utc};

use super::{SecretKind, SecretMeta, SecretStore, SecretValue};

pub struct EnvStore {
    path: PathBuf,
}

impl EnvStore {
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }

    fn secret_name(&self) -> String {
        self.path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| "env".to_string())
    }

    fn mtime_as_utc(&self) -> DateTime<Utc> {
        std::fs::metadata(&self.path)
            .and_then(|m| m.modified())
            .ok()
            .and_then(|t| {
                let dur = t.duration_since(std::time::UNIX_EPOCH).ok()?;
                Utc.timestamp_opt(dur.as_secs() as i64, dur.subsec_nanos()).single()
            })
            .unwrap_or_else(Utc::now)
    }

    fn parse_env(&self) -> anyhow::Result<HashMap<String, String>> {
        let data = std::fs::read_to_string(&self.path)
            .with_context(|| format!("could not read '{}'", self.path.display()))?;
        serde_envfile::from_str(&data)
            .with_context(|| format!("failed to parse env file '{}'", self.path.display()))
    }

    fn meta_from_map(&self, map: &HashMap<String, String>) -> SecretMeta {
        let keys: Vec<String> = {
            let mut k: Vec<_> = map.keys().cloned().collect();
            k.sort();
            k
        };
        let ts = self.mtime_as_utc();
        SecretMeta {
            name: self.secret_name(),
            kind: SecretKind::Structured { keys },
            created_at: ts,
            updated_at: ts,
        }
    }
}

impl SecretStore for EnvStore {
    fn default_name(&self) -> Option<String> {
        Some(self.secret_name())
    }

    async fn list(&self) -> anyhow::Result<Vec<SecretMeta>> {
        if !self.path.exists() {
            return Ok(vec![]);
        }
        let map = self.parse_env()?;
        Ok(vec![self.meta_from_map(&map)])
    }

    async fn describe(&self, name: &str) -> anyhow::Result<SecretMeta> {
        if !self.path.exists() {
            bail!("env file '{}' not found", self.path.display());
        }
        let expected = self.secret_name();
        if name != expected {
            bail!("secret '{}' not found (this env store manages '{}')", name, expected);
        }
        let map = self.parse_env()?;
        Ok(self.meta_from_map(&map))
    }

    async fn get_value(&self, name: &str) -> anyhow::Result<SecretValue> {
        let expected = self.secret_name();
        if name != expected {
            bail!("secret '{}' not found (this env store manages '{}')", name, expected);
        }
        let map = self.parse_env()?;
        let json = serde_json::to_string(&map)?;
        Ok(SecretValue::new(json))
    }

    async fn put_value(&self, name: &str, value: &SecretValue) -> anyhow::Result<()> {
        let expected = self.secret_name();
        if name != expected {
            bail!("secret '{}' not found (this env store manages '{}')", name, expected);
        }
        let map: HashMap<String, String> = serde_json::from_str(value.expose())
            .context("value must be a JSON object with string values")?;
        let env_str: String = serde_envfile::to_string(&map)
            .context("failed to serialize to env format")?;
        std::fs::write(&self.path, env_str)?;
        Ok(())
    }

    async fn create(&self, name: &str, _kind: SecretKind, value: &SecretValue) -> anyhow::Result<()> {
        if self.path.exists() {
            bail!("env file '{}' already exists", self.path.display());
        }
        let expected = self.secret_name();
        if name != expected {
            bail!("env store name is '{}', cannot create '{}'", expected, name);
        }
        // Write the initial value
        let map: HashMap<String, String> = serde_json::from_str(value.expose())
            .unwrap_or_default();
        let env_str: String = serde_envfile::to_string(&map)
            .context("failed to serialize to env format")?;
        if let Some(parent) = self.path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(&self.path, env_str)?;
        Ok(())
    }

    async fn delete(&self, name: &str) -> anyhow::Result<()> {
        let expected = self.secret_name();
        if name != expected {
            bail!("secret '{}' not found (this env store manages '{}')", name, expected);
        }
        if !self.path.exists() {
            bail!("env file '{}' not found", self.path.display());
        }
        std::fs::remove_file(&self.path)?;
        Ok(())
    }

}
