# credctl — Requirements

## Overview

credctl is a secret manager that separates metadata operations (CLI) from value editing (GUI). This ensures secret values never leak through terminal history, logs, or shell output.

## Core invariant

Secret values MUST:
- Never be printed to stdout or stderr
- Never be written to log files or debug output
- Never be persisted to local disk (except by the dev-store for testing)
- Only exist in process memory during active editing
- Be transmitted directly to the secret store backend over its native protocol

## Secret types

- **Plain text**: a single string value.
- **Structured (JSON object)**: a flat key-value object (`{ "key": "value", ... }`). The schema defines which keys exist. Values are always strings.

## SecretValue

A newtype wrapper around `String`:
- Zeroized on drop (memory cleared)
- No `Display`; `Debug` prints `SecretValue(****)`
- Inner value accessed only via `.expose() -> &str`

## CLI (`credctl`)

Subcommands for managing secrets without exposing values:

- `list` — list secrets (names, kind)
- `describe <name>` — show secret metadata (kind, timestamps)
- `create <name> [--json]` — create a new secret (opens editor UI for initial value)
- `edit <name>` — open editor UI to modify a secret's value
- `delete <name>` — delete a secret
- `schema <name>` — view a secret's schema
- `schema <name> --set <json>` — set schema keys (JSON array of strings)
- `help` — agent-friendly plain text help

The CLI never accepts secret values as arguments or via stdin piping.

## Editor UI

A minimal xilem window that:
- Opens only when explicitly triggered by `create` or `edit`
- Displays a text area for editing the secret value
- Loads the current value from the store into memory
- On save, writes directly to the store and closes
- On cancel/close, discards the in-memory value

## SecretStore trait

```rust
pub trait SecretStore {
    async fn list(&self) -> Result<Vec<SecretMeta>>;
    async fn describe(&self, name: &str) -> Result<SecretMeta>;
    async fn get_value(&self, name: &str) -> Result<SecretValue>;
    async fn put_value(&self, name: &str, value: &SecretValue) -> Result<()>;
    async fn create(&self, name: &str, kind: SecretKind, value: &SecretValue) -> Result<()>;
    async fn delete(&self, name: &str) -> Result<()>;
    async fn set_schema(&self, name: &str, kind: SecretKind) -> Result<()>;
}
```

### AWS Secrets Manager backend

- Uses `aws-sdk-secretsmanager` with default credential chain
- Region resolved from SDK default chain
- No custom auth configuration — relies on local `aws` auth context

### Local file store (dev-store)

- Feature flag: `dev-store`
- Stores secrets as `<base_dir>/<name>.json` files
- Each file: `{ "meta": { ... }, "value": "..." }`
- Default base dir: `.credctl/secrets/` relative to CWD
- For development/testing only
