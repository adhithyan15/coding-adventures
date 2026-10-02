import { chmod, mkdtemp, mkdir, readFile, rm, symlink, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { afterEach, describe, expect, it } from "vitest";
import {
  executePluginInstall,
  escapeTerminalText,
  snapshotPluginDirectory,
  type CapabilityReview,
} from "../src/index.js";

const roots: string[] = [];
const MANIFEST = `
manifestVersion = 1
[plugin]
name = "@example/product-install"
version = "1.2.3"
apiVersion = 1
[runtime]
kind = "node"
entry = "plugin.mjs"
[[capabilities.required]]
realm = "storage"
scope = "read"
reason = "Read authored project content"
[[capabilities.optional]]
realm = "network"
scope = "*"
reason = "Fetch remote enrichments"
[[capabilities.optional]]
realm = "filesystem"
scope = "read"
detail = "$storageRoot"
reason = "Read the configured storage root"
[[capabilities.optional]]
realm = "filesystem"
scope = "write"
detail = "$cacheDir"
reason = "Write the configured cache"
[[contributes.stages]]
id = "product-install"
consumes = "ContentSource"
produces = "ContentNode"
`;

afterEach(async () => {
  await Promise.all(roots.splice(0).map(root => rm(root, { recursive: true, force: true })));
});

async function fixture(): Promise<{ project: string; packagePath: string; trustStorePath: string }> {
  const root = await mkdtemp(join(tmpdir(), "forme-cli-install-"));
  roots.push(root);
  const project = join(root, "project");
  const packagePath = join(root, "package");
  await mkdir(project, { mode: 0o700 });
  await mkdir(packagePath, { mode: 0o700 });
  await writeFile(join(packagePath, "plugin.toml"), MANIFEST);
  await writeFile(join(packagePath, "plugin.mjs"), "export const plugin = true;\n");
  return { project, packagePath, trustStorePath: join(root, "missing-trust.toml") };
}

function capabilityPath(path: string): string {
  return encodeURIComponent(path).replaceAll("%2F", "/");
}

describe("executePluginInstall", () => {
  it("reviews every capability and atomically persists exactly the accepted grants", async () => {
    const setup = await fixture();
    const reviews: CapabilityReview[] = [];
    const result = await executePluginInstall({
      packagePath: setup.packagePath,
      projectRoot: setup.project,
      storageRoot: join(setup.project, "content"),
      cacheDir: join(setup.project, ".cache"),
      trustStorePath: setup.trustStorePath,
      reviewCapability: async review => {
        reviews.push(review);
        return review.required;
      },
      now: () => new Date("2026-10-01T12:00:00Z"),
      verifyWindowsAcl: async () => true,
    });

    expect(reviews).toEqual([
      {
        pluginName: "@example/product-install",
        pluginVersion: "1.2.3",
        trustTier: "unverified-third-party",
        capability: "storage:read",
        required: true,
        reason: "Read authored project content",
        sensitive: false,
      },
      {
        pluginName: "@example/product-install",
        pluginVersion: "1.2.3",
        trustTier: "unverified-third-party",
        capability: "network:*",
        required: false,
        reason: "Fetch remote enrichments",
        sensitive: true,
      },
      {
        pluginName: "@example/product-install",
        pluginVersion: "1.2.3",
        trustTier: "unverified-third-party",
        capability: `filesystem:read:${capabilityPath(join(setup.project, "content"))}`,
        required: false,
        reason: "Read the configured storage root",
        sensitive: false,
      },
      {
        pluginName: "@example/product-install",
        pluginVersion: "1.2.3",
        trustTier: "unverified-third-party",
        capability: `filesystem:write:${capabilityPath(join(setup.project, ".cache"))}`,
        required: false,
        reason: "Write the configured cache",
        sensitive: false,
      },
    ]);
    expect(result).toMatchObject({
      status: "installed",
      pluginName: "@example/product-install",
      pluginVersion: "1.2.3",
      trustTier: "unverified-third-party",
      grantedCapabilities: ["storage:read"],
    });
    await expect(readFile(join(result.destinationPath, "grants.toml"), "utf8"))
      .resolves.toContain('capability = "storage:read"');
    await expect(readFile(join(result.destinationPath, "grants.toml"), "utf8"))
      .resolves.not.toContain("network:*");
  });

  it("refuses installation when a required capability is denied", async () => {
    const setup = await fixture();
    await expect(executePluginInstall({
      packagePath: setup.packagePath,
      projectRoot: setup.project,
      storageRoot: setup.project,
      cacheDir: join(setup.project, ".cache"),
      trustStorePath: setup.trustStorePath,
      reviewCapability: async () => false,
      verifyWindowsAcl: async () => true,
    })).rejects.toThrow(/required capability.*denied/i);
    await expect(readFile(join(setup.project, "forme-plugins", "plugin-QGV4YW1wbGUvcHJvZHVjdC1pbnN0YWxs")))
      .rejects.toThrow();
  });

  it("re-reviews and replaces grants instead of trusting package-supplied authority", async () => {
    const setup = await fixture();
    await writeFile(join(setup.packagePath, "grants.toml"), "attacker controlled");
    let prompted = false;
    await expect(executePluginInstall({
      packagePath: setup.packagePath,
      projectRoot: setup.project,
      storageRoot: setup.project,
      cacheDir: join(setup.project, ".cache"),
      trustStorePath: setup.trustStorePath,
      reviewCapability: async () => { prompted = true; return true; },
      verifyWindowsAcl: async () => true,
    })).rejects.toThrow(/grants\.toml/i);
    expect(prompted).toBe(false);
  });

  it("completes package preflight before presenting authority prompts", async () => {
    const setup = await fixture();
    await rm(join(setup.packagePath, "plugin.mjs"));
    let prompted = false;
    await expect(executePluginInstall({
      packagePath: setup.packagePath,
      projectRoot: setup.project,
      storageRoot: setup.project,
      cacheDir: join(setup.project, ".cache"),
      trustStorePath: setup.trustStorePath,
      reviewCapability: async () => { prompted = true; return true; },
      verifyWindowsAcl: async () => true,
    })).rejects.toThrow(/missing runtime entry/i);
    expect(prompted).toBe(false);
  });

  it.runIf(process.platform !== "win32")("rejects a shared-writable project root before prompting", async () => {
    const setup = await fixture();
    await chmod(setup.project, 0o770);
    let prompted = false;
    await expect(executePluginInstall({
      packagePath: setup.packagePath,
      projectRoot: setup.project,
      storageRoot: setup.project,
      cacheDir: join(setup.project, ".cache"),
      trustStorePath: setup.trustStorePath,
      reviewCapability: async () => { prompted = true; return true; },
    })).rejects.toThrow(/project root.*not writable/i);
    expect(prompted).toBe(false);
  });
});

describe("terminal rendering", () => {
  it("escapes control, ANSI, and bidirectional override characters", () => {
    expect(escapeTerminalText("safe\n\u001b[2J\u061c\u200f\u2029\u202efake")).toBe(
      "safe\\u{000a}\\u{001b}[2J\\u{061c}\\u{200f}\\u{2029}\\u{202e}fake",
    );
  });
});

describe("snapshotPluginDirectory", () => {
  it("copies a canonical sorted regular-file snapshot", async () => {
    const setup = await fixture();
    await mkdir(join(setup.packagePath, "schemas"));
    await writeFile(join(setup.packagePath, "schemas", "config.json"), "{}");
    const files = await snapshotPluginDirectory(setup.packagePath);
    expect(files.map(file => file.path)).toEqual([
      "plugin.mjs",
      "plugin.toml",
      "schemas/config.json",
    ]);
  });

  it("rejects linked and non-regular source entries", async () => {
    const setup = await fixture();
    await symlink("plugin.mjs", join(setup.packagePath, "linked.mjs"));
    await expect(snapshotPluginDirectory(setup.packagePath)).rejects.toThrow(/symbolic link/i);
  });

  it("bounds each source-file read before allocating package bytes", async () => {
    const setup = await fixture();
    await writeFile(join(setup.packagePath, "oversized.bin"), Buffer.alloc(16 * 1024 * 1024 + 1));
    await expect(snapshotPluginDirectory(setup.packagePath)).rejects.toThrow(/per-file byte limit/i);
  });
});
