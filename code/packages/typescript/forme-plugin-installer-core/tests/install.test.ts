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

function installForTest(options: Parameters<typeof installPreparedPlugin>[0]) {
  return installPreparedPlugin({
    ...options,
    // The production contract requires the native host to verify Windows ACLs.
    // These filesystem tests supply that already-reviewed host decision explicitly.
    verifyWindowsAcl: async () => true,
  });
}

describe("installPreparedPlugin", () => {
  it.runIf(process.platform === "win32")(
    "fails closed without an ACL verifier and checks both protected scopes",
    async () => {
      const { prepared } = await fixture();
      await expect(installPreparedPlugin({ prepared }))
        .rejects.toMatchObject({ code: "ROOT_UNSAFE" });
      await expect(installPreparedPlugin({ prepared, verifyWindowsAcl: async () => false }))
        .rejects.toMatchObject({ code: "ROOT_UNSAFE" });

      const scopes: string[] = [];
      const verifyWindowsAcl = async (_path: string, scope: "install-root" | "existing-target-tree") => {
        scopes.push(scope);
        return true;
      };
      await installPreparedPlugin({ prepared, verifyWindowsAcl });
      await installPreparedPlugin({ prepared, verifyWindowsAcl });
      expect(scopes).toEqual(["install-root", "install-root", "existing-target-tree"]);
    },
  );

  it("publishes a complete prepared snapshot and leaves no transaction artifacts", async () => {
    const { installRoot, prepared } = await fixture();
    await expect(installForTest({ prepared })).resolves.toMatchObject({
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
    await installForTest({ prepared });
    await expect(installForTest({ prepared })).resolves.toMatchObject({ status: "unchanged" });
  });

  it("installs from the private immutable snapshot when returned bytes are mutated", async () => {
    const { prepared } = await fixture();
    const exposed = prepared.files.find(file => file.path === "plugin.mjs")!;
    exposed.bytes.fill(0x78);
    await installForTest({ prepared });
    await expect(readFile(join(prepared.destinationPath, "plugin.mjs"), "utf8"))
      .resolves.toBe("export const value = 1;\n");
  });

  it("atomically replaces a changed install unless immutable mode is requested", async () => {
    const first = await fixture("export const value = 1;\n");
    await installForTest({ prepared: first.prepared });
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
    await expect(installForTest({ prepared: changed, mode: "immutable" }))
      .rejects.toMatchObject({ code: "TARGET_EXISTS" });
    await expect(installForTest({ prepared: changed })).resolves.toMatchObject({ status: "updated" });
    await expect(readFile(join(changed.destinationPath, "plugin.mjs"), "utf8"))
      .resolves.toBe("export const value = 2;\n");
  });

  it("rejects symlink and hardlink content in an existing target", async () => {
    const symlinkCase = await fixture();
    await mkdir(symlinkCase.prepared.destinationPath);
    await symlink("../outside", join(symlinkCase.prepared.destinationPath, "plugin.mjs"));
    await expect(installForTest({ prepared: symlinkCase.prepared }))
      .rejects.toMatchObject({ code: "TARGET_UNSAFE" });

    const hardlinkCase = await fixture();
    await mkdir(hardlinkCase.prepared.destinationPath);
    const outside = join(hardlinkCase.parent, "outside");
    await writeFile(outside, "outside");
    await link(outside, join(hardlinkCase.prepared.destinationPath, "plugin.mjs"));
    await expect(installForTest({ prepared: hardlinkCase.prepared }))
      .rejects.toMatchObject({ code: "TARGET_UNSAFE" });
  });

  it("rejects an unsafe root or target and an already-held lock", async () => {
    const rootCase = await fixture();
    const movedRoot = `${rootCase.installRoot}-real`;
    const { rename, rm } = await import("node:fs/promises");
    await rename(rootCase.installRoot, movedRoot);
    await symlink(movedRoot, rootCase.installRoot);
    await expect(installForTest({ prepared: rootCase.prepared }))
      .rejects.toMatchObject({ code: "ROOT_UNSAFE" });
    await rm(rootCase.installRoot);

    const targetCase = await fixture();
    await writeFile(targetCase.prepared.destinationPath, "not a directory");
    await expect(installForTest({ prepared: targetCase.prepared }))
      .rejects.toMatchObject({ code: "TARGET_UNSAFE" });

    const lockCase = await fixture();
    await mkdir(join(lockCase.installRoot, `.${lockCase.prepared.destinationName}.forme-lock`));
    await expect(installForTest({ prepared: lockCase.prepared }))
      .rejects.toMatchObject({ code: "TARGET_BUSY" });
  });

  it("honours cancellation without leaving stage, backup, or lock paths", async () => {
    const { installRoot, prepared } = await fixture();
    const controller = new AbortController();
    controller.abort(new Error("cancelled"));
    await expect(installForTest({ prepared, signal: controller.signal }))
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
    await expect(installForTest({ prepared: staged.prepared, signal: stageSignal }))
      .rejects.toMatchObject({ code: "ABORTED" });
    expect((await readdir(staged.installRoot)).filter(name => name.startsWith("."))).toEqual([]);

    const replacing = await fixture("export const value = 1;\n");
    await installForTest({ prepared: replacing.prepared });
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
    await expect(installForTest({ prepared: changed, signal: commitSignal }))
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
    await installForTest({ prepared });
    await expect(readFile(join(prepared.destinationPath, "nested/plugin.mjs"), "utf8")).resolves.toBe("nested");
    await expect(installForTest({ prepared })).resolves.toMatchObject({ status: "unchanged" });
  });

  it("detects extra, missing, size-mismatched, content-mismatched, and invalid target files", async () => {
    const extra = await fixture();
    await installForTest({ prepared: extra.prepared });
    await writeFile(join(extra.prepared.destinationPath, "extra"), "x");
    await expect(installForTest({ prepared: extra.prepared, mode: "immutable" }))
      .rejects.toMatchObject({ code: "TARGET_EXISTS" });

    const missing = await fixture();
    await installForTest({ prepared: missing.prepared });
    const { rm } = await import("node:fs/promises");
    await rm(join(missing.prepared.destinationPath, "plugin.mjs"));
    await expect(installForTest({ prepared: missing.prepared, mode: "immutable" }))
      .rejects.toMatchObject({ code: "TARGET_EXISTS" });

    const sized = await fixture();
    await installForTest({ prepared: sized.prepared });
    await chmod(join(sized.prepared.destinationPath, "plugin.mjs"), 0o600);
    await writeFile(join(sized.prepared.destinationPath, "plugin.mjs"), "short");
    await expect(installForTest({ prepared: sized.prepared, mode: "immutable" }))
      .rejects.toMatchObject({ code: "TARGET_EXISTS" });

    const content = await fixture();
    await installForTest({ prepared: content.prepared });
    const original = await readFile(join(content.prepared.destinationPath, "plugin.mjs"));
    await chmod(join(content.prepared.destinationPath, "plugin.mjs"), 0o600);
    await writeFile(join(content.prepared.destinationPath, "plugin.mjs"), Buffer.alloc(original.length, 0x78));
    await expect(installForTest({ prepared: content.prepared, mode: "immutable" }))
      .rejects.toMatchObject({ code: "TARGET_EXISTS" });

    const invalid = await fixture();
    await mkdir(invalid.prepared.destinationPath);
    await writeFile(join(invalid.prepared.destinationPath, "bad name"), "x");
    await expect(installForTest({ prepared: invalid.prepared }))
      .rejects.toMatchObject({ code: "TARGET_UNSAFE" });
  });

  it("rejects a missing or non-directory install root", async () => {
    const missing = await fixture();
    const { rm } = await import("node:fs/promises");
    await rm(missing.installRoot, { recursive: true });
    await expect(installForTest({ prepared: missing.prepared }))
      .rejects.toMatchObject({ code: "ROOT_UNSAFE" });

    const fileRoot = await fixture();
    await rm(fileRoot.installRoot, { recursive: true });
    await writeFile(fileRoot.installRoot, "file");
    await expect(installForTest({ prepared: fileRoot.prepared }))
      .rejects.toMatchObject({ code: "ROOT_UNSAFE" });
  });

  it.runIf(process.platform !== "win32")("rejects a group- or world-writable install root", async () => {
    const unsafe = await fixture();
    await chmod(unsafe.installRoot, 0o777);
    await expect(installForTest({ prepared: unsafe.prepared }))
      .rejects.toMatchObject({ code: "ROOT_UNSAFE" });
  });

  it.runIf(process.platform !== "win32")("rejects shared-writable existing target content", async () => {
    const root = await fixture();
    await installForTest({ prepared: root.prepared });
    await chmod(root.prepared.destinationPath, 0o770);
    await expect(installForTest({ prepared: root.prepared }))
      .rejects.toMatchObject({ code: "TARGET_UNSAFE" });

    const nested = await fixture();
    await installForTest({ prepared: nested.prepared });
    await chmod(join(nested.prepared.destinationPath, "plugin.mjs"), 0o420);
    await expect(installForTest({ prepared: nested.prepared }))
      .rejects.toMatchObject({ code: "TARGET_UNSAFE" });
  });

  it("bounds cancellation, target depth, and target entry scans", async () => {
    const cancelled = await fixture();
    await installForTest({ prepared: cancelled.prepared });
    let checks = 0;
    const signal = {
      get aborted() { checks += 1; return checks === 3; },
      reason: new Error("cancel scan"),
    } as AbortSignal;
    await expect(installForTest({ prepared: cancelled.prepared, signal }))
      .rejects.toMatchObject({ code: "ABORTED" });

    const deep = await fixture();
    await mkdir(join(deep.prepared.destinationPath, ...Array.from({ length: 258 }, () => "a")), { recursive: true });
    await expect(installForTest({ prepared: deep.prepared }))
      .rejects.toMatchObject({ code: "TARGET_UNSAFE" });

    const crowded = await fixture();
    await mkdir(crowded.prepared.destinationPath);
    await Promise.all(Array.from({ length: 4_098 }, (_, index) =>
      writeFile(join(crowded.prepared.destinationPath, `f-${index}`), "")));
    await expect(installForTest({ prepared: crowded.prepared }))
      .rejects.toMatchObject({ code: "TARGET_UNSAFE" });

    const directoryCrowd = await fixture();
    await mkdir(directoryCrowd.prepared.destinationPath);
    await Promise.all(Array.from({ length: 4_097 }, (_, index) =>
      mkdir(join(directoryCrowd.prepared.destinationPath, `d-${index}`))));
    await expect(installForTest({ prepared: directoryCrowd.prepared }))
      .rejects.toMatchObject({ code: "TARGET_UNSAFE" });
  }, 15_000);
});
