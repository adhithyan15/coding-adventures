import { fileURLToPath } from "node:url";
import {
  createNativeSandboxFactory,
  type NativeLauncherOptions,
  type SandboxProcessFactory,
} from "@coding-adventures/forme-sandbox-core";

export const WINDOWS_SANDBOX_PROVIDER = "forme-windows-v1" as const;

export interface WindowsSandboxOptions extends NativeLauncherOptions {
  readonly launcherPath?: string;
}

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

function defaultLauncherPath(): string {
  return fileURLToPath(new URL("../native/forme-sandbox-windows.exe", import.meta.url));
}
