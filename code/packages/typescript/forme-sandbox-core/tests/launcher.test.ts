import { mkdtemp, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { afterEach, describe, expect, it } from "vitest";
import { computeManifestHash, type Manifest } from "@coding-adventures/forme-manifest";
import {
  createNativeSandboxFactory,
  launchWithNativeHelper,
  type NativeSandboxPolicy,
  type SandboxLaunchRequest,
} from "../src/index.js";

const roots: string[] = [];
afterEach(async () => Promise.all(roots.splice(0).map(root => rm(root, { recursive: true, force: true }))));

const helper = new URL("./fixtures/readiness-helper.mjs", import.meta.url).pathname;

function policy(): NativeSandboxPolicy {
  return {
    platform: process.platform,
    provider: "forme-test-v1",
    launcherExecutable: process.execPath,
    launcherPrefixArguments: [helper],
    ...(process.platform === "win32" ? { supervisorControl: "windows-fd4" as const } : {}),
  };
}

async function request(): Promise<SandboxLaunchRequest> {
  const workingDirectory = await mkdtemp(join(tmpdir(), "forme-native-launch-test-"));
  roots.push(workingDirectory);
  const manifest: Manifest = {
    manifestVersion: 1,
    plugin: { name: "@example/launch", version: "1.0.0", apiVersion: 1 },
    runtime: { kind: "node", entry: "entry.mjs" },
    capabilities: { required: [], optional: [] },
    contributes: {
      stages: [{ id: "main", consumes: "ContentSource", produces: "ContentNode" }],
      kinds: [],
    },
    resources: { maxMemoryMb: 64, maxWallClockMs: 1_000, maxFileDescriptors: 32 },
  };
  const entryBytes = new TextEncoder().encode("process.stdin.resume();\n");
  return {
    plugin: { manifest, manifestHash: computeManifestHash(manifest, entryBytes), entryBytes },
    stage: manifest.contributes.stages[0]!,
    instanceId: "launch/main",
    workingDirectory,
    resources: manifest.resources,
    configSchema: null,
  };
}

describe("launchWithNativeHelper", () => {
  it("accepts a bounded readiness record from the pre-exec helper", async () => {
    let launcherPid: number | undefined;
    const child = await launchWithNativeHelper(await request(), policy(), {
      readinessTimeoutMs: 1_000,
      onLauncherSpawn(pid) { launcherPid = pid; },
    });
    expect(child.isolation).toBe("sandboxed");
    expect(child.isolationProvider).toBe("forme-test-v1");
    expect(launcherPid).toBeTypeOf("number");
    child.signal("SIGKILL");
    await child.exited;
  });

  it("fails closed when the helper does not attest the exact provider", async () => {
    await expect(launchWithNativeHelper(await request(), {
      ...policy(),
      provider: "forme-wrong-v1",
    }, { readinessTimeoutMs: 1_000 })).rejects.toMatchObject({ code: "ATTESTATION_MISMATCH" });
  });

  it("fails closed when a launcher without a prefix helper exits before attestation", async () => {
    await expect(launchWithNativeHelper(await request(), {
      ...policy(), launcherPrefixArguments: undefined,
    }, { readinessTimeoutMs: 1_000 })).rejects.toMatchObject({ code: "ATTESTATION_MISMATCH" });
  });

  it("cleans up the supervisor when a diagnostic launch hook throws", async () => {
    await expect(launchWithNativeHelper(await request(), policy(), {
      readinessTimeoutMs: 1_000,
      onLauncherSpawn() { throw new Error("diagnostic hook failed"); },
    })).rejects.toThrow("diagnostic hook failed");
  });

  it("does not copy ambient environment variables into the helper", async () => {
    process.env.FORME_SANDBOX_AMBIENT_SENTINEL = "must-not-cross";
    try {
      const child = await launchWithNativeHelper(await request(), policy(), {
        readinessTimeoutMs: 1_000,
      });
      child.signal("SIGKILL");
      expect(await child.exited).toMatchObject(process.platform === "win32"
        ? { code: 137 }
        : { code: null, signal: "SIGKILL" });
    } finally {
      delete process.env.FORME_SANDBOX_AMBIENT_SENTINEL;
    }
  });

  it("exposes a structurally compatible factory", async () => {
    const child = await createNativeSandboxFactory(policy(), { readinessTimeoutMs: 1_000 })
      .launch(await request());
    child.signal("SIGKILL");
    await child.exited;
  });

  it("forwards ordinary signals through the supervised process group", async () => {
    const child = await launchWithNativeHelper(await request(), policy(), { readinessTimeoutMs: 1_000 });
    child.signal("SIGTERM");
    expect(await child.exited).toMatchObject(process.platform === "win32"
      ? { code: 137 }
      : { code: null, signal: "SIGTERM" });
  });

  it("applies host-owned defaults when the manifest omits resource limits", async () => {
    const value = await request();
    (value as { resources: undefined }).resources = undefined;
    const child = await launchWithNativeHelper(value, policy(), { readinessTimeoutMs: 1_000 });
    child.signal("SIGKILL");
    await child.exited;
  });

  it("refuses an unsupported platform and an unversioned provider", async () => {
    const otherPlatform = process.platform === "darwin" ? "linux" : "darwin";
    await expect(launchWithNativeHelper(await request(), {
      ...policy(), platform: otherPlatform,
    })).rejects.toMatchObject({ code: "SANDBOX_UNAVAILABLE" });
    await expect(launchWithNativeHelper(await request(), {
      ...policy(), provider: "unversioned",
    })).rejects.toMatchObject({ code: "INVALID_LAUNCH_REQUEST" });
  });

  it("requires absolute existing launcher and non-node runtime executables", async () => {
    await expect(launchWithNativeHelper(await request(), {
      ...policy(), launcherExecutable: "relative-helper",
    })).rejects.toMatchObject({ code: "SANDBOX_UNAVAILABLE" });
    await expect(launchWithNativeHelper(await request(), {
      ...policy(), launcherExecutable: "/definitely/missing/forme-helper",
    })).rejects.toMatchObject({ code: "SANDBOX_UNAVAILABLE" });
    const value = await request();
    (value.plugin.manifest.runtime as { kind: "python" }).kind = "python";
    (value.plugin as { manifestHash: string }).manifestHash = computeManifestHash(
      value.plugin.manifest,
      value.plugin.entryBytes,
    );
    await expect(launchWithNativeHelper(value, policy())).rejects.toMatchObject({ code: "SANDBOX_UNAVAILABLE" });

    await expect(launchWithNativeHelper(await request(), {
      ...policy(), launcherExecutable: tmpdir(),
    })).rejects.toMatchObject({ code: "SANDBOX_UNAVAILABLE" });
  });

  it("requires an existing runtime distribution root that contains the runtime", async () => {
    await expect(launchWithNativeHelper(await request(), policy(), {
      runtimeRoots: { node: "relative" },
    })).rejects.toMatchObject({ code: "SANDBOX_UNAVAILABLE" });
    await expect(launchWithNativeHelper(await request(), policy(), {
      runtimeRoots: { node: "/definitely/missing/runtime-root" },
    })).rejects.toMatchObject({ code: "SANDBOX_UNAVAILABLE" });
    await expect(launchWithNativeHelper(await request(), policy(), {
      runtimeRoots: { node: tmpdir() },
    })).rejects.toMatchObject({ code: "SANDBOX_UNAVAILABLE" });
  });

  it.each([0, -1, 1.5, Number.MAX_SAFE_INTEGER])(
    "rejects invalid resource limit %s",
    async limit => {
      const value = await request();
      (value as { resources: Manifest["resources"] }).resources = {
        ...value.resources,
        maxMemoryMb: limit,
      };
      await expect(launchWithNativeHelper(value, policy())).rejects.toMatchObject({
        code: "INVALID_LAUNCH_REQUEST",
      });
    },
  );

  it("rejects a memory limit whose byte representation is unsafe", async () => {
    const value = await request();
    (value as { resources: Manifest["resources"] }).resources = {
      ...value.resources,
      maxMemoryMb: Number.MAX_SAFE_INTEGER - 1,
    };
    await expect(launchWithNativeHelper(value, policy())).rejects.toMatchObject({
      code: "INVALID_LAUNCH_REQUEST",
    });
  });

  it.each(["maxWallClockMs", "maxFileDescriptors"] as const)(
    "rejects invalid %s resource limits",
    async field => {
      const value = await request();
      (value as { resources: Manifest["resources"] }).resources = {
        ...value.resources,
        [field]: 0,
      };
      await expect(launchWithNativeHelper(value, policy())).rejects.toMatchObject({
        code: "INVALID_LAUNCH_REQUEST",
      });
    },
  );

  it.each([
    ["eof", "ATTESTATION_MISMATCH"],
    ["oversized", "ATTESTATION_MISMATCH"],
    ["trailing", "ATTESTATION_MISMATCH"],
    ["malformed", "ATTESTATION_MISMATCH"],
    ["invalid-shape", "ATTESTATION_MISMATCH"],
    ["invalid-fields", "ATTESTATION_MISMATCH"],
    ["wrong-entry", "ATTESTATION_MISMATCH"],
    ["silent", "ATTESTATION_TIMEOUT"],
  ])("fails closed for readiness mode %s", async (mode, code) => {
    await expect(launchWithNativeHelper(await request(), {
      ...policy(), launcherPrefixArguments: [helper, `--mode=${mode}`],
    }, { readinessTimeoutMs: mode === "silent" ? 10 : 1_000 })).rejects.toMatchObject({ code });
  });
});
