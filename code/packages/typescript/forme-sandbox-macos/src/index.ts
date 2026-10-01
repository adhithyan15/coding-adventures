import { fileURLToPath } from "node:url";
import { existsSync } from "node:fs";
import {
  createNativeSandboxFactory,
  type NativeLauncherOptions,
  type SandboxProcessFactory,
} from "@coding-adventures/forme-sandbox-core";

export const MACOS_SANDBOX_PROVIDER = "forme-macos-v1" as const;

export interface MacosSandboxOptions extends NativeLauncherOptions {
  readonly launcherPath?: string;
  /** Additional host-trusted runtime dependency roots required by dynamic loaders. */
  readonly runtimeReadPaths?: readonly string[];
}

export interface MacosSeatbeltProfileOptions {
  readonly workingDirectory: string;
  readonly runtimeReadPaths: readonly string[];
}

export function macosSeatbeltProfile(options: MacosSeatbeltProfileOptions): string {
  const cwd = seatbeltLiteral(options.workingDirectory);
  if (options.runtimeReadPaths.length === 0) throw new Error("at least one runtime read path is required");
  const runtimeRules = options.runtimeReadPaths
    .map(path => `  (allow file-read* (literal "${seatbeltLiteral(path)}"))`)
    .join("\n");
  return [
    "(version 1)",
    "(deny default)",
    '(import "system.sb")',
    '(allow process-exec)',
    '(deny process-fork)',
    '(deny network*)',
    '(allow sysctl-read)',
    '(allow mach-lookup (global-name "com.apple.system.opendirectoryd.libinfo"))',
    `  (allow file-read* (subpath "${cwd}"))`,
    `  (allow file-write* (subpath "${cwd}"))`,
    runtimeRules,
    "",
  ].join("\n");
}

export function createMacosSandboxFactory(options: MacosSandboxOptions = {}): SandboxProcessFactory {
  const {
    launcherPath = defaultLauncherPath(),
    runtimeReadPaths = defaultRuntimeReadPaths(),
    ...launcherOptions
  } = options;
  if (runtimeReadPaths.length > 32 || runtimeReadPaths.some(path => !path.startsWith("/") || /[\0\r\n]/.test(path))) {
    throw new Error("runtime read paths must be at most 32 absolute paths without control characters");
  }
  return createNativeSandboxFactory({
    platform: "darwin",
    provider: MACOS_SANDBOX_PROVIDER,
    launcherExecutable: launcherPath,
    launcherPrefixArguments: runtimeReadPaths.map(path => `--runtime-read-path=${path}`),
  }, launcherOptions);
}

function seatbeltLiteral(value: string): string {
  if (/[\0\r\n]/.test(value)) throw new Error("Seatbelt path literals cannot contain control characters");
  return value.replaceAll("\\", "\\\\").replaceAll('"', '\\"');
}

function defaultLauncherPath(): string {
  return fileURLToPath(new URL("../native/forme-sandbox-macos", import.meta.url));
}

function defaultRuntimeReadPaths(): readonly string[] {
  const report = process.report?.getReport() as { readonly sharedObjects?: readonly string[] } | undefined;
  return macosRuntimeReadPaths(report?.sharedObjects ?? []);
}

/** Derive stable Homebrew dependency roots from the runtime's loaded images. */
export function macosRuntimeReadPaths(
  sharedObjects: readonly string[],
  pathExists: (path: string) => boolean = existsSync,
): readonly string[] {
  const paths = new Set<string>();
  for (const sharedObject of sharedObjects) {
    const match = /^(.*)\/Cellar\/([^/]+)\//.exec(sharedObject);
    if (match) {
      paths.add(`${match[1]}/opt/${match[2]}`);
      const configuration = `${match[1]}/etc/${match[2]}`;
      if (pathExists(configuration)) paths.add(configuration);
    }
  }
  return Object.freeze([...paths].sort());
}
