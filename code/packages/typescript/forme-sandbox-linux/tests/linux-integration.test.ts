import { mkdtemp, readFile, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { afterEach, describe, expect, it } from "vitest";
import { computeManifestHash, type Manifest } from "@coding-adventures/forme-manifest";
import type { SandboxLaunchRequest } from "@coding-adventures/forme-sandbox-core";
import { createLinuxSandboxFactory } from "../src/index.js";

const packageRoot = dirname(dirname(fileURLToPath(import.meta.url)));
const launcherPath = join(packageRoot, "native", "forme-sandbox-linux");
const probePath = join(packageRoot, "native", "forme-sandbox-probe");
const cgroupRoot = "/sys/fs/cgroup/forme-sandbox";
const roots: string[] = [];
afterEach(async () => Promise.all(roots.splice(0).map(root => rm(root, { recursive: true, force: true }))));

async function request(probe: string): Promise<SandboxLaunchRequest> {
  const workingDirectory = await mkdtemp(join(tmpdir(), "forme-linux-integration-"));
  roots.push(workingDirectory);
  const entryBytes = new Uint8Array(await readFile(probePath));
  const manifest: Manifest = {
    manifestVersion: 1,
    plugin: { name: "@example/linux-probe", version: "1.0.0", apiVersion: 1 },
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
  const workingDirectory = await mkdtemp(join(tmpdir(), "forme-linux-node-"));
  roots.push(workingDirectory);
  const entryBytes = new TextEncoder().encode(`
let input = "";
process.stdin.setEncoding("utf8");
process.stdin.on("data", chunk => { input += chunk; });
process.stdin.on("end", () => process.stdout.write(input, () => process.exit(0)));
`);
  const manifest: Manifest = {
    manifestVersion: 1,
    plugin: { name: "@example/linux-node", version: "1.0.0", apiVersion: 1 },
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

describe.skipIf(process.platform !== "linux")("Linux native sandbox", () => {
  it("exchanges protocol bytes over inherited pipes after installing the sandbox", async () => {
    const child = await createLinuxSandboxFactory({ launcherPath, cgroupRoot, readinessTimeoutMs: 2_000 })
      .launch(await nodeRequest());
    const chunks: Buffer[] = [];
    child.stdout.on("data", chunk => chunks.push(Buffer.from(chunk)));
    child.stdin.end("sandbox protocol probe");
    expect(await child.exited).toEqual({ code: 0, signal: null });
    expect(Buffer.concat(chunks).toString("utf8")).toBe("sandbox protocol probe");
  });

  it.each(["filesystem", "network", "process", "memory", "descriptors", "environment", "cpu", "wall-clock", "mount"])(
    "blocks unauthorized %s access",
    async probe => {
      process.env.FORME_SANDBOX_AMBIENT_SENTINEL = "must-not-cross";
      try {
        const child = await createLinuxSandboxFactory({ launcherPath, cgroupRoot, readinessTimeoutMs: 2_000 })
          .launch(await request(probe));
        const exit = await child.exited;
        if (["memory", "cpu", "wall-clock"].includes(probe)) expect(exit.code).not.toBe(0);
        else expect(exit).toEqual({ code: 0, signal: null });
      } finally {
        delete process.env.FORME_SANDBOX_AMBIENT_SENTINEL;
      }
    },
  );
});
