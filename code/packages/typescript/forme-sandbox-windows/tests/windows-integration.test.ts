import { createHash } from "node:crypto";
import { execFile } from "node:child_process";
import { mkdir, mkdtemp, readFile, rm, symlink, writeFile } from "node:fs/promises";
import { homedir, tmpdir } from "node:os";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { promisify } from "node:util";
import { afterEach, describe, expect, it } from "vitest";
import { computeManifestHash, type Manifest } from "@coding-adventures/forme-manifest";
import type { SandboxLaunchRequest } from "@coding-adventures/forme-sandbox-core";
import { createWindowsInstallAclVerifier, createWindowsSandboxFactory } from "../src/index.js";

const packageRoot = dirname(dirname(fileURLToPath(import.meta.url)));
const launcherPath = join(packageRoot, "native", "forme-sandbox-windows.exe");
const probePath = join(packageRoot, "native", "forme-sandbox-probe.exe");
const roots: string[] = [];
const execFileAsync = promisify(execFile);
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
  it("accepts an owned tree and rejects reparse points or untrusted writers", async () => {
    // Hosted-runner temp roots are intentionally shared and therefore fail
    // the verifier's ancestor replacement-authority check. The user profile
    // gives this positive fixture the same trusted ancestry required in use.
    const root = await mkdtemp(join(homedir(), "forme-windows-acl-"));
    roots.push(root);
    await mkdir(join(root, "plugin"));
    await writeFile(join(root, "plugin", "entry.mjs"), "process.exit(0);\n");
    const verify = createWindowsInstallAclVerifier({ launcherPath });
    expect(await verify(root, "install-root")).toBe(true);
    expect(await verify(join(root, "plugin"), "existing-target-tree")).toBe(true);

    const transactionRoot = join(root, "transaction-root");
    await mkdir(transactionRoot);
    await execFileAsync("icacls.exe", [transactionRoot, "/grant", "*S-1-1-0:(OI)(CI)(IO)M", "/Q"]);
    expect(await verify(transactionRoot, "install-root")).toBe(false);
    expect(await verify(transactionRoot, "existing-target-tree")).toBe(true);
    await execFileAsync("icacls.exe", [transactionRoot, "/remove:g", "*S-1-1-0", "/Q"]);
    expect(await verify(transactionRoot, "install-root")).toBe(true);

    await execFileAsync("icacls.exe", [root, "/grant", "*S-1-1-0:(NP)M", "/Q"]);
    expect(await verify(join(root, "plugin"), "install-root")).toBe(false);
    await execFileAsync("icacls.exe", [root, "/remove:g", "*S-1-1-0", "/Q"]);

    const outside = await mkdtemp(join(tmpdir(), "forme-windows-acl-outside-"));
    roots.push(outside);
    await symlink(outside, join(root, "plugin", "link"), "junction");
    expect(await verify(join(root, "plugin"), "existing-target-tree")).toBe(false);
    await rm(join(root, "plugin", "link"), { force: true });

    await execFileAsync("icacls.exe", [root, "/grant", "*S-1-1-0:(OI)(CI)M", "/T", "/Q"]);
    expect(await verify(root, "existing-target-tree")).toBe(false);
  });

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
