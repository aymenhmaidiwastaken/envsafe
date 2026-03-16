# Contributing to envsafe

Welcome! We appreciate your interest in contributing to envsafe. Whether you are fixing a bug, adding a feature, improving documentation, or suggesting an idea, your help is valued.

## Prerequisites

Before you begin, make sure you have the following installed:

- **Rust** (stable toolchain) -- install via [rustup](https://rustup.rs/)
- **git**

## Dev Setup

```bash
git clone https://github.com/aymenhmaidiwastaken/envsafe.git
cd envsafe
cargo build
cargo test
```

## Project Structure

The source code lives under `src/` and is organized into the following modules:

| Directory    | Description                                              |
| ------------ | -------------------------------------------------------- |
| `src/cli/`   | CLI argument parsing, command definitions, and dispatch  |
| `src/vault/` | Encrypted vault storage, key management, and crypto ops  |
| `src/env/`   | `.env` file parsing, serialization, and environment I/O  |
| `src/sync/`  | Sync providers (AWS, GCP, Azure, Vault, etc.)            |
| `src/git/`   | Git integration (hooks, diff detection, branch tracking) |
| `src/tui/`   | Terminal UI components and interactive prompts            |

Other notable top-level source files include `main.rs` (entry point), `config.rs` / `config_file.rs` (configuration), `plugin.rs` (plugin system), `audit.rs`, `logging.rs`, `telemetry.rs`, and `webhooks.rs`.

## How to Add a New Command

1. Create a new module or file under `src/cli/` for your command.
2. Define the command's arguments using `clap` (derive or builder API).
3. Register the command in the main CLI enum / dispatch logic in `src/cli/mod.rs`.
4. Implement the command handler, calling into the appropriate domain modules (`vault`, `env`, `sync`, etc.).
5. Add tests for the new command (unit tests in the module, integration tests in `tests/`).

## How to Add a New Sync Provider

1. Create a new module under `src/sync/` for the provider (e.g., `src/sync/my_provider.rs`).
2. Implement the sync provider trait defined in `src/sync/mod.rs`.
3. Register the provider in the provider registry so the CLI can discover it.
4. Add configuration fields to `config.rs` / `config_file.rs` if needed.
5. Write tests covering authentication, push, pull, and error handling.

## Code Style

- **Format** your code before committing:
  ```bash
  cargo fmt
  ```
- **Lint** with no warnings allowed:
  ```bash
  cargo clippy -- -D warnings
  ```

Please fix all formatting and lint issues before opening a pull request.

## Testing

- Run the full test suite:
  ```bash
  cargo test
  ```
- Integration tests live in the `tests/` directory. If your change touches user-facing behavior, add or update an integration test.

## Commit Message Format

We follow [Conventional Commits](https://www.conventionalcommits.org/):

```
<type>(<scope>): <short summary>
```

Common types: `feat`, `fix`, `docs`, `style`, `refactor`, `test`, `chore`.

Examples:

```
feat(sync): add support for S3 sync provider
fix(vault): handle empty passphrase gracefully
docs(readme): update installation instructions
```

## Pull Request Process

1. Fork the repository and create a feature branch from `main`.
2. Make your changes, ensuring all tests pass and code is formatted/linted.
3. Write a clear PR description explaining **what** changed and **why**.
4. Link any related issues (e.g., `Closes #42`).
5. A maintainer will review your PR. Please be responsive to feedback.
6. Once approved, a maintainer will merge your PR.

## License

By contributing to envsafe, you agree that your contributions will be licensed under the [MIT License](LICENSE).
