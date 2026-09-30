import { chmod, mkdtemp, mkdir, readFile, readdir, realpath, stat, symlink, writeFile, link } from "node:fs/promises";
import { join } from "node:path";
import { tmpdir } from "node:os";
import { afterEach, describe, expect, it } from "vitest";
import {
  PluginInstallError,
  installPreparedPlugin,
  preparePluginInstallSnapshot,
  type PluginPackageFile,
} from "../src/index.js";

const roots: string[] = [];
const WHEN = "2026-09-30T13:30:00Z";
const MANIFEST = `
manifestVersion = 1
[plugin]
name = "@example/install"
version = "1.0.0"
apiVersion = 1
[runtime]
kind = "node"
entry = "plugin.mjs"
[[capabilities.required]]
realm = "storage"
scope = "read"
reason = "Read content"
[[contributes.stages]]
id = "install"
consumes = "ContentSource"
produces = "ContentNode"
`;

afterEach(async () => {
  const { rm } = await import("node:fs/promises");
  await Promise.all(roots.splice(0).map(root => rm(root, { recursive: true, force: true })));
});

async function fixture(entry = "export const value = 1;\n") {
  const parent = await mkdtemp(join(tmpdir(), "forme-installer-test-"));
  roots.push(parent);
  const requestedRoot = join(parent, "plugins");
  await mkdir(requestedRoot);
  const installRoot = await realpath(requestedRoot);
  const files: PluginPackageFile[] = [
    { path: "plugin.toml", bytes: new TextEncoder().encode(MANIFEST) },
    { path: "plugin.mjs", bytes: new TextEncoder().encode(entry) },
  ];
  const prepared = preparePluginInstallSnapshot({
    installRoot,
    files,
    trustStore: { trustedKeys: [] },
    reviewedGrants: [{ capability: "storage:read", grantedAt: WHEN }],
    capabilityEnvironment: { storageRoot: join(parent, "content"), cacheDir: null },
  });
  return { parent, installRoot, prepared };
}

describe("installPreparedPlugin", () => {
  it("publishes a complete prepared snapshot and leaves no transaction artifacts", async () => {
    const { installRoot, prepared } = await fixture();
    await expect(installPreparedPlugin({ prepared })).resolves.toMatchObject({
      status: "installed",
      pluginName: "@example/install",
    });
    await expect(readFile(join(prepared.destinationPath, "plugin.mjs"), "utf8"))
      .resolves.toBe("export const value = 1;\n");
    if (process.platform !== "win32") {
      expect((await stat(join(prepared.destinationPath, "plugin.mjs"))).mode & 0o777).toBe(0o400);
    }
    expect((await readdir(installRoot)).filter(name => name.startsWith("."))).toEqual([]);
  });

  it("reports an exact reinstall as unchanged", async () => {
    const { prepared } = await fixture();
    await installPreparedPlugin({ prepared });
    await expect(installPreparedPlugin({ prepared })).resolves.toMatchObject({ status: "unchanged" });
  });

  it("installs from the private immutable snapshot when returned bytes are mutated", async () => {
    const { prepared } = await fixture();
    const exposed = prepared.files.find(file => file.path === "plugin.mjs")!;
    exposed.bytes.fill(0x78);
    await installPreparedPlugin({ prepared });
    await expect(readFile(join(prepared.destinationPath, "plugin.mjs"), "utf8"))
      .resolves.toBe("export const value = 1;\n");
  });

  it("atomically replaces a changed install unless immutable mode is requested", async () => {
    const first = await fixture("export const value = 1;\n");
    await installPreparedPlugin({ prepared: first.prepared });
    const changed = preparePluginInstallSnapshot({
      installRoot: first.installRoot,
      files: [
        { path: "plugin.toml", bytes: new TextEncoder().encode(MANIFEST) },
        { path: "plugin.mjs", bytes: new TextEncoder().encode("export const value = 2;\n") },
      ],
      trustStore: { trustedKeys: [] },
      reviewedGrants: [{ capability: "storage:read", grantedAt: WHEN }],
      capabilityEnvironment: { storageRoot: join(first.parent, "content"), cacheDir: null },
    });
    await expect(installPreparedPlugin({ prepared: changed, mode: "immutable" }))
      .rejects.toMatchObject({ code: "TARGET_EXISTS" });
    await expect(installPreparedPlugin({ prepared: changed })).resolves.toMatchObject({ status: "updated" });
    await expect(readFile(join(changed.destinationPath, "plugin.mjs"), "utf8"))
      .resolves.toBe("export const value = 2;\n");
  });

  it("rejects symlink and hardlink content in an existing target", async () => {
    const symlinkCase = await fixture();
    await mkdir(symlinkCase.prepared.destinationPath);
    await symlink("../outside", join(symlinkCase.prepared.destinationPath, "plugin.mjs"));
    await expect(installPreparedPlugin({ prepared: symlinkCase.prepared }))
      .rejects.toMatchObject({ code: "TARGET_UNSAFE" });

    const hardlinkCase = await fixture();
    await mkdir(hardlinkCase.prepared.destinationPath);
    const outside = join(hardlinkCase.parent, "outside");
    await writeFile(outside, "outside");
    await link(outside, join(hardlinkCase.prepared.destinationPath, "plugin.mjs"));
    await expect(installPreparedPlugin({ prepared: hardlinkCase.prepared }))
      .rejects.toMatchObject({ code: "TARGET_UNSAFE" });
  });

  it("rejects an unsafe root or target and an already-held lock", async () => {
    const rootCase = await fixture();
    const movedRoot = `${rootCase.installRoot}-real`;
    const { rename, rm } = await import("node:fs/promises");
    await rename(rootCase.installRoot, movedRoot);
    await symlink(movedRoot, rootCase.installRoot);
    await expect(installPreparedPlugin({ prepared: rootCase.prepared }))
      .rejects.toMatchObject({ code: "ROOT_UNSAFE" });
    await rm(rootCase.installRoot);

    const targetCase = await fixture();
    await writeFile(targetCase.prepared.destinationPath, "not a directory");
    await expect(installPreparedPlugin({ prepared: targetCase.prepared }))
      .rejects.toMatchObject({ code: "TARGET_UNSAFE" });

    const lockCase = await fixture();
    await mkdir(join(lockCase.installRoot, `.${lockCase.prepared.destinationName}.forme-lock`));
    await expect(installPreparedPlugin({ prepared: lockCase.prepared }))
      .rejects.toMatchObject({ code: "TARGET_BUSY" });
  });

  it("honours cancellation without leaving stage, backup, or lock paths", async () => {
    const { installRoot, prepared } = await fixture();
    const controller = new AbortController();
    controller.abort(new Error("cancelled"));
    await expect(installPreparedPlugin({ prepared, signal: controller.signal }))
      .rejects.toBeInstanceOf(PluginInstallError);
    expect((await readdir(installRoot)).filter(name => name.startsWith("."))).toEqual([]);
  });

  it("cleans a partial stage and restores the old install when cancellation lands during replacement", async () => {
    const staged = await fixture();
    let checks = 0;
    const stageSignal = {
      get aborted() { checks += 1; return checks === 3; },
      reason: new Error("cancel stage"),
    } as AbortSignal;
    await expect(installPreparedPlugin({ prepared: staged.prepared, signal: stageSignal }))
      .rejects.toMatchObject({ code: "ABORTED" });
    expect((await readdir(staged.installRoot)).filter(name => name.startsWith("."))).toEqual([]);

    const replacing = await fixture("export const value = 1;\n");
    await installPreparedPlugin({ prepared: replacing.prepared });
    const changed = preparePluginInstallSnapshot({
      installRoot: replacing.installRoot,
      files: [
        { path: "plugin.toml", bytes: new TextEncoder().encode(MANIFEST) },
        { path: "plugin.mjs", bytes: new TextEncoder().encode("export const value = 2;\n") },
      ],
      trustStore: { trustedKeys: [] },
      reviewedGrants: [{ capability: "storage:read", grantedAt: WHEN }],
      capabilityEnvironment: { storageRoot: join(replacing.parent, "content"), cacheDir: null },
    });
    checks = 0;
    const commitSignal = {
      get aborted() { checks += 1; return checks === 15; },
      reason: new Error("cancel commit"),
    } as AbortSignal;
    await expect(installPreparedPlugin({ prepared: changed, signal: commitSignal }))
      .rejects.toMatchObject({ code: "ABORTED" });
    await expect(readFile(join(changed.destinationPath, "plugin.mjs"), "utf8"))
      .resolves.toBe("export const value = 1;\n");
    expect((await readdir(replacing.installRoot)).filter(name => name.startsWith("."))).toEqual([]);
  });

  it("materializes nested files and directories", async () => {
    const { parent, installRoot } = await fixture();
    const manifest = MANIFEST.replace('entry = "plugin.mjs"', 'entry = "./nested/plugin.mjs"');
    const prepared = preparePluginInstallSnapshot({
      installRoot,
      files: [
        { path: "plugin.toml", bytes: new TextEncoder().encode(manifest) },
        { path: "nested/plugin.mjs", bytes: new TextEncoder().encode("nested") },
      ],
      trustStore: { trustedKeys: [] },
      reviewedGrants: [{ capability: "storage:read", grantedAt: WHEN }],
      capabilityEnvironment: { storageRoot: join(parent, "content"), cacheDir: null },
    });
    await installPreparedPlugin({ prepared });
    await expect(readFile(join(prepared.destinationPath, "nested/plugin.mjs"), "utf8")).resolves.toBe("nested");
    await expect(installPreparedPlugin({ prepared })).resolves.toMatchObject({ status: "unchanged" });
  });

  it("detects extra, missing, size-mismatched, content-mismatched, and invalid target files", async () => {
    const extra = await fixture();
    await installPreparedPlugin({ prepared: extra.prepared });
    await writeFile(join(extra.prepared.destinationPath, "extra"), "x");
    await expect(installPreparedPlugin({ prepared: extra.prepared, mode: "immutable" }))
      .rejects.toMatchObject({ code: "TARGET_EXISTS" });

    const missing = await fixture();
    await installPreparedPlugin({ prepared: missing.prepared });
    const { rm } = await import("node:fs/promises");
    await rm(join(missing.prepared.destinationPath, "plugin.mjs"));
    await expect(installPreparedPlugin({ prepared: missing.prepared, mode: "immutable" }))
      .rejects.toMatchObject({ code: "TARGET_EXISTS" });

    const sized = await fixture();
    await installPreparedPlugin({ prepared: sized.prepared });
    await chmod(join(sized.prepared.destinationPath, "plugin.mjs"), 0o600);
    await writeFile(join(sized.prepared.destinationPath, "plugin.mjs"), "short");
    await expect(installPreparedPlugin({ prepared: sized.prepared, mode: "immutable" }))
      .rejects.toMatchObject({ code: "TARGET_EXISTS" });

    const content = await fixture();
    await installPreparedPlugin({ prepared: content.prepared });
    const original = await readFile(join(content.prepared.destinationPath, "plugin.mjs"));
    await chmod(join(content.prepared.destinationPath, "plugin.mjs"), 0o600);
    await writeFile(join(content.prepared.destinationPath, "plugin.mjs"), Buffer.alloc(original.length, 0x78));
    await expect(installPreparedPlugin({ prepared: content.prepared, mode: "immutable" }))
      .rejects.toMatchObject({ code: "TARGET_EXISTS" });

    const invalid = await fixture();
    await mkdir(invalid.prepared.destinationPath);
    await writeFile(join(invalid.prepared.destinationPath, "bad name"), "x");
    await expect(installPreparedPlugin({ prepared: invalid.prepared }))
      .rejects.toMatchObject({ code: "TARGET_UNSAFE" });
  });

  it("rejects a missing or non-directory install root", async () => {
    const missing = await fixture();
    const { rm } = await import("node:fs/promises");
    await rm(missing.installRoot, { recursive: true });
    await expect(installPreparedPlugin({ prepared: missing.prepared }))
      .rejects.toMatchObject({ code: "ROOT_UNSAFE" });

    const fileRoot = await fixture();
    await rm(fileRoot.installRoot, { recursive: true });
    await writeFile(fileRoot.installRoot, "file");
    await expect(installPreparedPlugin({ prepared: fileRoot.prepared }))
      .rejects.toMatchObject({ code: "ROOT_UNSAFE" });
  });

  it("bounds cancellation, target depth, and target entry scans", async () => {
    const cancelled = await fixture();
    await installPreparedPlugin({ prepared: cancelled.prepared });
    let checks = 0;
    const signal = {
      get aborted() { checks += 1; return checks === 3; },
      reason: new Error("cancel scan"),
    } as AbortSignal;
    await expect(installPreparedPlugin({ prepared: cancelled.prepared, signal }))
      .rejects.toMatchObject({ code: "ABORTED" });

    const deep = await fixture();
    await mkdir(join(deep.prepared.destinationPath, ...Array.from({ length: 258 }, () => "a")), { recursive: true });
    await expect(installPreparedPlugin({ prepared: deep.prepared }))
      .rejects.toMatchObject({ code: "TARGET_UNSAFE" });

    const crowded = await fixture();
    await mkdir(crowded.prepared.destinationPath);
    await Promise.all(Array.from({ length: 4_098 }, (_, index) =>
      writeFile(join(crowded.prepared.destinationPath, `f-${index}`), "")));
    await expect(installPreparedPlugin({ prepared: crowded.prepared }))
      .rejects.toMatchObject({ code: "TARGET_UNSAFE" });
  });
});
