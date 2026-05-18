use std::path::PathBuf;

use anyhow::{Context, bail};
use chrono::Utc;
use serde::{Deserialize, Serialize};

use super::{SecretKind, SecretMeta, SecretStore, SecretValue};

#[derive(Serialize, Deserialize)]
struct SecretFile {
    meta: SecretMeta,
    value: String,
}

pub struct FileStore {
    base_dir: PathBuf,
}

impl FileStore {
    pub fn new(base_dir: impl Into<PathBuf>) -> Self {
        Self {
            base_dir: base_dir.into(),
        }
    }

    fn secret_path(&self, name: &str) -> PathBuf {
        self.base_dir.join(format!("{name}.json"))
    }

    fn read_file(&self, name: &str) -> anyhow::Result<SecretFile> {
        let path = self.secret_path(name);
        let data = std::fs::read_to_string(&path)
            .with_context(|| format!("secret '{}' not found", name))?;
        serde_json::from_str(&data)
            .with_context(|| format!("corrupt secret file: {}", path.display()))
    }

    fn write_file(&self, sf: &SecretFile) -> anyhow::Result<()> {
        std::fs::create_dir_all(&self.base_dir)?;
        let path = self.secret_path(&sf.meta.name);
        let data = serde_json::to_string_pretty(sf)?;
        std::fs::write(&path, data)?;
        Ok(())
    }
}

impl SecretStore for FileStore {
    async fn list(&self) -> anyhow::Result<Vec<SecretMeta>> {
        let dir = match std::fs::read_dir(&self.base_dir) {
            Ok(d) => d,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(vec![]),
            Err(e) => return Err(e.into()),
        };

        let mut metas = Vec::new();
        for entry in dir {
            let entry = entry?;
            let path = entry.path();
            if path.extension().is_some_and(|e| e == "json") {
                let data = std::fs::read_to_string(&path)?;
                let sf: SecretFile = serde_json::from_str(&data)?;
                metas.push(sf.meta);
            }
        }
        metas.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(metas)
    }

    async fn describe(&self, name: &str) -> anyhow::Result<SecretMeta> {
        Ok(self.read_file(name)?.meta)
    }

    async fn get_value(&self, name: &str) -> anyhow::Result<SecretValue> {
        Ok(SecretValue::new(self.read_file(name)?.value))
    }

    async fn put_value(&self, name: &str, value: &SecretValue) -> anyhow::Result<()> {
        let mut sf = self.read_file(name)?;
        sf.value = value.expose().to_owned();
        sf.meta.updated_at = Utc::now();
        self.write_file(&sf)
    }

    async fn create(
        &self,
        name: &str,
        kind: SecretKind,
        value: &SecretValue,
    ) -> anyhow::Result<()> {
        let path = self.secret_path(name);
        if path.exists() {
            bail!("secret '{}' already exists", name);
        }
        let now = Utc::now();
        let sf = SecretFile {
            meta: SecretMeta {
                name: name.to_owned(),
                kind,
                created_at: now,
                updated_at: now,
            },
            value: value.expose().to_owned(),
        };
        self.write_file(&sf)
    }

    async fn delete(&self, name: &str) -> anyhow::Result<()> {
        let path = self.secret_path(name);
        if !path.exists() {
            bail!("secret '{}' not found", name);
        }
        std::fs::remove_file(&path)?;
        Ok(())
    }
}
