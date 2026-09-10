# 🔒 credctl

**Agent-first secret management. CLI for metadata, GUI for values.**

`credctl` is a minimal, agent-friendly secret manager. Coding agents can freely manage secrets through the CLI — listing, describing, creating, editing — without ever seeing or leaking a secret value. When a value needs to be entered, a native GUI window opens on the user's machine for direct human input.

This makes `credctl` safe to hand to an autonomous agent: the CLI is fully scriptable, but values stay out of terminal history, logs, context windows, and stdout.

## ✨ Features

- **Agent-Safe by Design:** The CLI never prints, accepts, or returns secret values. Agents operate on metadata only.
- **Human-in-the-Loop Values:** A transient native window (powered by [Xilem](https://github.com/linebender/xilem)) opens for value input — the human handles the sensitive part.
- **Two Secret Types:** `Plain Text` (single string) and `Structured` (key-value maps, auto-detected from JSON values).
- **Pluggable Backends:** AWS Secrets Manager and local `.env` files.
- **`kubectl`-style Contexts:** Switch between local dev and cloud environments.
- **Agent Skill:** Ships a `SKILL.md` ([agentskills.io](https://agentskills.io/)) so coding agents automatically use `credctl` for secret operations.

## 🚀 Quick Start

### Installation

```bash
cargo install --path .
```

Prebuilt binaries for Windows, Linux and macOS are attached to each
[GitHub release](https://github.com/backkem/credctl/releases). Every release
ships a `SHA256SUMS` file and a signed SLSA build provenance attestation.
To verify a download:

```bash
sha256sum -c SHA256SUMS --ignore-missing
gh attestation verify credctl-v0.1.0-x86_64-unknown-linux-gnu.tar.gz --repo backkem/credctl
```

The second command proves the archive was built by this repository's release
workflow from the tagged commit, not by someone with a stolen token.

### The Basics

```bash
# Create a secret (opens the native GUI to input the value)
credctl create my-token

# Create a structured key-value secret
credctl create db-creds --structured

# Edit with extra keys pre-populated in the editor
credctl edit db-creds --keys host,port,user,password

# View metadata (safe: values are NEVER printed)
credctl list
credctl describe db-creds
```

## 🌍 Contexts & Backends

`credctl` uses contexts to switch between different secret stores, configured in `~/.credctl/config.json`. Out of the box, you start in the `default` context, which uses a local `.env` file.

### 1. AWS Secrets Manager (`aws`)

Uses your local AWS credentials (`~/.aws/credentials`, SSO, etc.) to read/write directly to AWS Secrets Manager. Metadata is stored as AWS resource tags.

```bash
credctl context set staging --store aws --profile my-aws-profile
credctl context use staging
```

### 2. Dotenv (`env`)

Maps a single `.env` file to one structured secret. Because this store manages exactly one secret, the `[name]` argument becomes optional for all commands.

```bash
credctl context set prod --store env --path .env.prod
credctl context use prod

# Instantly edit .env.prod in the secure GUI
credctl edit
```

### Context Management Commands

```bash
credctl context list                  # List all (* = current)
credctl context current               # Print active context
credctl context use <name>            # Switch contexts
credctl --context <name> <command>    # One-off override for any command
```

## 🛡️ The Editor

Whenever you run `create` or `edit`, `credctl` launches a lightweight, native window:

- **Plain text:** A single masked input field.
- **Structured:** A key-value table.
- Values are masked by default (click the 👁️ icon to reveal).
- Click **Save** to write directly to the backend and close the window, or **Cancel** to abort.

_(Your terminal stays completely clean the entire time)._

## 🤖 Agent Skill

`credctl` ships an [agent skill](https://agentskills.io/) in `SKILL.md`. Install it in your coding agent and it will automatically route secret operations through `credctl`, keeping values out of the agent's context entirely.
