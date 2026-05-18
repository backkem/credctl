use clap::{Parser, Subcommand, ValueEnum};
use clap_complete::Shell;

#[derive(Parser)]
#[command(
    name = "credctl",
    about = "Minimal secret manager",
    disable_help_subcommand = true
)]
pub struct Cli {
    /// Override the active context for this invocation
    #[arg(long, global = true)]
    pub context: Option<String>,

    /// Editor UI to use (default: gui). Requires --features tui for the tui option.
    #[arg(long, global = true, value_enum, default_value_t = EditorKind::Gui)]
    pub editor: EditorKind,

    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Clone, Copy, Debug, ValueEnum, PartialEq, Eq)]
pub enum EditorKind {
    Gui,
    Tui,
}

#[derive(Subcommand)]
pub enum Commands {
    /// List all secrets
    List,

    /// Show metadata for a secret (name, kind, keys, timestamps)
    Describe {
        /// Secret name (optional for single-secret stores like env)
        name: Option<String>,
    },

    /// Create a new secret
    Create {
        /// Secret name (optional for single-secret stores like env)
        name: Option<String>,
        /// Create as structured key-value secret
        #[arg(long)]
        structured: bool,
    },

    /// Edit a secret's value (opens editor UI)
    Edit {
        /// Secret name (optional for single-secret stores like env)
        name: Option<String>,
        /// Extra keys to show in the structured editor (comma-separated)
        #[arg(long, value_delimiter = ',')]
        keys: Vec<String>,
    },

    /// Delete a secret
    Delete {
        /// Secret name (optional for single-secret stores like env)
        name: Option<String>,
    },

    /// Manage contexts (store backends)
    Context {
        #[command(subcommand)]
        action: ContextAction,
    },

    /// Print shell completion script for the given shell (bash, zsh, fish, ...)
    Completions {
        /// Shell to generate completions for
        #[arg(value_enum)]
        shell: Shell,
    },

    /// Show help with all commands and examples
    Help,
}

#[derive(Subcommand)]
pub enum ContextAction {
    /// List all contexts
    List,
    /// Print the current context name
    Current,
    /// Switch to a different context
    Use {
        /// Context name
        name: String,
    },
    /// Create or update a context
    Set {
        /// Context name
        name: String,
        /// Store type: file, env, aws
        #[arg(long)]
        store: String,
        /// Path (for file and env stores)
        #[arg(long)]
        path: Option<String>,
        /// AWS CLI profile name (for aws store)
        #[arg(long)]
        profile: Option<String>,
    },
    /// Delete a context
    Delete {
        /// Context name
        name: String,
    },
}

pub fn print_help() {
    println!(
        "\
credctl — minimal secret manager

USAGE:
  credctl [--context <name>] <command> [args]

COMMANDS:
  list                          List all secrets
  describe [name]               Show metadata (kind, keys, timestamps)
  create [name] [--structured]  Create a new secret (plain text or --structured key-value)
  edit [name] [--keys K1,K2]    Open editor UI (--keys adds extra rows to structured editor)
  delete [name]                 Delete a secret (requires confirmation)

  Name is optional for single-secret stores (e.g. env).
  context list                  List all contexts (* = current)
  context current               Print current context name
  context use <name>            Switch to a context
  context set <name> --store <type> [--path <p>] [--profile <p>]
                                Create or update a context
  context delete <name>         Delete a context
  completions <shell>           Print shell completion script (bash, zsh, fish, ...)
  help                          Show this help text

GLOBAL FLAGS:
  --context <name>              Override the active context for this invocation
  --editor <gui|tui>            Editor UI (default: gui; tui requires --features tui)

EXAMPLES:
  credctl create my-secret
  credctl create db-creds --structured
  credctl edit db-creds --keys host,port,user,password
  credctl list
  credctl --context prod list
  credctl context set prod --store env --path .env.prod
  credctl context use prod

SHELL COMPLETIONS:
  bash:  credctl completions bash | sudo tee /etc/bash_completion.d/credctl
  zsh:   credctl completions zsh > \"${{fpath[1]}}/_credctl\""
    );
}
