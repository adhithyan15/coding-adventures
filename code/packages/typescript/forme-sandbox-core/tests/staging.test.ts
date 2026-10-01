import { mkdtemp, readFile, rm, symlink, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { afterEach, describe, expect, it } from "vitest";
import { computeManifestHash, type Manifest } from "@coding-adventures/forme-manifest";
import {
  SandboxLaunchError,
  stageVerifiedPlugin,
  type SandboxLaunchRequest,
} from "../src/index.js";

const roots: string[] = [];

afterEach(async () => {
  await Promise.all(roots.splice(0).map(root => rm(root, { recursive: true, force: true })));
});

function manifest(): Manifest {
  return {
    manifestVersion: 1,
    plugin: { name: "@example/exact", version: "1.0.0", apiVersion: 1 },
    runtime: { kind: "node", entry: "entry.mjs" },
    capabilities: { required: [], optional: [] },
    contributes: {
      stages: [{ id: "main", consumes: "ContentSource", produces: "ContentNode" }],
      kinds: [],
    },
    resources: { maxMemoryMb: 64, maxWallClockMs: 1_000, maxFileDescriptors: 32 },
  };
}

async function request(): Promise<SandboxLaunchRequest> {
  const workingDirectory = await mkdtemp(join(tmpdir(), "forme-sandbox-core-test-"));
  roots.push(workingDirectory);
  const entryBytes = new TextEncoder().encode("export default 1;\n");
  const value = manifest();
  return {
    plugin: {
      manifest: value,
      manifestHash: computeManifestHash(value, entryBytes),
      entryBytes,
    },
    stage: value.contributes.stages[0]!,
    instanceId: "exact/main",
    workingDirectory,
    resources: value.resources,
    configSchema: {
      relativePath: "schemas/config.json",
      bytes: new TextEncoder().encode('{"type":"object"}\n'),
      hash: "sha256:9091a8164f97eaca182b3d06d0e5a59e923c880ebc0148056c453c651f5b46cb",
    },
  };
}

describe("stageVerifiedPlugin", () => {
  it("exclusively stages and re-verifies the exact entry and schema snapshots", async () => {
    const value = await request();
    const staged = await stageVerifiedPlugin(value);

    expect(new Uint8Array(await readFile(staged.entryPath))).toEqual(value.plugin.entryBytes);
    expect(new Uint8Array(await readFile(staged.configSchemaPath!))).toEqual(value.configSchema!.bytes);
    expect(staged.manifestHash).toBe(value.plugin.manifestHash);
    expect(staged.configSchemaHash).toBe(value.configSchema!.hash);
    expect(staged.entryHash).toMatch(/^sha256:[0-9a-f]{64}$/);
  });

  it("rejects a manifest identity that does not cover the supplied entry bytes", async () => {
    const value = await request();
    value.plugin.entryBytes[0] ^= 1;
    await expect(stageVerifiedPlugin(value)).rejects.toMatchObject({
      code: "SNAPSHOT_IDENTITY_MISMATCH",
    });
  });

  it("captures caller-owned snapshots before the first asynchronous boundary", async () => {
    const value = await request();
    const expectedEntry = Uint8Array.from(value.plugin.entryBytes);
    const expectedSchema = Uint8Array.from(value.configSchema!.bytes);
    const stagedPromise = stageVerifiedPlugin(value);
    value.plugin.entryBytes.fill(0);
    value.configSchema!.bytes.fill(0);
    (value.plugin.manifest.plugin as { name: string }).name = "@attacker/mutated";
    (value.stage as { id: string }).id = "mutated";
    (value.resources as { maxMemoryMb?: number }).maxMemoryMb = 1;
    const staged = await stagedPromise;
    expect(new Uint8Array(await readFile(staged.entryPath))).toEqual(expectedEntry);
    expect(new Uint8Array(await readFile(staged.configSchemaPath!))).toEqual(expectedSchema);
  });

  it("rejects a non-empty work directory rather than overwriting a path", async () => {
    const value = await request();
    await writeFile(join(value.workingDirectory, "attacker"), "occupied");
    await expect(stageVerifiedPlugin(value)).rejects.toBeInstanceOf(SandboxLaunchError);
    await expect(readFile(join(value.workingDirectory, "attacker"), "utf8")).resolves.toBe("occupied");
  });

  it("rejects a schema whose sha256 identity does not match its bytes", async () => {
    const value = await request();
    value.configSchema!.bytes[0] ^= 1;
    await expect(stageVerifiedPlugin(value)).rejects.toMatchObject({
      code: "SNAPSHOT_IDENTITY_MISMATCH",
    });
  });

  it("stages schemas at a fixed host-owned name rather than a plugin-selected path", async () => {
    const value = await request();
    (value.configSchema! as { relativePath: string }).relativePath = "plugin-entry.mjs";
    const staged = await stageVerifiedPlugin(value);
    expect(staged.configSchemaPath).toBe(join(value.workingDirectory, ".forme-snapshot", "plugin-config-schema.json"));
    expect(staged.configSchemaPath).not.toBe(staged.entryPath);
  });

  it("rejects missing and symbolic-link working directories", async () => {
    const missing = await request();
    await rm(missing.workingDirectory, { recursive: true });
    await expect(stageVerifiedPlugin(missing)).rejects.toMatchObject({ code: "INVALID_LAUNCH_REQUEST" });

    const linked = await request();
    const target = await mkdtemp(join(tmpdir(), "forme-sandbox-link-target-"));
    roots.push(target);
    await rm(linked.workingDirectory, { recursive: true });
    await symlink(target, linked.workingDirectory);
    await expect(stageVerifiedPlugin(linked)).rejects.toMatchObject({ code: "INVALID_LAUNCH_REQUEST" });
  });

  it("rejects stage and instance identities outside the verified manifest", async () => {
    const wrongStage = await request();
    (wrongStage as { stage: SandboxLaunchRequest["stage"] }).stage = {
      ...wrongStage.stage,
      id: "other",
    };
    await expect(stageVerifiedPlugin(wrongStage)).rejects.toMatchObject({ code: "INVALID_LAUNCH_REQUEST" });

    const emptyInstance = await request();
    (emptyInstance as { instanceId: string }).instanceId = "";
    await expect(stageVerifiedPlugin(emptyInstance)).rejects.toMatchObject({ code: "INVALID_LAUNCH_REQUEST" });

    const nulInstance = await request();
    (nulInstance as { instanceId: string }).instanceId = "bad\0instance";
    await expect(stageVerifiedPlugin(nulInstance)).rejects.toMatchObject({ code: "INVALID_LAUNCH_REQUEST" });
  });

  it.each(["", "../escape.json", "/absolute.json", "bad\0name"])(
    "rejects unsafe schema path %j",
    async relativePath => {
      const value = await request();
      (value.configSchema! as { relativePath: string }).relativePath = relativePath;
      await expect(stageVerifiedPlugin(value)).rejects.toMatchObject({ code: "INVALID_LAUNCH_REQUEST" });
    },
  );

  it.each(["python", "binary"] as const)("uses a runtime-specific staged name for %s", async kind => {
    const value = await request();
    (value.plugin.manifest.runtime as { kind: typeof kind }).kind = kind;
    (value.plugin as { manifestHash: string }).manifestHash = computeManifestHash(
      value.plugin.manifest,
      value.plugin.entryBytes,
    );
    const staged = await stageVerifiedPlugin(value);
    expect(staged.entryPath.endsWith(kind === "python" ? ".py" : "plugin-entry")).toBe(true);
  });
});
