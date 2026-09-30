import { Buffer } from "node:buffer";
import { generateKeypair } from "@coding-adventures/ed25519";
import {
  canonicalManifestToml,
  parseManifest,
  signManifest,
} from "@coding-adventures/forme-manifest";
import { describe, expect, it } from "vitest";
import {
  preparePluginInstallSnapshot,
  type PluginPackageFile,
} from "../src/index.js";

const WHEN = "2026-09-30T13:30:00Z";
const ENTRY = new TextEncoder().encode("export const plugin = true;\n");
const SEED = new Uint8Array(32).fill(0x49);
const MANIFEST_TEXT = `
manifestVersion = 1

[plugin]
name = "@example/minimal"
version = "1.2.3"
apiVersion = 1

[runtime]
kind = "node"
entry = "plugin.mjs"

[[capabilities.required]]
realm = "storage"
scope = "read"
reason = "Read project content"

[[capabilities.optional]]
realm = "env"
scope = "THEME"
reason = "Select an optional theme"

[[contributes.stages]]
id = "minimal"
consumes = "ContentSource"
produces = "ContentNode"
`;

function files(manifestText = MANIFEST_TEXT, extras: readonly PluginPackageFile[] = []): PluginPackageFile[] {
  return [
    { path: "plugin.toml", bytes: new TextEncoder().encode(manifestText) },
    { path: "plugin.mjs", bytes: ENTRY },
    ...extras,
  ];
}

function options(packageFiles: readonly PluginPackageFile[]) {
  return {
    installRoot: "/project/forme-plugins",
    files: packageFiles,
    trustStore: { trustedKeys: [] },
    reviewedGrants: [{ capability: "storage:read", grantedAt: WHEN }],
    capabilityEnvironment: { storageRoot: "/project/content", cacheDir: null },
  } as const;
}

describe("preparePluginInstallSnapshot", () => {
  it("validates, copies, and canonically orders a complete unsigned snapshot", () => {
    const input = files();
    const prepared = preparePluginInstallSnapshot(options(input));
    expect(prepared).toMatchObject({
      pluginName: "@example/minimal",
      pluginVersion: "1.2.3",
      trustTier: "unverified-third-party",
      destinationName: `plugin-${Buffer.from("@example/minimal").toString("base64url")}`,
      fileCount: 3,
      grantedCapabilities: ["storage:read"],
    });
    expect(prepared.files.map(file => file.path)).toEqual(["grants.toml", "plugin.mjs", "plugin.toml"]);
    input[1]!.bytes[0] = 0;
    expect(prepared.files.find(file => file.path === "plugin.mjs")?.bytes[0]).not.toBe(0);
  });

  it("assigns verified trust only to a trusted minimal signed package", () => {
    const keypair = generateKeypair(SEED);
    const manifest = parseManifest(MANIFEST_TEXT);
    const signature = signManifest(manifest, ENTRY, {
      secretSeed: SEED,
      publicKey: keypair.publicKey,
    }, WHEN);
    const signedText = `${canonicalManifestToml(manifest)}\n[signature]\n` +
      `algorithm = ${JSON.stringify(signature.algorithm)}\n` +
      `publicKey = ${JSON.stringify(signature.publicKey)}\n` +
      `signature = ${JSON.stringify(signature.signature)}\n` +
      `signedAt = ${JSON.stringify(signature.signedAt)}\n`;
    const trustStore = {
      trustedKeys: [{ algorithm: "ed25519" as const, publicKey: signature.publicKey, addedAt: WHEN }],
    };
    const minimal = preparePluginInstallSnapshot({ ...options(files(signedText)), trustStore });
    expect(minimal.trustTier).toBe("verified-third-party");
    const withAuxiliary = preparePluginInstallSnapshot({
      ...options(files(signedText, [{ path: "helper.mjs", bytes: new Uint8Array([1]) }])),
      trustStore,
    });
    expect(withAuxiliary.trustTier).toBe("unverified-third-party");
  });

  it.each([
    "",
    "/absolute",
    "../escape",
    "nested/../escape",
    "nested\\windows",
    "./dot",
    "nul\0byte",
    "e\u0301.txt",
    "CON",
  ])("rejects unsafe or non-portable package path %j", path => {
    expect(() => preparePluginInstallSnapshot(options(files(MANIFEST_TEXT, [
      { path, bytes: new Uint8Array([1]) },
    ])))).toThrow();
  });

  it("rejects duplicate and portable-case-colliding package paths", () => {
    expect(() => preparePluginInstallSnapshot(options(files(MANIFEST_TEXT, [
      { path: "plugin.mjs", bytes: new Uint8Array([1]) },
    ])))).toThrow(/duplicate/i);
    expect(() => preparePluginInstallSnapshot(options(files(MANIFEST_TEXT, [
      { path: "PLUGIN.MJS", bytes: new Uint8Array([1]) },
    ])))).toThrow(/collid/i);
  });

  it("rejects missing required or undeclared reviewed capabilities", () => {
    expect(() => preparePluginInstallSnapshot({
      ...options(files()),
      reviewedGrants: [],
    })).toThrow(/required/i);
    expect(() => preparePluginInstallSnapshot({
      ...options(files()),
      reviewedGrants: [{ capability: "process:spawn", grantedAt: WHEN }],
    })).toThrow(/declared/i);
  });

  it("rejects row and byte limits before retaining an unbounded snapshot", () => {
    expect(() => preparePluginInstallSnapshot(options(Array.from({ length: 4_097 }, (_, index) => ({
      path: `file-${index}`,
      bytes: new Uint8Array(),
    }))))).toThrow(/file limit/i);
    expect(() => preparePluginInstallSnapshot(options(files(MANIFEST_TEXT, [
      { path: "huge.bin", bytes: new Uint8Array(16 * 1024 * 1024 + 1) },
    ])))).toThrow(/byte limit/i);
  });
});
