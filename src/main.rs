mod cli;
mod config;
mod store;
mod ui;

use std::io::{BufRead, Write};

use anyhow::Result;
use clap::{CommandFactory, Parser};
use clap_complete::generate;
use cli::{Cli, Commands};
use config::StoreType;

#[tokio::main]
async fn main() -> Result<()> {
    // Suppress xilem/wgpu log noise unless the user explicitly sets RUST_LOG.
    if std::env::var_os("RUST_LOG").is_none() {
        // SAFETY: called before tokio spawns any threads.
        unsafe { std::env::set_var("RUST_LOG", "error") };
    }

    let cli = Cli::parse();

    match &cli.command {
        Commands::Help => {
            cli::print_help();
            return Ok(());
        }
        Commands::Completions { shell } => {
            let mut cmd = Cli::command();
            let name = cmd.get_name().to_string();
            generate(*shell, &mut cmd, name, &mut std::io::stdout());
            return Ok(());
        }
        Commands::Context { action } => {
            let mut cfg = config::load_config()?;
            config::handle_context_command(&mut cfg, action)?;
            return Ok(());
        }
        _ => {}
    }

    let cfg = config::load_config()?;
    let (_, store_type) = config::resolve_context(&cfg, cli.context.as_deref())?;

    match store_type.clone() {
        #[cfg(feature = "dev-store")]
        StoreType::File { path } => {
            let dir = path.unwrap_or_else(|| ".credctl/secrets".to_string());
            let s = store::file::FileStore::new(dir);
            run_with_store(&s, &ui::XilemEditor, cli.command).await
        }
        #[cfg(not(feature = "dev-store"))]
        StoreType::File { .. } => {
            anyhow::bail!("file store requires building with --features dev-store")
        }
        StoreType::Env { path } => {
            let p = path.unwrap_or_else(|| ".env".to_string());
            let s = store::env::EnvStore::new(p);
            run_with_store(&s, &ui::XilemEditor, cli.command).await
        }
        StoreType::Aws { profile } => {
            let s = store::aws::AwsStore::new(profile).await?;
            run_with_store(&s, &ui::XilemEditor, cli.command).await
        }
    }
}

/// If the value is a JSON object with all-string values, treat it as structured.
fn detect_kind(value: &str) -> store::SecretKind {
    if let Ok(serde_json::Value::Object(map)) = serde_json::from_str(value) {
        if map.values().all(|v| v.is_string()) {
            let mut keys: Vec<String> = map.keys().cloned().collect();
            keys.sort();
            return store::SecretKind::Structured { keys };
        }
    }
    store::SecretKind::PlainText
}

async fn run_with_store(
    store: &impl store::SecretStore,
    editor: &impl ui::SecretEditor,
    cmd: Commands,
) -> Result<()> {
    use store::SecretKind;

    // Resolve optional name against store's default (for single-secret stores like env).
    let resolve = |name: Option<String>| -> Result<String> {
        name.or_else(|| store.default_name())
            .ok_or_else(|| anyhow::anyhow!("secret name is required for this store"))
    };

    match cmd {
        Commands::List => {
            let metas = store.list().await?;
            if metas.is_empty() {
                println!("no secrets");
            } else {
                for m in &metas {
                    println!("{}", m.name);
                }
            }
        }

        Commands::Describe { name } => {
            let name = resolve(name)?;
            let meta = store.describe(&name).await?;
            let kind = match &meta.kind {
                SecretKind::PlainText => {
                    let val = store.get_value(&name).await?;
                    detect_kind(val.expose())
                }
                other => other.clone(),
            };
            println!("name:       {}", meta.name);
            match &kind {
                SecretKind::PlainText => println!("kind:       plain-text"),
                SecretKind::Structured { keys } => {
                    println!("kind:       structured");
                    if keys.is_empty() {
                        println!("keys:       (none)");
                    } else {
                        println!("keys:       {}", keys.join(", "));
                    }
                }
            }
            println!("created_at: {}", meta.created_at);
            println!("updated_at: {}", meta.updated_at);
        }

        Commands::Create { name, structured } => {
            let name = resolve(name)?;
            let kind = if structured {
                SecretKind::Structured { keys: vec![] }
            } else {
                SecretKind::PlainText
            };
            match editor.edit(&kind, None) {
                Ok(value) => {
                    store.create(&name, kind, &value).await?;
                    println!("created secret '{}'", name);
                }
                Err(e) if e.is::<ui::Cancelled>() => {
                    println!("user cancelled operation");
                }
                Err(e) => return Err(e),
            }
        }

        Commands::Edit { name, keys } => {
            let name = resolve(name)?;
            let meta = store.describe(&name).await?;
            let current = store.get_value(&name).await?;
            // Auto-detect structured secrets: if marked PlainText but the value
            // is a JSON object with string values, open as structured instead.
            let mut kind = match &meta.kind {
                SecretKind::PlainText => detect_kind(current.expose()),
                other => other.clone(),
            };
            // --keys forces structured mode and merges extra keys.
            if !keys.is_empty() {
                let existing = match &kind {
                    SecretKind::Structured { keys } => keys.clone(),
                    SecretKind::PlainText => vec![],
                };
                let mut merged = existing;
                for k in keys {
                    if !merged.contains(&k) {
                        merged.push(k);
                    }
                }
                kind = SecretKind::Structured { keys: merged };
            }
            match editor.edit(&kind, Some(current)) {
                Ok(value) => {
                    store.put_value(&name, &value).await?;
                    println!("updated secret '{}'", name);
                }
                Err(e) if e.is::<ui::Cancelled>() => {
                    println!("user cancelled operation");
                }
                Err(e) => return Err(e),
            }
        }

        Commands::Delete { name } => {
            let name = resolve(name)?;
            print!("Type '{}' to confirm deletion: ", name);
            std::io::stdout().flush()?;
            let line = std::io::stdin()
                .lock()
                .lines()
                .next()
                .unwrap_or_else(|| Ok(String::new()))?;
            if line.trim() != name {
                anyhow::bail!("confirmation did not match, aborting");
            }
            store.delete(&name).await?;
            println!("deleted secret '{}'", name);
        }

        Commands::Help | Commands::Context { .. } | Commands::Completions { .. } => unreachable!(),
    }

    Ok(())
}
