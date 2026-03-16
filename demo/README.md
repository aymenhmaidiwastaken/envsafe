# Demo

This directory contains tooling to generate an animated GIF demo of envsafe.

## Prerequisites

1. **envsafe** built in release mode:

   ```
   cargo build --release
   ```

2. **agg** (asciinema GIF generator):

   ```
   cargo install --git https://github.com/asciinema/agg
   ```

## Regenerating the demo

```bash
node demo/record.js
```

This will:

1. Run real envsafe commands in a temporary directory to capture their output.
2. Build an asciinema v2 cast file at `demo/demo.cast`.
3. Render it to `demo/demo.gif` using agg with the Dracula theme.

## Playing the cast file directly

If you have asciinema installed you can preview the recording without rendering a GIF:

```bash
asciinema play demo/demo.cast
```
