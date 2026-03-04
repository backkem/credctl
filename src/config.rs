use std::collections::HashMap;
use std::path::PathBuf;

use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    pub current_context: String,
    pub contexts: HashMap<String, StoreType>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "store", rename_all = "lowercase")]
pub enum StoreType {
    File { path: Option<String> },
    Env { path: Option<String> },
    Aws { profile: Option<String> },
}

impl std::fmt::Display for StoreType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StoreType::File { path } => {
                write!(f, "file")?;
                if let Some(p) = path {
                    write!(f, " ({})", p)?;
                }
                Ok(())
            }
            StoreType::Env { path } => {
                write!(f, "env")?;
                if let Some(p) = path {
                    write!(f, " ({})", p)?;
                }
                Ok(())
            }
            StoreType::Aws { profile } => {
                write!(f, "aws")?;
                if let Some(p) = profile {
                    write!(f, " (profile: {})", p)?;
                }
                Ok(())
            }
        }
    }
}

impl Default for Config {
    fn default() -> Self {
        let mut contexts = HashMap::new();
        contexts.insert("default".to_string(), StoreType::Env { path: None });
        Config {
            current_context: "default".to_string(),
            contexts,
        }
    }
}

fn config_path() -> Result<PathBuf> {
    let home = dirs::home_dir().context("could not determine home directory")?;
    Ok(home.join(".credctl").join("config.json"))
}

pub fn load_config() -> Result<Config> {
    let path = config_path()?;
    if !path.exists() {
        return Ok(Config::default());
    }
    let data = std::fs::read_to_string(&path)
        .with_context(|| format!("failed to read config: {}", path.display()))?;
    serde_json::from_str(&data)
        .with_context(|| format!("failed to parse config: {}", path.display()))
}

pub fn save_config(config: &Config) -> Result<()> {
    let path = config_path()?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let data = serde_json::to_string_pretty(config)?;
    std::fs::write(&path, data)?;
    Ok(())
}

pub fn resolve_context<'a>(config: &'a Config, override_name: Option<&'a str>) -> Result<(&'a str, &'a StoreType)> {
    let name = override_name.unwrap_or(&config.current_context);
    let store_type = config.contexts.get(name)
        .ok_or_else(|| anyhow::anyhow!("context '{}' not found", name))?;
    Ok((name, store_type))
}

pub fn handle_context_command(config: &mut Config, action: &crate::cli::ContextAction) -> Result<()> {
    match action {
        crate::cli::ContextAction::List => {
            if config.contexts.is_empty() {
                println!("no contexts configured");
            } else {
                let mut names: Vec<_> = config.contexts.keys().collect();
                names.sort();
                for name in names {
                    let marker = if name == &config.current_context { " *" } else { "" };
                    let store = &config.contexts[name];
                    println!("{:<20} {}{}", name, store, marker);
                }
            }
        }

        crate::cli::ContextAction::Current => {
            println!("{}", config.current_context);
        }

        crate::cli::ContextAction::Use { name } => {
            if !config.contexts.contains_key(name) {
                bail!("context '{}' not found", name);
            }
            config.current_context = name.clone();
            save_config(config)?;
            println!("switched to context '{}'", name);
        }

        crate::cli::ContextAction::Set { name, store, path, profile } => {
            let store_type = match store.as_str() {
                "file" => StoreType::File { path: path.clone() },
                "env" => StoreType::Env { path: path.clone() },
                "aws" => StoreType::Aws { profile: profile.clone() },
                other => bail!("unknown store type '{}' (expected: file, env, aws)", other),
            };
            config.contexts.insert(name.clone(), store_type);
            save_config(config)?;
            println!("context '{}' configured", name);
        }

        crate::cli::ContextAction::Delete { name } => {
            if name == &config.current_context {
                bail!("cannot delete the current context '{}'. Switch first.", name);
            }
            if config.contexts.remove(name).is_none() {
                bail!("context '{}' not found", name);
            }
            save_config(config)?;
            println!("deleted context '{}'", name);
        }
    }
    Ok(())
}
