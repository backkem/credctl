---
name: credctl-secrets
description: Agent-first secret manager. Use for ALL secret operations — list, describe, create, edit, delete. The CLI only exposes metadata; values are entered by the human through a native GUI. Prefer over raw AWS CLI, .env file edits, or other direct store access.
---

# credctl

`credctl` is an agent-first secret manager. The CLI gives you full control over secret metadata — listing, describing, creating, editing, deleting — but never exposes actual values. When a value needs to be entered or changed, a native GUI window opens on the user's machine for direct human input. This keeps secret values out of terminal output, agent context, and logs.

## Command Classification

| Command    | Type     | Opens GUI | Touches values |
|------------|----------|-----------|----------------|
| `list`     | metadata | no        | no             |
| `describe` | metadata | no        | no             |
| `create`   | value    | yes       | yes            |
| `edit`     | value    | yes       | yes            |
| `delete`   | danger   | no        | no (deletes)   |

**GUI commands** (`create`, `edit`): The CLI blocks and opens a native GUI window on the user's machine. Inform the user to interact with the window and wait for the command to exit. Secret values are NEVER printed to stdout/stderr.

**Metadata commands** (`list`, `describe`): Non-interactive, safe for agents and scripts. No secret values are ever returned.

To consume a secret value in another program at runtime, read from the underlying store directly (e.g., parse `.env` file, use AWS SDK). For all other secret operations, use credctl.

## Core Commands

- `credctl list` - List available secrets.
- `credctl describe [name]` - Show metadata: name, detected kind (plain-text or structured), keys, and timestamps.
- `credctl create [name]` - Create plain-text secret (opens GUI).
- `credctl create [name] --structured` - Create key-value secret (opens GUI).
- `credctl edit [name]` - Update a secret's value (opens GUI). Auto-detects structured secrets from the value.
- `credctl edit [name] --keys K1,K2,...` - Edit with extra keys pre-populated in the structured editor. Forces structured mode.
- `credctl delete [name]` - Delete secret (requires typing name to confirm).

_(Note: `[name]` is optional for `env` store contexts)._

## Contexts & Store Backends

Configuration lives at `~/.credctl/config.json`. You can switch backends using contexts, or override per-command with `--context <name>`.

- `credctl context list` / `credctl context current`
- `credctl context use <name>`
- `credctl context delete <name>`

### 1. Env Store (Default)

Maps exactly one structured secret to a single `.env` file. The default context uses `.env` in the working directory.
`credctl context set prod --store env --path .env.prod`

### 2. AWS Secrets Manager

Reads/writes directly to AWS using local AWS credentials.
`credctl context set staging --store aws --profile my-profile`

## Example Agent Workflow

If a user asks to "Create a new database credential in AWS":

1. Setup context: `credctl context set aws-prod --store aws`
2. Initiate creation: `credctl --context aws-prod create db-creds --structured`
3. Send a message to the user: _"A secure window has opened on your machine. Please enter the database credentials and click Save."_

If a user asks to "Add a new key to an existing secret":

1. `credctl --context aws-prod edit db-creds --keys NEW_KEY`
2. Send a message to the user: _"The editor is open with the new key added. Please fill in the value and click Save."_
