# EnvSafe

Your secrets, encrypted, everywhere. One tool for all .env management.

<p align="center"><img src="demo/demo.gif" width="720" alt="EnvSafe Demo"/></p>

<p align="center">
  <a href="https://github.com/aymenhmaidiwastaken/envsafe/releases"><img src="https://img.shields.io/github/v/release/aymenhmaidiwastaken/envsafe" alt="GitHub Release"/></a>
  <a href="https://github.com/aymenhmaidiwastaken/envsafe/blob/main/LICENSE"><img src="https://img.shields.io/github/license/aymenhmaidiwastaken/envsafe" alt="License"/></a>
  <a href="https://github.com/aymenhmaidiwastaken/envsafe/actions"><img src="https://img.shields.io/github/actions/workflow/status/aymenhmaidiwastaken/envsafe/ci.yml?branch=main&label=CI" alt="CI"/></a>
  <a href="https://crates.io/crates/envsafe"><img src="https://img.shields.io/crates/v/envsafe" alt="Crates.io"/></a>
</p>

---

[Install](#installation) | [Quick Start](#quick-start) | [Features](#features) | [Commands](#commands) | [Config](#configuration) | [Contributing](#contributing)

---

## The Problem

Managing environment variables and secrets across projects, teams, and environments is a persistent source of friction and risk:

- **Scattered .env files** -- secrets live in plaintext files spread across machines with no central management, no versioning, and no consistency.
- **Accidental commits** -- a single mistake pushes credentials to version control, exposing them permanently in git history.
- **Insecure sharing** -- teams pass secrets through Slack, email, or shared documents with no encryption and no audit trail.
- **Environment switching pain** -- juggling dev, staging, and production configurations means manually copying and renaming files, leading to misconfigurations and outages.
- **No unified tool** -- existing solutions each solve one piece of the puzzle, forcing teams to stitch together multiple tools, plugins, and workflows.

## The Solution

EnvSafe replaces all of that with a single, fast, cross-platform binary:

```bash
envsafe init
envsafe set DATABASE_URL "postgres://..."
envsafe set API_KEY "sk-..." --secret
envsafe run -- npm start
```

Secrets are encrypted at rest, never written as plaintext, injected directly into processes, and shareable through git-safe vault files. One tool, zero leaks.

---

## Installation

### Binary Download

```bash
curl -sSL https://github.com/aymenhmaidiwastaken/envsafe/releases/latest/download/envsafe-x86_64-unknown-linux-gnu.tar.gz \
  | tar xz -C /usr/local/bin
```

Pre-built binaries for Linux, macOS (arm64/amd64), and Windows are available on the [GitHub Releases](https://github.com/aymenhmaidiwastaken/envsafe/releases) page.

### Cargo

```bash
cargo install envsafe
```

### From Source

```bash
git clone https://github.com/aymenhmaidiwastaken/envsafe.git
cd envsafe
cargo build --release
# Binary is at target/release/envsafe
```

### npm

```bash
npx envsafe
```

---

## Quick Start

```bash
# Initialize envsafe in your project
envsafe init

# Store some variables
envsafe set DATABASE_URL "postgres://user:pass@localhost/mydb"
envsafe set API_KEY "sk-abc123" --secret

# Run your application with secrets injected
envsafe run -- npm start

# Export as a dotenv file
envsafe export --format dotenv > .env

# Lock the vault for git-safe sharing
envsafe lock
```

---

## Features

| Feature | Description |
|---|---|
| Encrypted Vault | All secrets stored in AES-256-GCM encrypted vaults. No plaintext on disk, ever. |
| Environment Profiles | Manage separate `dev`, `staging`, and `prod` configurations side by side. |
| Process Injection | Run any command with secrets injected as environment variables, without touching the shell. |
| Git-Safe Sharing | Lock secrets into an encrypted vault file safe for version control. Team members decrypt with a shared key. |
| Secret Scanning | Detect leaked API keys, tokens, and credentials in staged files before they reach your repository. |
| Cloud Sync | Pull and push secrets from AWS SSM, HashiCorp Vault, 1Password, and Google Cloud Secret Manager. |
| Schema Validation | Define required variables, types, and patterns in `.envsafe.yaml`. Catch misconfigurations before deployment. |
| Interactive TUI | Browse, search, and edit secrets in a full terminal user interface (`envsafe ui`). |
| Shell Integration | Auto-inject secrets when entering a project directory with `eval "$(envsafe hook-shell bash)"`. |
| Plugin System | Extend EnvSafe with external plugins. Any executable named `envsafe-plugin-<name>` in your PATH is discovered automatically. |

---

## Commands

| Command | Description |
|---|---|
| `init` | Initialize envsafe in the current project directory |
| `set KEY VALUE` | Set an environment variable (supports `--secret`, `--env`, `--expires`) |
| `get KEY` | Retrieve an environment variable value |
| `rm KEY` | Remove an environment variable |
| `ls` | List all environment variables (values masked by default, use `--show`) |
| `run -- CMD` | Run a command with secrets injected as environment variables |
| `export` | Export variables in shell, dotenv, json, docker, or kubernetes format |
| `import FILE` | Import variables from an existing `.env` file |
| `envs` | List all configured environments |
| `diff ENV1 ENV2` | Compare variables across two environments with color diff |
| `lock` | Encrypt vault into a git-safe `.env.vault` file |
| `unlock` | Decrypt vault from `.env.vault` file |
| `key export` | Export the project encryption key for sharing |
| `key import KEY` | Import a project encryption key from a team member |
| `validate` | Validate environment against `.envsafe.yaml` schema |
| `hook install` | Install git pre-commit hook to prevent secret leaks |
| `hook uninstall` | Remove the git pre-commit hook |
| `scan` | Scan repository for accidentally committed secrets |
| `pull PROVIDER` | Pull secrets from a cloud provider into the local vault |
| `push PROVIDER` | Push secrets from the local vault to a cloud provider |
| `template` | Generate a `.env.example` template file with placeholder values |
| `ui` | Open interactive TUI mode for browsing and editing secrets |
| `rotate-key` | Rotate the project encryption key (backs up old key) |
| `audit` | View the audit log of all vault operations |
| `completions SHELL` | Generate shell completions (bash, zsh, fish, powershell) |
| `hook-shell SHELL` | Print shell hook for automatic directory-based injection |
| `man-page` | Print the envsafe man page |
| `telemetry enable` | Enable anonymous usage telemetry |
| `telemetry disable` | Disable anonymous usage telemetry |
| `telemetry status` | Show current telemetry status |
| `plugin NAME` | Run an installed plugin by name |
| `plugins` | List all discovered plugins |

All commands support `--verbose` and `--debug` global flags.

---

## Export Formats

```bash
# Shell (default) -- source directly or use with eval
eval $(envsafe export --format shell)

# Dotenv -- standard KEY=VALUE format
envsafe export --format dotenv > .env

# JSON -- structured output for programmatic use
envsafe export --format json

# Docker -- generates --env flags for docker run
docker run $(envsafe export --format docker) myimage

# Kubernetes -- generates a Kubernetes Secret manifest
envsafe export --format kubernetes > k8s-secret.yaml
```

---

## Cloud Providers

EnvSafe supports bidirectional sync with major cloud secret managers. Use `pull` to import secrets into your local vault and `push` to deploy them.

### AWS SSM Parameter Store

```bash
envsafe pull aws-ssm --prefix /myapp/prod --env prod
envsafe push aws-ssm --prefix /myapp/prod --env prod
```

Requires configured AWS credentials (`~/.aws/credentials`, environment variables, or IAM role).

### HashiCorp Vault

```bash
export VAULT_ADDR="https://vault.example.com"
export VAULT_TOKEN="s.xxxxxxxx"

envsafe pull vault --path secret/data/myapp
envsafe push vault --path secret/data/myapp
```

### 1Password

```bash
envsafe pull 1password --vault-name "Development"
envsafe push 1password --vault-name "Development"
```

Requires the [1Password CLI](https://developer.1password.com/docs/cli/) installed and authenticated.

### Google Cloud Secret Manager

```bash
envsafe pull gcp --path projects/my-project/secrets
envsafe push gcp --path projects/my-project/secrets
```

Authenticate via `gcloud auth application-default login` or a service account key.

---

## Configuration

EnvSafe uses an optional `.envsafe.yaml` file for project-level configuration and schema validation. This file is safe and recommended to commit to version control.

```yaml
# .envsafe.yaml

required:
  - name: DATABASE_URL
    pattern: "^postgres://"
    description: "PostgreSQL connection string"

  - name: API_KEY
    pattern: "^sk-"
    description: "API key starting with sk-"

  - name: PORT
    type: integer
    default: 3000

  - name: LOG_LEVEL
    description: "Application log level"
    default: "info"

  - name: REDIS_URL
    pattern: "^redis://"
    description: "Redis connection URL"
```

Run `envsafe validate` to check your environment against this schema:

```
ERROR: DATABASE_URL does not match pattern "^postgres://"
ERROR: API_KEY is missing
WARNING: PORT is not set, using default: 3000
```

---

## Security Model

| Layer | Detail |
|---|---|
| Encryption | AES-256-GCM with a unique random nonce for every encryption operation |
| Key Derivation | Argon2id -- resistant to GPU and ASIC brute-force attacks |
| Key Storage | Master keys stored in `~/.config/envsafe/keys/`, never inside the project directory |
| Memory Safety | Written in Rust -- no buffer overflows, no use-after-free, no data races |
| Zeroize | Secret values are zeroized from memory immediately after use |
| At Rest | Vault files are always encrypted on disk. No plaintext storage. |
| In Transit | Cloud sync uses provider-native TLS. No secrets pass through EnvSafe servers. |
| No Server | Everything runs locally or communicates directly with your chosen cloud provider. There is no EnvSafe server. |

---

## Comparison

| Feature | EnvSafe | dotenvx | direnv | chamber | doppler | 1password-cli |
|---|:---:|:---:|:---:|:---:|:---:|:---:|
| Encrypted local vault | &#10003; | &#10003; | &#10007; | &#10007; | &#10007; | &#10003; |
| Environment profiles | &#10003; | &#10003; | &#10003; | &#10003; | &#10003; | &#10007; |
| Process injection | &#10003; | &#10003; | &#10003; | &#10003; | &#10003; | &#10003; |
| Git-safe sharing | &#10003; | &#10003; | &#10007; | &#10007; | &#10007; | &#10007; |
| Pre-commit hook | &#10003; | &#10007; | &#10007; | &#10007; | &#10007; | &#10007; |
| Secret scanning | &#10003; | &#10007; | &#10007; | &#10007; | &#10007; | &#10007; |
| Multi-cloud sync | &#10003; | &#10007; | &#10007; | &#10003; | &#10007; | &#10007; |
| Schema validation | &#10003; | &#10007; | &#10007; | &#10007; | &#10007; | &#10007; |
| Interactive TUI | &#10003; | &#10007; | &#10007; | &#10007; | &#10003; | &#10007; |
| Plugin system | &#10003; | &#10007; | &#10007; | &#10007; | &#10007; | &#10007; |
| Cross-platform binary | &#10003; | &#10003; | &#10007; | &#10003; | &#10003; | &#10003; |
| No external service | &#10003; | &#10003; | &#10003; | &#10007; | &#10007; | &#10007; |

---

## Contributing

Contributions are welcome. Please see [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines on how to get started, submit pull requests, and report issues.

---

## License

MIT License. See [LICENSE](LICENSE) for details.
