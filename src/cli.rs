use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "credctl", about = "Minimal secret manager", disable_help_subcommand = true)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// List all secrets
    List,

    /// Show metadata for a secret
    Describe {
        /// Secret name
        name: String,
    },

    /// Create a new secret
    Create {
        /// Secret name
        name: String,
        /// Create as structured JSON secret
        #[arg(long)]
        json: bool,
    },

    /// Edit a secret's value (opens editor UI)
    Edit {
        /// Secret name
        name: String,
    },

    /// Delete a secret
    Delete {
        /// Secret name
        name: String,
    },

    /// View or update a secret's schema
    Schema {
        /// Secret name
        name: String,
        /// Set schema keys (JSON array of strings, e.g. '["user","pass"]')
        #[arg(long)]
        set: Option<String>,
    },

    /// Show help with all commands and examples
    Help,
}

pub fn print_help() {
    println!(
        "\
credctl — minimal secret manager

USAGE:
  credctl <command> [args]

COMMANDS:
  list                          List all secrets (name, kind)
  describe <name>               Show metadata and schema for a secret
  create <name> [--json]        Create a new secret (plain text by default, --json for structured)
  edit <name>                   Open editor UI to modify a secret's value
  delete <name>                 Delete a secret
  schema <name>                 Show the schema for a secret
  schema <name> --set <json>    Set schema keys (JSON array: '[\"user\",\"pass\"]')
  help                          Show this help text

EXAMPLES:
  credctl create my-secret
  credctl create db-creds --json
  credctl schema db-creds --set '[\"host\",\"port\",\"user\",\"password\"]'
  credctl list
  credctl describe my-secret
  credctl edit my-secret
  credctl delete my-secret"
    );
}
