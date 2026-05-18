use anyhow::Context;
use aws_sdk_secretsmanager::Client;
use chrono::{DateTime, Utc};

use super::{SecretKind, SecretMeta, SecretStore, SecretValue};

pub struct AwsStore {
    client: Client,
}

impl AwsStore {
    pub async fn new(profile: Option<String>) -> anyhow::Result<Self> {
        let mut loader = aws_config::defaults(aws_config::BehaviorVersion::latest());
        if let Some(profile) = profile {
            loader = loader.profile_name(profile);
        }
        let config = loader.load().await;
        Ok(Self {
            client: Client::new(&config),
        })
    }

    fn parse_tags(&self, tags: &[aws_sdk_secretsmanager::types::Tag]) -> SecretKind {
        let mut kind_str = None;
        let mut schema_str = None;
        for tag in tags {
            match tag.key() {
                Some("credctl:kind") => kind_str = tag.value(),
                Some("credctl:schema") => schema_str = tag.value(),
                _ => {}
            }
        }
        match kind_str {
            Some("structured") => {
                let keys: Vec<String> = schema_str
                    .and_then(|s| serde_json::from_str(s).ok())
                    .unwrap_or_default();
                SecretKind::Structured { keys }
            }
            _ => SecretKind::PlainText,
        }
    }

    fn aws_ts_to_chrono(ts: &aws_sdk_secretsmanager::primitives::DateTime) -> DateTime<Utc> {
        DateTime::from_timestamp(ts.secs(), ts.subsec_nanos()).unwrap_or_else(Utc::now)
    }

    fn kind_tag_value(kind: &SecretKind) -> &'static str {
        match kind {
            SecretKind::PlainText => "plaintext",
            SecretKind::Structured { .. } => "structured",
        }
    }

    fn schema_tag_value(kind: &SecretKind) -> Option<String> {
        match kind {
            SecretKind::Structured { keys } if !keys.is_empty() => {
                Some(serde_json::to_string(keys).unwrap())
            }
            _ => None,
        }
    }
}

impl SecretStore for AwsStore {
    async fn list(&self) -> anyhow::Result<Vec<SecretMeta>> {
        let mut metas = Vec::new();
        let mut next_token: Option<String> = None;

        loop {
            let mut req = self.client.list_secrets();
            if let Some(token) = next_token.take() {
                req = req.next_token(token);
            }
            let resp = req.send().await.context("failed to list AWS secrets")?;

            for entry in resp.secret_list() {
                let name = entry.name().unwrap_or_default().to_string();
                let tags = entry.tags();
                let kind = self.parse_tags(tags);

                let created_at = entry
                    .created_date()
                    .map(Self::aws_ts_to_chrono)
                    .unwrap_or_else(Utc::now);
                let updated_at = entry
                    .last_changed_date()
                    .map(Self::aws_ts_to_chrono)
                    .unwrap_or(created_at);

                metas.push(SecretMeta {
                    name,
                    kind,
                    created_at,
                    updated_at,
                });
            }

            next_token = resp.next_token().map(String::from);
            if next_token.is_none() {
                break;
            }
        }

        metas.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(metas)
    }

    async fn describe(&self, name: &str) -> anyhow::Result<SecretMeta> {
        let resp = self
            .client
            .describe_secret()
            .secret_id(name)
            .send()
            .await
            .with_context(|| format!("failed to describe secret '{}'", name))?;

        let tags = resp.tags();
        let kind = self.parse_tags(tags);

        let created_at = resp
            .created_date()
            .map(Self::aws_ts_to_chrono)
            .unwrap_or_else(Utc::now);
        let updated_at = resp
            .last_changed_date()
            .map(Self::aws_ts_to_chrono)
            .unwrap_or(created_at);

        Ok(SecretMeta {
            name: name.to_string(),
            kind,
            created_at,
            updated_at,
        })
    }

    async fn get_value(&self, name: &str) -> anyhow::Result<SecretValue> {
        let resp = match self.client.get_secret_value().secret_id(name).send().await {
            Ok(r) => r,
            Err(e) => {
                // Secret exists but has no value version yet — treat as empty.
                if e.as_service_error()
                    .is_some_and(|se| se.is_resource_not_found_exception())
                {
                    return Ok(SecretValue::new(String::new()));
                }
                return Err(e)
                    .with_context(|| format!("failed to get value for secret '{}'", name));
            }
        };

        let value = resp.secret_string().ok_or_else(|| {
            anyhow::anyhow!(
                "secret '{}' has no string value (binary secrets not supported)",
                name
            )
        })?;

        Ok(SecretValue::new(value.to_string()))
    }

    async fn put_value(&self, name: &str, value: &SecretValue) -> anyhow::Result<()> {
        self.client
            .put_secret_value()
            .secret_id(name)
            .secret_string(value.expose())
            .send()
            .await
            .with_context(|| format!("failed to put value for secret '{}'", name))?;
        Ok(())
    }

    async fn create(
        &self,
        name: &str,
        kind: SecretKind,
        value: &SecretValue,
    ) -> anyhow::Result<()> {
        let mut req = self
            .client
            .create_secret()
            .name(name)
            .secret_string(value.expose());

        // Add tags for kind metadata
        req = req.tags(
            aws_sdk_secretsmanager::types::Tag::builder()
                .key("credctl:kind")
                .value(Self::kind_tag_value(&kind))
                .build(),
        );
        if let Some(schema) = Self::schema_tag_value(&kind) {
            req = req.tags(
                aws_sdk_secretsmanager::types::Tag::builder()
                    .key("credctl:schema")
                    .value(schema)
                    .build(),
            );
        }

        req.send()
            .await
            .with_context(|| format!("failed to create secret '{}'", name))?;
        Ok(())
    }

    async fn delete(&self, name: &str) -> anyhow::Result<()> {
        self.client
            .delete_secret()
            .secret_id(name)
            .force_delete_without_recovery(true)
            .send()
            .await
            .with_context(|| format!("failed to delete secret '{}'", name))?;
        Ok(())
    }
}
