#!/usr/bin/env node

"use strict";

const os = require("os");
const fs = require("fs");
const path = require("path");
const https = require("https");
const { execSync } = require("child_process");

const REPO = "aymenhmaidiwastaken/envsafe";
const VERSION = require("./package.json").version;

// Map Node.js platform/arch to Rust target triples
const PLATFORM_MAP = {
  darwin: {
    x64: "envsafe-x86_64-apple-darwin",
    arm64: "envsafe-aarch64-apple-darwin",
  },
  linux: {
    x64: "envsafe-x86_64-unknown-linux-gnu",
    arm64: "envsafe-aarch64-unknown-linux-gnu",
  },
  win32: {
    x64: "envsafe-x86_64-pc-windows-msvc.exe",
    arm64: "envsafe-aarch64-pc-windows-msvc.exe",
  },
};

function getBinaryName() {
  const platform = os.platform();
  const arch = os.arch();

  const platformTargets = PLATFORM_MAP[platform];
  if (!platformTargets) {
    throw new Error(
      `Unsupported platform: ${platform}. envsafe supports darwin, linux, and win32.`
    );
  }

  const binaryName = platformTargets[arch];
  if (!binaryName) {
    throw new Error(
      `Unsupported architecture: ${arch} on ${platform}. envsafe supports x64 and arm64.`
    );
  }

  return binaryName;
}

function getDownloadUrl(binaryName) {
  return `https://github.com/${REPO}/releases/download/v${VERSION}/${binaryName}`;
}

function getOutputPath() {
  const binDir = path.join(__dirname, "bin");
  if (!fs.existsSync(binDir)) {
    fs.mkdirSync(binDir, { recursive: true });
  }
  const ext = os.platform() === "win32" ? ".exe" : "";
  return path.join(binDir, `envsafe${ext}`);
}

function download(url, dest) {
  return new Promise((resolve, reject) => {
    const request = (url) => {
      https
        .get(url, (response) => {
          // Handle redirects (GitHub releases use 302)
          if (
            response.statusCode >= 300 &&
            response.statusCode < 400 &&
            response.headers.location
          ) {
            request(response.headers.location);
            return;
          }

          if (response.statusCode !== 200) {
            reject(
              new Error(
                `Failed to download envsafe: HTTP ${response.statusCode}\n` +
                  `URL: ${url}\n` +
                  `Make sure release v${VERSION} exists at https://github.com/${REPO}/releases`
              )
            );
            return;
          }

          const file = fs.createWriteStream(dest);
          response.pipe(file);
          file.on("finish", () => {
            file.close(resolve);
          });
          file.on("error", (err) => {
            fs.unlink(dest, () => {});
            reject(err);
          });
        })
        .on("error", (err) => {
          reject(
            new Error(
              `Failed to download envsafe: ${err.message}\n` +
                `URL: ${url}\n` +
                `Check your network connection and try again.`
            )
          );
        });
    };

    request(url);
  });
}

async function main() {
  try {
    const binaryName = getBinaryName();
    const url = getDownloadUrl(binaryName);
    const outputPath = getOutputPath();

    console.log(`Downloading envsafe v${VERSION} for ${os.platform()}-${os.arch()}...`);
    console.log(`  From: ${url}`);
    console.log(`  To:   ${outputPath}`);

    await download(url, outputPath);

    // Make executable on Unix platforms
    if (os.platform() !== "win32") {
      fs.chmodSync(outputPath, 0o755);
    }

    console.log("envsafe installed successfully!");
  } catch (err) {
    console.error(`\nError installing envsafe:\n  ${err.message}`);
    console.error(
      "\nYou can also install envsafe manually from:\n" +
        `  https://github.com/${REPO}/releases\n`
    );
    process.exit(1);
  }
}

main();
