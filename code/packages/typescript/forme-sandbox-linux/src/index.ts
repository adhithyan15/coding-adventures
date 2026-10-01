import { fileURLToPath } from "node:url";
import {
  createNativeSandboxFactory,
  type NativeLauncherOptions,
  type SandboxProcessFactory,
} from "@coding-adventures/forme-sandbox-core";

export const LINUX_SANDBOX_PROVIDER = "forme-linux-v1" as const;

export interface LinuxSandboxOptions extends NativeLauncherOptions {
  readonly launcherPath?: string;
  readonly cgroupRoot?: string;
}

export interface LinuxSandboxContract {
  readonly provider: typeof LINUX_SANDBOX_PROVIDER;
  readonly namespaces: readonly string[];
  readonly deniedProcessSyscalls: readonly string[];
  readonly deniedNetworkSyscalls: readonly string[];
  readonly requiresNoNewPrivileges: true;
  readonly requiresPrivateRoot: true;
  readonly requiresCgroupV2: true;
}

export function linuxSandboxContract(): LinuxSandboxContract {
  return Object.freeze({
    provider: LINUX_SANDBOX_PROVIDER,
    namespaces: Object.freeze(["user", "mount", "pid", "network", "ipc", "uts"]),
    deniedProcessSyscalls: Object.freeze([
      "fork", "vfork", "clone-process", "clone3", "ptrace", "process_vm_readv", "process_vm_writev",
      "mount", "umount2", "pivot_root", "chroot", "setns", "unshare", "setsid", "setpgid",
    ]),
    deniedNetworkSyscalls: Object.freeze([
      "socket", "socketpair", "connect", "bind", "listen", "accept", "accept4", "sendto", "recvfrom",
    ]),
    requiresNoNewPrivileges: true,
    requiresPrivateRoot: true,
    requiresCgroupV2: true,
  });
}

export function createLinuxSandboxFactory(options: LinuxSandboxOptions = {}): SandboxProcessFactory {
  const {
    launcherPath = defaultLauncherPath(),
    cgroupRoot = "/sys/fs/cgroup/forme-sandbox",
    ...launcherOptions
  } = options;
  return createNativeSandboxFactory({
    platform: "linux",
    provider: LINUX_SANDBOX_PROVIDER,
    launcherExecutable: launcherPath,
    launcherPrefixArguments: [`--cgroup-root=${cgroupRoot}`],
  }, launcherOptions);
}

function defaultLauncherPath(): string {
  return fileURLToPath(new URL("../native/forme-sandbox-linux", import.meta.url));
}
