mod cli;
mod store;
mod ui;

use anyhow::Result;
use clap::Parser;
use cli::{Cli, Commands};

#[cfg(feature = "dev-store")]
fn make_store() -> store::file::FileStore {
    store::file::FileStore::new(store::file::FileStore::default_dir())
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Help => {
            cli::print_help();
        }

        #[cfg(feature = "dev-store")]
        _ => run_with_store(&make_store(), &ui::XilemEditor, cli.command).await?,

        #[cfg(not(feature = "dev-store"))]
        _ => {
            anyhow::bail!("no secret store configured. Build with --features dev-store for local file store.");
        }
    }

    Ok(())
}

#[cfg(feature = "dev-store")]
async fn run_with_store(
    store: &impl store::SecretStore,
    editor: &impl ui::SecretEditor,
    cmd: Commands,
) -> Result<()> {
    use store::SecretKind;

    match cmd {
        Commands::List => {
            let metas = store.list().await?;
            if metas.is_empty() {
                println!("no secrets");
            } else {
                for m in &metas {
                    println!("{:<30} {}", m.name, m.kind);
                }
            }
        }

        Commands::Describe { name } => {
            let meta = store.describe(&name).await?;
            println!("name:       {}", meta.name);
            println!("kind:       {}", meta.kind);
            println!("created_at: {}", meta.created_at);
            println!("updated_at: {}", meta.updated_at);
        }

        Commands::Create { name, json } => {
            let kind = if json {
                SecretKind::Json { keys: vec![] }
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

        Commands::Edit { name } => {
            let meta = store.describe(&name).await?;
            let current = store.get_value(&name).await?;
            match editor.edit(&meta.kind, Some(current)) {
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
            store.delete(&name).await?;
            println!("deleted secret '{}'", name);
        }

        Commands::Schema { name, set } => {
            if let Some(json_str) = set {
                let keys: Vec<String> = serde_json::from_str(&json_str)
                    .map_err(|e| anyhow::anyhow!("invalid JSON array: {}", e))?;
                store
                    .set_schema(&name, SecretKind::Json { keys })
                    .await?;
                println!("updated schema for '{}'", name);
            } else {
                let meta = store.describe(&name).await?;
                match &meta.kind {
                    SecretKind::PlainText => println!("plain-text (no schema)"),
                    SecretKind::Json { keys } => {
                        if keys.is_empty() {
                            println!("json (no keys defined)");
                        } else {
                            for k in keys {
                                println!("  - {}", k);
                            }
                        }
                    }
                }
            }
        }

        Commands::Help => unreachable!(),
    }

    Ok(())
}
