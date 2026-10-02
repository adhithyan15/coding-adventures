import { fileURLToPath } from "node:url";
import { execFile } from "node:child_process";
import { lstat, realpath } from "node:fs/promises";
import { isAbsolute, resolve } from "node:path";
import {
  createNativeSandboxFactory,
  type NativeLauncherOptions,
  type SandboxProcessFactory,
} from "@coding-adventures/forme-sandbox-core";

export const WINDOWS_SANDBOX_PROVIDER = "forme-windows-v1" as const;

export interface WindowsSandboxOptions extends NativeLauncherOptions {
  readonly launcherPath?: string;
}

export interface WindowsInstallAclVerifierOptions {
  readonly launcherPath?: string;
  readonly timeoutMs?: number;
}

export type WindowsInstallAclVerifier = (
  canonicalPath: string,
  scope: "install-root" | "existing-target-tree",
) => Promise<boolean>;

export interface WindowsSandboxContract {
  readonly provider: typeof WINDOWS_SANDBOX_PROVIDER;
  readonly jobLimits: readonly string[];
  readonly token: Readonly<{ restricted: true; integrity: "low" }>;
  readonly appContainerCapabilities: readonly string[];
  readonly mitigations: readonly string[];
  readonly inheritedHandles: readonly string[];
}

export function windowsSandboxContract(): WindowsSandboxContract {
  return Object.freeze({
    provider: WINDOWS_SANDBOX_PROVIDER,
    jobLimits: Object.freeze([
      "process-memory", "job-memory", "process-time", "active-process-one", "kill-on-close", "no-breakaway",
    ]),
    token: Object.freeze({ restricted: true, integrity: "low" }),
    appContainerCapabilities: Object.freeze([]),
    mitigations: Object.freeze(["dep", "aslr", "cfg", "no-remote-images"]),
    inheritedHandles: Object.freeze(["stdin", "stdout", "stderr"]),
  });
}

export function createWindowsSandboxFactory(options: WindowsSandboxOptions = {}): SandboxProcessFactory {
  const { launcherPath = defaultLauncherPath(), ...launcherOptions } = options;
  return createNativeSandboxFactory({
    platform: "win32",
    provider: WINDOWS_SANDBOX_PROVIDER,
    launcherExecutable: launcherPath,
    supervisorControl: "windows-fd4",
  }, launcherOptions);
}

/** Use the native launcher to prove an install tree has no untrusted writer. */
/* v8 ignore start -- native verifier execution is exercised by BUILD_windows */
export function createWindowsInstallAclVerifier(
  options: WindowsInstallAclVerifierOptions = {},
): WindowsInstallAclVerifier {
  const launcherPath = options.launcherPath ?? defaultLauncherPath();
  const timeoutMs = options.timeoutMs ?? 5_000;
  return async (canonicalPath, scope) => {
    if (process.platform !== "win32" || !isAbsolute(canonicalPath)
        || (scope !== "install-root" && scope !== "existing-target-tree")
        || !Number.isSafeInteger(timeoutMs) || timeoutMs <= 0) return false;
    const canonicalLauncher = await trustedRegularFile(launcherPath);
    const canonicalTarget = await realpath(canonicalPath).catch(() => null);
    if (canonicalLauncher === null || canonicalTarget === null
        || resolve(canonicalPath).toLowerCase() !== canonicalTarget.toLowerCase()) return false;
    return new Promise(resolveResult => {
      execFile(canonicalLauncher, [
        `--verify-acl-path=${canonicalTarget}`,
        `--verify-acl-scope=${scope}`,
      ], {
        windowsHide: true,
        timeout: timeoutMs,
        maxBuffer: 1_024,
      }, error => resolveResult(error === null));
    });
  };
}

async function trustedRegularFile(path: string): Promise<string | null> {
  if (!isAbsolute(path)) return null;
  const requested = resolve(path);
  const requestedStat = await lstat(requested).catch(() => null);
  if (requestedStat?.isFile() !== true || requestedStat.isSymbolicLink()) return null;
  const canonical = await realpath(requested).catch(() => null);
  if (canonical === null) return null;
  const stat = await lstat(canonical).catch(() => null);
  return stat?.isFile() === true && stat.isSymbolicLink() === false
      && requested.toLowerCase() === canonical.toLowerCase()
    ? canonical
    : null;
}
/* v8 ignore stop */

function defaultLauncherPath(): string {
  return fileURLToPath(new URL("../native/forme-sandbox-windows.exe", import.meta.url));
}
