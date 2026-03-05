#!/usr/bin/env node

const https = require("https");
const fs = require("fs");
const path = require("path");
const os = require("os");
const { pipeline } = require("stream");
const { promisify } = require("util");
const tar = require("tar");

const streamPipeline = promisify(pipeline);

function getTarget() {
  const platform = process.platform;
  const arch = process.arch;

  if (platform === "linux" && arch === "x64") {
    return "x86_64-unknown-linux-gnu";
  }

  if (platform === "darwin" && arch === "arm64") {
    return "aarch64-apple-darwin";
  }

  if (platform === "win32" && arch === "x64") {
    return "x86_64-pc-windows-msvc";
  }

  return null;
}

async function download(url, dest) {
  await fs.promises.mkdir(path.dirname(dest), { recursive: true });

  await new Promise((resolve, reject) => {
    const file = fs.createWriteStream(dest);

    https
      .get(url, (res) => {
        if (res.statusCode && res.statusCode >= 300 && res.statusCode < 400 && res.headers.location) {
          // Redirect
          https.get(res.headers.location, (res2) => {
            if (res2.statusCode !== 200) {
              reject(
                new Error(`Failed to download binary. HTTP status code: ${res2.statusCode}`)
              );
              return;
            }
            streamPipeline(res2, file).then(resolve).catch(reject);
          }).on("error", reject);
          return;
        }

        if (res.statusCode !== 200) {
          reject(
            new Error(`Failed to download binary. HTTP status code: ${res.statusCode}`)
          );
          return;
        }

        streamPipeline(res, file).then(resolve).catch(reject);
      })
      .on("error", reject);
  });
}

async function main() {
  const target = getTarget();
  if (!target) {
    console.error(
      "roost: prebuilt binary is not available for this platform yet.\n" +
        "Supported from npm: linux-x64, darwin-arm64, win32-x64.\n" +
        "You can still install by downloading a release binary or building from source:\n" +
        "  https://github.com/itsbjoern/roost#install"
    );
    process.exitCode = 1;
    return;
  }

  const pkgPath = path.join(__dirname, "package.json");
  let version = null;
  try {
    // eslint-disable-next-line import/no-dynamic-require, global-require
    version = require(pkgPath).version;
  } catch {
    // best-effort; fall back to env if present
    version = process.env.npm_package_version || null;
  }

  if (!version) {
    console.error("roost: could not determine package version for download URL.");
    process.exitCode = 1;
    return;
  }

  const tag = `v${version}`;
  const fileName = `roost-${target}.tar.gz`;
  const url = `https://github.com/itsbjoern/roost/releases/download/${tag}/${fileName}`;

  const tmpDir = await fs.promises.mkdtemp(path.join(os.tmpdir(), "roost-"));
  const archivePath = path.join(tmpDir, fileName);
  const binDir = path.join(__dirname, "bin");

  try {
    console.log(`roost: downloading ${url}`);
    await download(url, archivePath);

    await fs.promises.mkdir(binDir, { recursive: true });

    await tar.x({
      file: archivePath,
      cwd: binDir,
    });

    const extractedName = process.platform === "win32" ? "roost.exe" : "roost";
    const extractedPath = path.join(binDir, extractedName);

    const exists = fs.existsSync(extractedPath);
    if (!exists) {
      throw new Error(
        `Extracted archive did not contain expected binary at ${extractedPath}`
      );
    }

    // On Unix, archive contains "roost" which would overwrite the Node wrapper.
    // Rename to roost-bin so bin/roost stays as the wrapper script.
    let binPath;
    if (process.platform === "win32") {
      binPath = extractedPath; // already bin/roost.exe
    } else {
      binPath = path.join(binDir, "roost-bin");
      await fs.promises.rename(extractedPath, binPath);
    }

    if (process.platform !== "win32") {
      await fs.promises.chmod(binPath, 0o755);
    }

    console.log("roost: binary installed.");
  } catch (err) {
    console.error(`roost: failed to install binary from GitHub Releases.\n${err}`);
    console.error(
      "You can download a binary manually or build from source:\n" +
        "  https://github.com/itsbjoern/roost#install"
    );
    process.exitCode = 1;
  } finally {
    // best-effort cleanup
    try {
      await fs.promises.rm(tmpDir, { recursive: true, force: true });
    } catch {
      // ignore
    }
  }
}

void main();

