# credctl

A minimal secret manager: CLI for schema/metadata, GUI for editing secret values.

## Architecture

- **CLI** (`credctl`): list/describe secrets, manage contexts. Never prints secret values.
- **Editor UI** (xilem): transient window for editing secret values. Values stay in memory only.
- **SecretStore trait**: pluggable backend. Implementations: file store, .env store, AWS Secrets Manager.
- **SecretEditor trait**: pluggable UI for editing values. Default: xilem window.
- **Context system**: kubectl-style context switcher for multiple store backends.

## Key invariant

**Secret values are NEVER printed to stdout/stderr, logged, or written to disk** (except by the file store for local dev). They exist only in process memory during editing and are transmitted directly to the secret store backend.

## Project structure

```
src/
  main.rs            - CLI entrypoint, config loading, context dispatch
  cli.rs             - Clap derive structs + help text + context subcommand
  config.rs          - Config structs (contexts, store types), load/save
  store/
    mod.rs           - SecretValue (zeroize), SecretKind, SecretMeta, SecretStore trait
    file.rs          - Local file store (JSON files under a directory)
    env.rs           - .env file store (one file = one structured secret)
    aws.rs           - AWS Secrets Manager store
  ui/
    mod.rs           - SecretEditor trait (kind + optional current value -> Result<SecretValue>)
    xilem_editor.rs  - Xilem-based editor: dark theme, plain text + structured key-value table
docs/
  requirements.md    - detailed requirements
  xilem.md           - xilem integration notes, patterns, and gotchas
  xilem-widgets.md   - xilem widget/style API reference
```

## Build & run

```sh
cargo build
cargo run -- help
cargo run -- create my-secret
cargo run -- create db-creds --structured
cargo run -- edit db-creds --keys host,port,user,password
cargo run -- list
```

## Context management

```sh
credctl context list                                          # list all (* = current)
credctl context set prod --store env --path .env.prod         # add env context
credctl context set staging --store aws --profile my-profile  # add AWS context
credctl context use prod                                      # switch
credctl context current                                       # print current
credctl --context staging list                                # one-off override
```

Config stored at `~/.credctl/config.json`. Default context: `env` store at `.env`.

## Store backends

- **env**: Single .env file = one structured secret. Default for `default` context.
- **aws**: AWS Secrets Manager. Kind metadata stored as tags (`credctl:kind`).
- **file**: Local JSON files in a directory. Behind `dev-store` feature flag, for development only.

## Xilem dependency

Uses a **git dependency** from `github.com/linebender/xilem` (latest main). The `AppState::keep_running()` check that allows programmatic window close is upstream.

## Exploring the xilem API

See `docs/xilem-widgets.md` for quick reference.

Key xilem patterns in our code:
- `Xilem::new` (not `new_simple`) with custom `AppState` for programmatic exit
- `Arc<Mutex<Option<String>>>` to pass result out of the consumed xilem state
- `tokio::task::block_in_place` in `XilemEditor::edit()` to avoid nested-runtime panic
- `Either<A, B>` from `xilem::core::one_of` for conditional rendering (masked/unmasked, etc.)
- Widget-specific methods (text_color, text_size, placeholder) must be chained BEFORE Style trait methods (border, background_color, corner_radius)

## Workflow

- When discovering new xilem patterns or gotchas, document them in `docs/xilem.md`

## Dependencies

- clap — CLI parsing
- serde / serde_json — serialization
- serde-envfile — .env file serialization
- chrono — timestamps
- tokio — async runtime
- zeroize — secure memory clearing
- anyhow — error handling
- dirs — home directory resolution
- aws-config / aws-sdk-secretsmanager — AWS Secrets Manager
- xilem (git dep) — GUI toolkit
