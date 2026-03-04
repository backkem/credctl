# credctl

A minimal secret manager: CLI for schema/metadata, GUI for editing secret values.

## Architecture

- **CLI** (`credctl`): manage secret schemas, list/describe secrets. Never prints secret values.
- **Editor UI** (xilem): transient window for editing secret values. Values stay in memory only.
- **SecretStore trait**: pluggable backend. Implementations: local file store (dev).
- **SecretEditor trait**: pluggable UI for editing values. Default: xilem window.

## Key invariant

**Secret values are NEVER printed to stdout/stderr, logged, or written to disk** (except by dev-store for testing). They exist only in process memory during editing and are transmitted directly to the secret store backend.

## Project structure

```
src/
  main.rs            - CLI entrypoint, async dispatch to store
  cli.rs             - Clap derive structs + help text
  store/
    mod.rs           - SecretValue (zeroize), SecretKind, SecretMeta, SecretStore trait
    file.rs          - Local file store (behind dev-store feature)
  ui/
    mod.rs           - SecretEditor trait (kind + optional current value -> Result<SecretValue>)
    xilem_editor.rs  - Xilem-based editor: dark theme, plain text + JSON key-value table
docs/
  requirements.md    - detailed requirements
  xilem.md           - xilem integration notes, patterns, and gotchas
  xilem-widgets.md   - xilem widget/style API reference
```

## Build & run

```sh
cargo build --features dev-store
cargo run --features dev-store -- help
cargo run --features dev-store -- create my-secret
cargo run --features dev-store -- create db-creds --json
cargo run --features dev-store -- schema db-creds --set '["host","port","user","password"]'
cargo run --features dev-store -- edit db-creds
cargo run --features dev-store -- list
```

## Feature flags

- `dev-store`: enables `FileStore` — local JSON files under `.credctl/secrets/`. Dev/testing only.

## Xilem dependency

Uses a **git dependency** from `github.com/backkem/xilem` branch `keep-running-after-action`.

That branch carries a small patch in `xilem/src/driver.rs`: check `AppState::keep_running()` after `run_logic` in `handle_message_result`, so button callbacks can signal app exit. Without this, the event loop hangs after programmatic window close. This resolves an existing upstream TODO.

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
- chrono — timestamps
- tokio — async runtime
- zeroize — secure memory clearing
- anyhow — error handling
- xilem (local path) — GUI toolkit
