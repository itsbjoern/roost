#!/usr/bin/env node

/** Optional dependency path for the current platform (matches bin/roost PLATFORMS). */
function getOptionalDepPath() {
  const { platform, arch } = process;
  if (platform === "darwin" && arch === "arm64")
    return "@itsbjoern/roost-darwin-arm64/roost";
  if (platform === "linux" && arch === "x64")
    return "@itsbjoern/roost-linux-x64/roost";
  if (platform === "win32" && arch === "x64")
    return "@itsbjoern/roost-win32-x64/roost.exe";
  return null;
}

function main() {
  const optionalPath = getOptionalDepPath();
  if (!optionalPath) {
    console.error(
      "roost: prebuilt binary is not available for this platform.\n" +
        "Supported: linux-x64, darwin-arm64, win32-x64.\n" +
        "https://github.com/itsbjoern/roost#install",
    );
    process.exitCode = 1;
    return;
  }

  try {
    require.resolve(optionalPath, { paths: [__dirname] });
  } catch {
    console.error(
      "roost: platform binary failed to install (optional dependency missing).\n" +
        "Reinstall or see: https://github.com/itsbjoern/roost#install",
    );
    process.exitCode = 1;
  }
}

main();
