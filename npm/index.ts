import fs from "fs";
import path from "path";
import { execFile, type ExecFileOptions } from "child_process";

/**
 * Options for execFile when running the roost binary.
 */
export interface RunRoostOptions extends ExecFileOptions {}

/**
 * Options for domain path helpers. Can be combined with exec options.
 * - generate: if true, run `domain add` when the domain doesn't exist
 * - exact: when generating, create cert for exact domain only (no wildcard)
 * - allow: when generating, allow any TLD (bypass allowlist)
 */
export interface DomainPathOptions extends RunRoostOptions {
  generate?: boolean;
  exact?: boolean;
  allow?: boolean;
}

/**
 * Resolved filesystem paths for a domain's certificate and private key.
 * Returned by {@link getDomainPaths}.
 */
export interface DomainPaths {
  cert: string;
  key: string;
}

/**
 * Literal certificate and private key contents (PEM strings).
 * Returned by {@link getDomainCerts} for direct use with HTTPS config—no file reads needed.
 */
export interface DomainCerts {
  cert: string;
  key: string;
}

interface PathOptionsMap {
  generate?: boolean;
  exact?: boolean;
  allow?: boolean;
}

const PATH_OPTION_KEYS = ["generate", "exact", "allow"] as const;

function getBinaryPath(): string {
  const exeName = process.platform === "win32" ? "roost.exe" : "roost";
  const exePath = path.join(__dirname, "bin", exeName);

  if (!fs.existsSync(exePath)) {
    const error = new Error(
      "roost: binary not found. Make sure @itsbjoern/roost is installed and the postinstall step completed successfully."
    ) as NodeJS.ErrnoException;
    error.code = "ENOENT";
    throw error;
  }

  return exePath;
}

export function runRoost(
  args: string[],
  options: RunRoostOptions = {}
): Promise<{ stdout: string; stderr: string }> {
  return new Promise((resolve, reject) => {
    const exePath = getBinaryPath();

    execFile(
      exePath,
      args,
      {
        encoding: "utf8",
        ...options,
      },
      (error, stdout, stderr) => {
        const out = typeof stdout === "string" ? stdout : stdout?.toString() ?? "";
        const err = typeof stderr === "string" ? stderr : stderr?.toString() ?? "";
        if (error) {
          (error as NodeJS.ErrnoException & { stdout?: string; stderr?: string }).stdout = out;
          (error as NodeJS.ErrnoException & { stdout?: string; stderr?: string }).stderr = err;
          reject(error);
          return;
        }
        resolve({ stdout: out, stderr: err });
      }
    );
  });
}

function domainPathArgs(
  kind: "cert" | "key",
  domain: string,
  pathOptions: PathOptionsMap = {}
): string[] {
  const args = ["domain", "path", kind, domain];
  if (pathOptions.generate) args.push("--generate");
  if (pathOptions.exact) args.push("--exact");
  if (pathOptions.allow) args.push("--allow");
  return args;
}

export async function getDomainPath(
  kind: "cert" | "key",
  domain: string,
  options: DomainPathOptions = {}
): Promise<string> {
  if (kind !== "cert" && kind !== "key") {
    throw new Error(`Invalid kind "${kind}". Expected "cert" or "key".`);
  }

  if (!domain) {
    throw new Error("Domain is required.");
  }

  const pathOptions: PathOptionsMap = {};
  const execOptions: RunRoostOptions = {};
  for (const [key, value] of Object.entries(options)) {
    if (PATH_OPTION_KEYS.includes(key as (typeof PATH_OPTION_KEYS)[number])) {
      (pathOptions as Record<string, boolean>)[key] = value;
    } else {
      (execOptions as Record<string, unknown>)[key] = value;
    }
  }

  const args = domainPathArgs(kind, domain, pathOptions);
  const { stdout } = await runRoost(args, execOptions);
  return stdout.trim();
}

export async function getDomainCertPath(
  domain: string,
  options: DomainPathOptions = {}
): Promise<string> {
  return getDomainPath("cert", domain, options);
}

export async function getDomainKeyPath(
  domain: string,
  options: DomainPathOptions = {}
): Promise<string> {
  return getDomainPath("key", domain, options);
}

/**
 * Resolve both cert and key **paths** for a domain in one call.
 *
 * @param domain - Domain name (e.g. "api.local")
 * @param options - Optional { generate, exact, allow } and/or exec options
 * @returns Promise resolving to `{ cert: string, key: string }` (filesystem paths)
 */
export async function getDomainPaths(
  domain: string,
  options: DomainPathOptions = {}
): Promise<DomainPaths> {
  const [cert, key] = await Promise.all([
    getDomainCertPath(domain, options),
    getDomainKeyPath(domain, options),
  ]);
  return { cert, key };
}

/**
 * Resolve both cert and key **contents** (literal PEM strings) for a domain in one call.
 * Build-tool friendly: pass the result directly to HTTPS config—no file reads needed.
 * Use `generate: true` to create the domain if it doesn't exist.
 *
 * @param domain - Domain name (e.g. "api.local")
 * @param options - Optional { generate, exact, allow } and/or exec options
 * @returns Promise resolving to `{ cert: string, key: string }` (PEM file contents)
 */
export async function getDomainCerts(
  domain: string,
  options: DomainPathOptions = {}
): Promise<DomainCerts> {
  const paths = await getDomainPaths(domain, options);
  const [cert, key] = await Promise.all([
    fs.promises.readFile(paths.cert, "utf8"),
    fs.promises.readFile(paths.key, "utf8"),
  ]);
  return { cert, key };
}
