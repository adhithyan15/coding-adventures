import { describe, expect, it } from "vitest";
import {
  createWindowsInstallAclVerifier,
  createWindowsSandboxFactory,
  windowsSandboxContract,
} from "../src/index.js";

describe("Windows sandbox policy", () => {
  it("pins the complete FM02 Job, token, AppContainer, and mitigation boundary", () => {
    const contract = windowsSandboxContract();
    expect(contract.provider).toBe("forme-windows-v1");
    expect(contract.jobLimits).toEqual(expect.arrayContaining([
      "process-memory", "job-memory", "process-time", "active-process-one",
      "kill-on-close", "no-breakaway",
    ]));
    expect(contract.token).toEqual({ restricted: true, integrity: "low" });
    expect(contract.appContainerCapabilities).toEqual([]);
    expect(contract.mitigations).toEqual(expect.arrayContaining([
      "dep", "aslr", "cfg", "no-remote-images",
    ]));
    expect(contract.inheritedHandles).toEqual(["stdin", "stdout", "stderr"]);
  });

  it("constructs only the versioned Windows factory", () => {
    const factory = createWindowsSandboxFactory({ launcherPath: "C:\\trusted\\forme-sandbox-windows.exe" });
    expect(typeof factory.launch).toBe("function");
    expect(typeof createWindowsSandboxFactory().launch).toBe("function");
  });

  it("constructs a fail-closed native install ACL verifier", async () => {
    const verify = createWindowsInstallAclVerifier({
      launcherPath: "C:\\trusted\\forme-sandbox-windows.exe",
    });
    expect(typeof verify).toBe("function");
    expect(await verify("relative", "install-root")).toBe(false);
  });
});
