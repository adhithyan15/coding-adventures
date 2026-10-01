import { mkdtemp, readFile, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { afterEach, describe, expect, it } from "vitest";
import { computeManifestHash, type Manifest } from "@coding-adventures/forme-manifest";
import type { SandboxLaunchRequest } from "@coding-adventures/forme-sandbox-core";
import { createMacosSandboxFactory } from "../src/index.js";

const packageRoot = dirname(dirname(fileURLToPath(import.meta.url)));
const launcherPath = join(packageRoot, "native", "forme-sandbox-macos");
const probePath = join(packageRoot, "native", "forme-sandbox-probe");
const roots: string[] = [];

afterEach(async () => Promise.all(roots.splice(0).map(root => rm(root, { recursive: true, force: true }))));

async function request(probe: string): Promise<SandboxLaunchRequest> {
  const workingDirectory = await mkdtemp(join(tmpdir(), "forme-macos-integration-"));
  roots.push(workingDirectory);
  const entryBytes = new Uint8Array(await readFile(probePath));
  const manifest: Manifest = {
    manifestVersion: 1,
    plugin: { name: "@example/macos-probe", version: "1.0.0", apiVersion: 1 },
    runtime: { kind: "binary", entry: "probe" },
    capabilities: { required: [], optional: [] },
    contributes: { stages: [{ id: probe, consumes: "ContentSource", produces: "ContentNode" }], kinds: [] },
    resources: { maxMemoryMb: 64, maxWallClockMs: 2_000, maxFileDescriptors: 32 },
  };
  return {
    plugin: { manifest, manifestHash: computeManifestHash(manifest, entryBytes), entryBytes },
    stage: manifest.contributes.stages[0]!,
    instanceId: `probe/${probe}`,
    workingDirectory,
    resources: manifest.resources,
    configSchema: null,
  };
}

async function nodeRequest(): Promise<SandboxLaunchRequest> {
  const workingDirectory = await mkdtemp(join(tmpdir(), "forme-macos-node-"));
  roots.push(workingDirectory);
  const entryBytes = new TextEncoder().encode("process.exit(0);\n");
  const manifest: Manifest = {
    manifestVersion: 1,
    plugin: { name: "@example/macos-node", version: "1.0.0", apiVersion: 1 },
    runtime: { kind: "node", entry: "entry.mjs" },
    capabilities: { required: [], optional: [] },
    contributes: { stages: [{ id: "main", consumes: "ContentSource", produces: "ContentNode" }], kinds: [] },
    resources: { maxMemoryMb: 64, maxWallClockMs: 2_000, maxFileDescriptors: 32 },
  };
  return {
    plugin: { manifest, manifestHash: computeManifestHash(manifest, entryBytes), entryBytes },
    stage: manifest.contributes.stages[0]!, instanceId: "node/main", workingDirectory,
    resources: manifest.resources, configSchema: null,
  };
}

describe.skipIf(process.platform !== "darwin")("macOS native sandbox", () => {
  it("boots the trusted Node runtime after installing the sandbox", async () => {
    const child = await createMacosSandboxFactory({ launcherPath, readinessTimeoutMs: 5_000 })
      .launch(await nodeRequest());
    const stderr: Buffer[] = [];
    child.stderr.on("data", chunk => stderr.push(Buffer.from(chunk)));
    const exit = await child.exited;
    expect(exit, Buffer.concat(stderr).toString("utf8")).toEqual({ code: 0, signal: null });
  });

  it.each(["filesystem", "network", "process", "memory", "descriptors", "environment", "cpu", "wall-clock", "process-group"])(
    "blocks unauthorized %s access",
    async probe => {
      process.env.FORME_SANDBOX_AMBIENT_SENTINEL = "must-not-cross";
      try {
        const child = await createMacosSandboxFactory({ launcherPath, readinessTimeoutMs: 2_000 })
          .launch(await request(probe));
        const exit = await child.exited;
        if (["memory", "cpu", "wall-clock"].includes(probe)) expect(exit.code).not.toBe(0);
        else expect(exit).toEqual({ code: 0, signal: null });
      } finally {
        delete process.env.FORME_SANDBOX_AMBIENT_SENTINEL;
      }
    },
  );

  it("retires the sandboxed session when the supervisor dies abnormally", async () => {
    const value = await request("parent-death");
    let launcherPid: number | undefined;
    const child = await createMacosSandboxFactory({
      launcherPath,
      readinessTimeoutMs: 2_000,
      onLauncherSpawn(pid) { launcherPid = pid; },
    }).launch(value);
    expect(launcherPid).toBeTypeOf("number");
    const pluginPid = Number.parseInt(await waitForFile(join(value.workingDirectory, "plugin.pid")), 10);
    expect(pluginPid).toBeGreaterThan(1);

    process.kill(launcherPid!, "SIGKILL");
    await child.exited;
    await waitForProcessExit(pluginPid);
  });

});

async function waitForFile(path: string): Promise<string> {
  const deadline = Date.now() + 2_000;
  for (;;) {
    try {
      return await readFile(path, "utf8");
    } catch (error) {
      if (Date.now() >= deadline) throw error;
      await delay(10);
    }
  }
}

async function waitForProcessExit(pid: number): Promise<void> {
  const deadline = Date.now() + 2_000;
  for (;;) {
    try {
      process.kill(pid, 0);
    } catch (error) {
      if ((error as NodeJS.ErrnoException).code === "ESRCH") return;
      throw error;
    }
    if (Date.now() >= deadline) throw new Error(`sandboxed process ${pid} survived supervisor death`);
    await delay(10);
  }
}

function delay(milliseconds: number): Promise<void> {
  return new Promise(resolve => setTimeout(resolve, milliseconds));
}
