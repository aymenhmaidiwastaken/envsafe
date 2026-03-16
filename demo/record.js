#!/usr/bin/env node
// demo/record.js - Generate an asciinema .cast file and render it to a GIF
//
// Prerequisites:
//   - envsafe built: cargo build --release
//   - agg installed: cargo install --git https://github.com/asciinema/agg
//
// Usage: node demo/record.js

const { execSync } = require('child_process');
const fs = require('fs');
const path = require('path');
const os = require('os');

// ---------------------------------------------------------------------------
// Configuration
// ---------------------------------------------------------------------------

const ENVSAFE = path.join(__dirname, '..', 'target', 'release', 'envsafe.exe');
const CAST_PATH = path.join(__dirname, 'demo.cast');
const GIF_PATH = path.join(__dirname, 'demo.gif');

// Verify the binary exists
if (!fs.existsSync(ENVSAFE)) {
  console.error(`envsafe binary not found at ${ENVSAFE}`);
  console.error('Build it first:  cargo build --release');
  process.exit(1);
}

// ---------------------------------------------------------------------------
// Recording helpers
// ---------------------------------------------------------------------------

const events = [];
let currentTime = 0;

function addDelay(seconds) {
  currentTime += seconds;
}

function addOutput(text) {
  events.push([parseFloat(currentTime.toFixed(4)), 'o', text]);
}

function typeCommand(cmd) {
  // Green bold prompt
  addOutput('\u001b[32m\u001b[1m\u276f\u001b[0m ');
  addDelay(0.3);

  // Type each character with realistic jitter
  for (const char of cmd) {
    addOutput(char);
    addDelay(0.03 + Math.random() * 0.04);
  }
  addOutput('\r\n');
  addDelay(0.1);
}

function addComment(text) {
  addOutput('\u001b[2m\u001b[3m# ' + text + '\u001b[0m\r\n');
  addDelay(0.8);
}

function showOutput(text) {
  const lines = text.split('\n');
  for (const line of lines) {
    addOutput(line + '\r\n');
    addDelay(0.06 + Math.random() * 0.06);
  }
}

function clearScreen() {
  addOutput('\u001b[2J\u001b[H');
  addDelay(0.3);
}

function stripAnsi(str) {
  // Remove ANSI escape sequences from real command output
  // eslint-disable-next-line no-control-regex
  return str.replace(/\u001b\[[0-9;]*[A-Za-z]/g, '');
}

function runCommand(cmd) {
  try {
    const output = execSync(cmd, { encoding: 'utf-8', cwd: tmpDir, timeout: 15000 });
    return stripAnsi(output.trimEnd());
  } catch (e) {
    const raw = e.stdout ? e.stdout : e.message;
    return stripAnsi(raw.trimEnd());
  }
}

// ---------------------------------------------------------------------------
// Temp project directory
// ---------------------------------------------------------------------------

const tmpDir = fs.mkdtempSync(path.join(os.tmpdir(), 'envsafe-demo-'));

// ---------------------------------------------------------------------------
// Scene 1: Initialize and add secrets
// ---------------------------------------------------------------------------

addComment('Initialize envsafe in your project');
addDelay(0.4);

typeCommand('envsafe init');
const initOutput = runCommand(`"${ENVSAFE}" init`);
showOutput(initOutput);
addDelay(3);

addOutput('\r\n');
addComment('Add some secrets');
addDelay(0.4);

typeCommand('envsafe set DATABASE_URL "postgres://user:pass@localhost/mydb"');
const setDbOutput = runCommand(`"${ENVSAFE}" set DATABASE_URL "postgres://user:pass@localhost/mydb"`);
showOutput(setDbOutput);
addDelay(2);

addOutput('\r\n');
typeCommand('envsafe set API_KEY "sk-proj-abc123secret" --secret');
const setApiOutput = runCommand(`"${ENVSAFE}" set API_KEY "sk-proj-abc123secret" --secret`);
showOutput(setApiOutput);
addDelay(2);

addOutput('\r\n');
typeCommand('envsafe set PORT "3000" --env staging');
const setPortOutput = runCommand(`"${ENVSAFE}" set PORT "3000" --env staging`);
showOutput(setPortOutput);
addDelay(3);

// ---------------------------------------------------------------------------
// Scene 2: View and manage
// ---------------------------------------------------------------------------

clearScreen();
addDelay(0.5);

addComment('List your secrets (values are masked by default)');
addDelay(0.4);

typeCommand('envsafe ls');
const lsOutput = runCommand(`"${ENVSAFE}" ls`);
showOutput(lsOutput);
addDelay(3);

addOutput('\r\n');
addComment('Compare environments');
addDelay(0.4);

typeCommand('envsafe diff dev staging');
const diffOutput = runCommand(`"${ENVSAFE}" diff dev staging`);
showOutput(diffOutput);
addDelay(3);

// ---------------------------------------------------------------------------
// Scene 3: Use secrets
// ---------------------------------------------------------------------------

clearScreen();
addDelay(0.5);

addComment('Run your app with secrets injected');
addDelay(0.4);

typeCommand('envsafe run -- printenv DATABASE_URL');
const runCmdOutput = runCommand(`"${ENVSAFE}" run -- printenv DATABASE_URL`);
showOutput(runCmdOutput);
addDelay(3);

addOutput('\r\n');
addComment('Export as JSON');
addDelay(0.4);

typeCommand('envsafe export --format json');
const exportOutput = runCommand(`"${ENVSAFE}" export --format json`);
showOutput(exportOutput);
addDelay(3);

// ---------------------------------------------------------------------------
// Scene 4: Lock for git sharing
// ---------------------------------------------------------------------------

clearScreen();
addDelay(0.5);

addComment('Lock vault for safe git sharing');
addDelay(0.4);

typeCommand('envsafe lock');
const lockOutput = runCommand(`"${ENVSAFE}" lock`);
showOutput(lockOutput);
addDelay(3);

addOutput('\r\n');
addComment('Scan for leaked secrets');
addDelay(0.4);

typeCommand('envsafe scan');
const scanOutput = runCommand(`"${ENVSAFE}" scan`);
showOutput(scanOutput);
addDelay(3);

// Final pause so the last output is visible
addDelay(2);

// ---------------------------------------------------------------------------
// Write .cast file
// ---------------------------------------------------------------------------

const header = JSON.stringify({
  version: 2,
  width: 90,
  height: 30,
  env: { TERM: 'xterm-256color' },
});

const castContent = header + '\n' + events.map((e) => JSON.stringify(e)).join('\n') + '\n';
fs.writeFileSync(CAST_PATH, castContent, 'utf-8');
console.log(`Cast file written to ${CAST_PATH}`);

// ---------------------------------------------------------------------------
// Render GIF with agg
// ---------------------------------------------------------------------------

try {
  execSync(
    `agg "${CAST_PATH}" "${GIF_PATH}" --theme dracula --font-size 16`,
    { stdio: 'inherit' }
  );
  console.log(`Demo GIF rendered to ${GIF_PATH}`);
} catch (e) {
  console.error('agg rendering failed. Is agg installed?');
  console.error('Install it with: cargo install --git https://github.com/asciinema/agg');
  console.error('The .cast file has still been written and can be played with: asciinema play demo/demo.cast');
}

// ---------------------------------------------------------------------------
// Cleanup
// ---------------------------------------------------------------------------

fs.rmSync(tmpDir, { recursive: true, force: true });
console.log('Done.');
