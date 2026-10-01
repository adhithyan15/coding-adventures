import { describe, expect, it } from "vitest";
import { createLinuxSandboxFactory, linuxSandboxContract } from "../src/index.js";

describe("Linux sandbox policy", () => {
  it("pins the complete FM02 kernel boundary", () => {
    const contract = linuxSandboxContract();
    expect(contract.provider).toBe("forme-linux-v1");
    expect(contract.namespaces).toEqual(["user", "mount", "pid", "network", "ipc", "uts"]);
    expect(contract.deniedProcessSyscalls).toEqual(expect.arrayContaining([
      "fork", "vfork", "clone-process", "clone3", "ptrace",
      "mount", "umount2", "pivot_root", "chroot", "setns", "unshare", "setsid", "setpgid",
    ]));
    expect(contract.deniedNetworkSyscalls).toEqual(expect.arrayContaining([
      "socket", "connect", "bind", "listen", "accept", "accept4",
    ]));
    expect(contract.requiresNoNewPrivileges).toBe(true);
    expect(contract.requiresPrivateRoot).toBe(true);
    expect(contract.requiresCgroupV2).toBe(true);
  });

  it("constructs only the versioned Linux factory", () => {
    const factory = createLinuxSandboxFactory({ launcherPath: "/trusted/forme-sandbox-linux" });
    expect(typeof factory.launch).toBe("function");
    expect(typeof createLinuxSandboxFactory().launch).toBe("function");
  });
});
