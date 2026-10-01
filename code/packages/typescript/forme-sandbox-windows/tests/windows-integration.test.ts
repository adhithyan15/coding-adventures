import { createHash } from "node:crypto";
import { mkdtemp, readFile, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { afterEach, describe, expect, it } from "vitest";
import { computeManifestHash, type Manifest } from "@coding-adventures/forme-manifest";
import type { SandboxLaunchRequest } from "@coding-adventures/forme-sandbox-core";
import { createWindowsSandboxFactory } from "../src/index.js";

const packageRoot = dirname(dirname(fileURLToPath(import.meta.url)));
const launcherPath = join(packageRoot, "native", "forme-sandbox-windows.exe");
const probePath = join(packageRoot, "native", "forme-sandbox-probe.exe");
const roots: string[] = [];
afterEach(async () => Promise.all(roots.splice(0).map(root => rm(root, { recursive: true, force: true }))));

async function request(probe: string): Promise<SandboxLaunchRequest> {
  const workingDirectory = await mkdtemp(join(tmpdir(), "forme-windows-integration-"));
  roots.push(workingDirectory);
  const entryBytes = new Uint8Array(await readFile(probePath));
  const manifest: Manifest = {
    manifestVersion: 1,
    plugin: { name: "@example/windows-probe", version: "1.0.0", apiVersion: 1 },
    runtime: { kind: "binary", entry: "probe.exe" },
    capabilities: { required: [], optional: [] },
    contributes: { stages: [{ id: probe, consumes: "ContentSource", produces: "ContentNode" }], kinds: [] },
    resources: { maxMemoryMb: 64, maxWallClockMs: 2_000, maxFileDescriptors: 256 },
  };
  const schemaBytes = new TextEncoder().encode('{"type":"object"}\n');
  return {
    plugin: { manifest, manifestHash: computeManifestHash(manifest, entryBytes), entryBytes },
    stage: manifest.contributes.stages[0]!,
    instanceId: `probe/${probe}`,
    workingDirectory,
    resources: manifest.resources,
    configSchema: probe === "snapshot" ? {
      relativePath: "plugin-config-schema.json",
      bytes: schemaBytes,
      hash: `sha256:${createHash("sha256").update(schemaBytes).digest("hex")}`,
    } : null,
  };
}

async function nodeRequest(): Promise<SandboxLaunchRequest> {
  const workingDirectory = await mkdtemp(join(tmpdir(), "forme-windows-node-"));
  roots.push(workingDirectory);
  const entryBytes = new TextEncoder().encode("setTimeout(() => process.exit(0), 250);\n");
  const manifest: Manifest = {
    manifestVersion: 1,
    plugin: { name: "@example/windows-node", version: "1.0.0", apiVersion: 1 },
    runtime: { kind: "node", entry: "entry.mjs" },
    capabilities: { required: [], optional: [] },
    contributes: { stages: [{ id: "main", consumes: "ContentSource", produces: "ContentNode" }], kinds: [] },
    resources: { maxMemoryMb: 64, maxWallClockMs: 2_000, maxFileDescriptors: 256 },
  };
  return {
    plugin: { manifest, manifestHash: computeManifestHash(manifest, entryBytes), entryBytes },
    stage: manifest.contributes.stages[0]!, instanceId: "node/main", workingDirectory,
    resources: manifest.resources, configSchema: null,
  };
}

async function exitWithStderr(child: Awaited<ReturnType<ReturnType<typeof createWindowsSandboxFactory>["launch"]>>): Promise<{
  readonly exit: Awaited<typeof child.exited>;
  readonly stderr: string;
}> {
  const chunks: Buffer[] = [];
  child.stderr.on("data", chunk => chunks.push(Buffer.from(chunk)));
  const exit = await child.exited;
  return { exit, stderr: Buffer.concat(chunks).toString("utf8") };
}

describe.skipIf(process.platform !== "win32")("Windows native sandbox", () => {
  it("boots the trusted Node runtime after installing the sandbox", async () => {
    const child = await createWindowsSandboxFactory({ launcherPath, readinessTimeoutMs: 2_000 })
      .launch(await nodeRequest());
    const result = await exitWithStderr(child);
    expect(result.exit, result.stderr).toEqual({ code: 0, signal: null });
  });

  it("serializes overlapping trusted-runtime ACL grants and revocations", async () => {
    const [first, second] = await Promise.all([
      createWindowsSandboxFactory({ launcherPath, readinessTimeoutMs: 2_000 }).launch(await nodeRequest()),
      createWindowsSandboxFactory({ launcherPath, readinessTimeoutMs: 2_000 }).launch(await nodeRequest()),
    ]);
    const results = await Promise.all([exitWithStderr(first), exitWithStderr(second)]);
    expect(results.map(result => result.exit), results.map(result => result.stderr).join("\n")).toEqual([
      { code: 0, signal: null },
      { code: 0, signal: null },
    ]);
  }, 15_000);

  it.each(["filesystem", "network", "process", "snapshot", "memory", "descriptors", "environment", "cpu", "wall-clock"])(
    "blocks unauthorized %s access",
    async probe => {
      process.env.FORME_SANDBOX_AMBIENT_SENTINEL = "must-not-cross";
      try {
        const child = await createWindowsSandboxFactory({ launcherPath, readinessTimeoutMs: 2_000 })
          .launch(await request(probe));
        const exit = await child.exited;
        if (["descriptors", "cpu", "wall-clock"].includes(probe)) expect(exit.code).not.toBe(0);
        else expect(exit).toEqual({ code: 0, signal: null });
      } finally {
        delete process.env.FORME_SANDBOX_AMBIENT_SENTINEL;
      }
    },
  );
});
