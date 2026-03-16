# EnvSafe VS Code Extension

Manage encrypted environment variables directly from VS Code using the EnvSafe CLI.

## Features

- Browse environment variables in the sidebar
- Add, edit, and delete variables
- Switch between environments
- Export variables as `.env` files
- Scan workspace for leaked secrets
- Warning when opening plain `.env` files

## Requirements

- The `envsafe` CLI must be installed and available on your PATH.
- A project initialized with `envsafe init` (`.envsafe/config.json` must exist).

## Usage

1. Open a workspace that contains an `.envsafe/config.json` file.
2. The EnvSafe panel will appear in the activity bar.
3. Use the tree view to browse variables or run commands from the command palette (`Ctrl+Shift+P`).

## Building

```bash
npm install
npm run compile
```

## Commands

| Command | Description |
|---|---|
| EnvSafe: Refresh Variables | Reload the variable tree |
| EnvSafe: Add Variable | Add a new secret |
| EnvSafe: Edit Variable | Update an existing secret |
| EnvSafe: Delete Variable | Remove a secret |
| EnvSafe: Switch Environment | Change the active environment |
| EnvSafe: Export as .env | Export variables to a dotenv file |
| EnvSafe: Scan for Secrets | Scan workspace for leaked secrets |
