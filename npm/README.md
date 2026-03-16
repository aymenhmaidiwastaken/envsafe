# envsafe

**Your secrets, encrypted, everywhere.**

This is the npm installer package for [envsafe](https://github.com/aymenhmaidiwastaken/envsafe), a universal `.env` and secrets manager built in Rust.

## Installation

```bash
npm install -g envsafe
```

This will download the correct pre-built binary for your platform.

## Usage

```bash
envsafe init
envsafe set API_KEY sk-secret-value
envsafe export --env dev
```

## Supported Platforms

| OS      | x64 | arm64 |
|---------|-----|-------|
| macOS   | Yes | Yes   |
| Linux   | Yes | Yes   |
| Windows | Yes | Yes   |

## More Information

For full documentation, see the [main repository](https://github.com/aymenhmaidiwastaken/envsafe).
