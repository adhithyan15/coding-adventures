import { generateKeypair } from "@coding-adventures/ed25519";
import { canonicalManifestToml, parseManifest, signManifest } from "@coding-adventures/forme-manifest";
import { describe, expect, it } from "vitest";
import {
  installPreparedPlugin,
  preparePluginInstallSnapshot,
  type PluginPackageFile,
} from "../src/index.js";

const ENTRY = new TextEncoder().encode("entry");
const WHEN = "2026-09-30T13:30:00Z";
const BASE = `
manifestVersion = 1
[plugin]
name = "@example/validation"
version = "1.0.0"
apiVersion = 2
[runtime]
kind = "node"
entry = "plugin.mjs"
[[capabilities.required]]
realm = "storage"
scope = "read"
reason = "read"
[[capabilities.optional]]
realm = "filesystem"
scope = "read"
detail = "$storageRoot"
reason = "optional"
[[contributes.stages]]
id = "s"
consumes = "ContentSource"
produces = "ContentNode"
`;

function packageFiles(manifest = BASE, extras: readonly PluginPackageFile[] = []): PluginPackageFile[] {
  return [
    { path: "plugin.toml", bytes: new TextEncoder().encode(manifest) },
    { path: "plugin.mjs", bytes: ENTRY },
    ...extras,
  ];
}

function opts(files: readonly PluginPackageFile[] = packageFiles()) {
  return {
    installRoot: "/tmp/plugins",
    files,
    trustStore: { trustedKeys: [] },
    reviewedGrants: [{ capability: "storage:read", grantedAt: WHEN, note: "reviewed" }],
    capabilityEnvironment: { storageRoot: "/tmp/content", cacheDir: "/tmp/cache" },
  } as const;
}

function signedText(manifestText = BASE, entry = ENTRY): string {
  const seed = new Uint8Array(32).fill(7);
  const keypair = generateKeypair(seed);
  const manifest = parseManifest(manifestText);
  const signature = signManifest(manifest, entry, { secretSeed: seed, publicKey: keypair.publicKey }, WHEN);
  return `${canonicalManifestToml(manifest)}\n[signature]\n` +
    `algorithm = ${JSON.stringify(signature.algorithm)}\n` +
    `publicKey = ${JSON.stringify(signature.publicKey)}\n` +
    `signature = ${JSON.stringify(signature.signature)}\n` +
    `signedAt = ${JSON.stringify(signature.signedAt)}\n`;
}

describe("snapshot defensive validation", () => {
  it("rejects malformed roots, rows, and authority inputs", () => {
    expect(() => preparePluginInstallSnapshot(null as never)).toThrow(/options/i);
    expect(() => preparePluginInstallSnapshot({ ...opts(), files: null as never })).toThrow(/files/i);
    expect(() => preparePluginInstallSnapshot({ ...opts(), installRoot: "relative" })).toThrow(/absolute/i);
    expect(() => preparePluginInstallSnapshot({ ...opts(), installRoot: "/tmp/\0bad" })).toThrow(/absolute/i);
    expect(() => preparePluginInstallSnapshot(opts([{ path: "x", bytes: ENTRY }]))).toThrow(/plugin.toml/i);
    expect(() => preparePluginInstallSnapshot(opts([
      ...packageFiles(),
      { path: "grants.toml", bytes: ENTRY },
    ]))).toThrow(/host-owned/i);
    expect(() => preparePluginInstallSnapshot(opts([
      ...packageFiles(),
      { path: "GRANTS.TOML", bytes: ENTRY },
    ]))).toThrow(/host-owned/i);
    expect(() => preparePluginInstallSnapshot(opts([
      ...packageFiles(),
      { path: "grants.toml/child", bytes: ENTRY },
    ]))).toThrow(/host-owned/i);
    expect(() => preparePluginInstallSnapshot(opts([
      { path: "plugin.toml", bytes: new Uint8Array([0xff]) },
      { path: "plugin.mjs", bytes: ENTRY },
    ]))).toThrow(/validation/i);
    expect(() => preparePluginInstallSnapshot(opts([
      { path: "plugin.toml", bytes: new TextEncoder().encode(BASE) },
    ]))).toThrow(/runtime entry/i);
    expect(() => preparePluginInstallSnapshot({ ...opts(), reviewedGrants: null as never })).toThrow(/reviewedGrants/i);
    expect(() => preparePluginInstallSnapshot({
      ...opts(),
      reviewedGrants: [
        { capability: "storage:read", grantedAt: WHEN },
        { capability: "storage:read", grantedAt: WHEN },
      ],
    })).toThrow(/duplicate/i);
  });

  it("rejects malformed grant rows and capability environments", () => {
    expect(() => preparePluginInstallSnapshot({ ...opts(), capabilityEnvironment: null as never })).toThrow(/environment/i);
    expect(() => preparePluginInstallSnapshot({
      ...opts(), capabilityEnvironment: { storageRoot: "relative", cacheDir: null },
    })).toThrow(/storageRoot/i);
    expect(() => preparePluginInstallSnapshot({
      ...opts(), capabilityEnvironment: { storageRoot: "/tmp/content", cacheDir: "relative" },
    })).toThrow(/cacheDir/i);
    expect(() => preparePluginInstallSnapshot({ ...opts(), reviewedGrants: [null as never] })).toThrow(/object/i);
    expect(() => preparePluginInstallSnapshot({
      ...opts(), reviewedGrants: [{ capability: 7 as never, grantedAt: WHEN }],
    })).toThrow(/capability/i);
    expect(() => preparePluginInstallSnapshot({
      ...opts(), reviewedGrants: [{ capability: "storage:read", grantedAt: 7 as never }],
    })).toThrow(/grantedAt/i);
    expect(() => preparePluginInstallSnapshot({
      ...opts(), reviewedGrants: [{ capability: "storage:read", grantedAt: WHEN, note: 7 as never }],
    })).toThrow(/note/i);
    expect(preparePluginInstallSnapshot({
      ...opts(),
      capabilityEnvironment: { storageRoot: "/tmp:portable", cacheDir: null },
    }).pluginName).toBe("@example/validation");
    expect(preparePluginInstallSnapshot({
      ...opts(),
      capabilityEnvironment: { storageRoot: "/tmp/bad path", cacheDir: null },
    }).pluginName).toBe("@example/validation");
  });

  it("rejects forged prepared snapshots and invalid modes", async () => {
    await expect(installPreparedPlugin(null as never)).rejects.toMatchObject({ code: "INVALID_SNAPSHOT" });
    await expect(installPreparedPlugin({ prepared: {} as never })).rejects.toMatchObject({ code: "INVALID_SNAPSHOT" });
    const prepared = preparePluginInstallSnapshot(opts());
    await expect(installPreparedPlugin({ prepared, mode: "other" as never }))
      .rejects.toMatchObject({ code: "INVALID_SNAPSHOT" });
  });

  it("rejects a signed manifest whose entry no longer verifies", () => {
    const signed = signedText();
    expect(() => preparePluginInstallSnapshot(opts([
      { path: "plugin.toml", bytes: new TextEncoder().encode(signed) },
      { path: "plugin.mjs", bytes: new TextEncoder().encode("tampered") },
    ]))).toThrow(/signature verification/i);
  });

  it.each([
    "a//b",
    "a/./b",
    "a/../b",
    "a b",
    "a.",
    "__proto__",
    `${"a".repeat(256)}.txt`,
    `${"a".repeat(2049)}`,
  ])("rejects additional nonportable path shape %j", path => {
    expect(() => preparePluginInstallSnapshot(opts(packageFiles(BASE, [{ path, bytes: ENTRY }])))).toThrow();
  });

  it("rejects prefix collisions and malformed file rows", () => {
    expect(() => preparePluginInstallSnapshot(opts(packageFiles(BASE, [
      { path: "tree", bytes: ENTRY },
      { path: "tree/leaf", bytes: ENTRY },
    ])))).toThrow(/prefix collision/i);
    expect(() => preparePluginInstallSnapshot(opts(packageFiles(BASE, [
      { path: "a", bytes: ENTRY },
      { path: "a-x", bytes: ENTRY },
      { path: "a/b", bytes: ENTRY },
    ])))).toThrow(/prefix collision/i);
    expect(() => preparePluginInstallSnapshot(opts(packageFiles(BASE, [
      { path: "Tree", bytes: ENTRY },
      { path: "tree/leaf", bytes: ENTRY },
    ])))).toThrow(/prefix collision/i);
    expect(() => preparePluginInstallSnapshot(opts([null as never]))).toThrow(/object/i);
    expect(() => preparePluginInstallSnapshot(opts([{ path: "x", bytes: "bad" as never }]))).toThrow(/Uint8Array/i);
  });

  it("enforces the aggregate byte limit", () => {
    const chunk = new Uint8Array(16 * 1024 * 1024);
    const extras = Array.from({ length: 8 }, (_, index) => ({ path: `chunk-${index}`, bytes: chunk }));
    expect(() => preparePluginInstallSnapshot(opts(packageFiles(BASE, extras)))).toThrow(/total byte limit/i);
  });

  it("bounds grant preprocessing, directory topology, depth, and destination names", () => {
    expect(() => preparePluginInstallSnapshot({
      ...opts(),
      reviewedGrants: Array.from({ length: 4_097 }, () => ({
        capability: "storage:read",
        grantedAt: WHEN,
      })),
    })).toThrow(/decision limit/i);

    const deepPath = `${Array.from({ length: 257 }, () => "a").join("/")}/leaf`;
    expect(() => preparePluginInstallSnapshot(opts(packageFiles(BASE, [{ path: deepPath, bytes: ENTRY }]))))
      .toThrow(/depth/i);

    const wideDirectories = Array.from({ length: 17 }, (_, group) => ({
      path: `${Array.from({ length: 256 }, (_, depth) => `x${group.toString(36)}${depth.toString(36)}`).join("/")}/leaf`,
      bytes: ENTRY,
    }));
    expect(() => preparePluginInstallSnapshot(opts(packageFiles(BASE, wideDirectories))))
      .toThrow(/directory limit/i);

    const longName = `@example/${"a".repeat(180)}`;
    const longManifest = BASE.replace("@example/validation", longName);
    expect(() => preparePluginInstallSnapshot(opts(packageFiles(longManifest))))
      .toThrow(/basename/i);
  });

  it("selects binary platform entries with canonical OS and architecture names", () => {
    const binary = BASE.replace(
      'kind = "node"\nentry = "plugin.mjs"',
      'kind = "binary"\nentry = "fallback"\n[runtime.platforms]\nwindows-x86_64 = "bin/win"\ndarwin-aarch64 = "bin/mac"\nlinux-x86_64 = "bin/linux-x64"\nlinux-riscv64 = "bin/linux"',
    );
    const files = [
      { path: "plugin.toml", bytes: new TextEncoder().encode(binary) },
      { path: "bin/win", bytes: ENTRY },
      { path: "bin/mac", bytes: ENTRY },
      { path: "bin/linux-x64", bytes: ENTRY },
      { path: "bin/linux", bytes: ENTRY },
    ];
    expect(preparePluginInstallSnapshot({ ...opts(files), platform: { os: "win32", arch: "x64" } }).pluginName)
      .toBe("@example/validation");
    expect(preparePluginInstallSnapshot({ ...opts(files), platform: { os: "darwin", arch: "arm64" } }).pluginName)
      .toBe("@example/validation");
    expect(preparePluginInstallSnapshot({ ...opts(files), platform: { os: "linux", arch: "x64" } }).pluginName)
      .toBe("@example/validation");
    expect(preparePluginInstallSnapshot({ ...opts(files), platform: { os: "linux", arch: "riscv64" } }).pluginName)
      .toBe("@example/validation");
    expect(preparePluginInstallSnapshot(opts(files)).pluginName).toBe("@example/validation");
    expect(() => preparePluginInstallSnapshot({ ...opts(files), platform: { os: "freebsd", arch: "x64" } }))
      .toThrow(/no entry/i);
  });

  it("requires referenced schemas and rejects signed manifests with external schemas", () => {
    const schemaManifest = BASE.replace(
      'produces = "ContentNode"',
      'produces = "ContentNode"\nconfigSchema = "./schemas/stage.json"\n[[contributes.kinds]]\nname = "ext:item"\nversion = "1.0"\nschema = "./schemas/kind.json"',
    );
    expect(() => preparePluginInstallSnapshot(opts(packageFiles(schemaManifest)))).toThrow(/missing schema/i);
    const schemas = [
      { path: "schemas/stage.json", bytes: new TextEncoder().encode("{}") },
      { path: "schemas/kind.json", bytes: new TextEncoder().encode("{}") },
    ];
    expect(preparePluginInstallSnapshot(opts(packageFiles(schemaManifest, schemas))).fileCount).toBe(5);
    const signed = signedText(schemaManifest);
    expect(() => preparePluginInstallSnapshot(opts(packageFiles(signed, schemas))))
      .toThrow(/signed plugins cannot reference schemas/i);
  });
});
