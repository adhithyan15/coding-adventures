import { mkdtemp, mkdir, realpath, symlink, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { fileURLToPath } from "node:url";
import { describe, expect, it } from "vitest";
import { descriptorForKindReference, discoverPlugins, PluginHostError, resolveContainedFile } from "../src/index.js";
import { __testing, readBoundedRegularFile } from "../src/discovery.js";

function manifest(name: string, version = "1.0.0"): string {
  return `manifestVersion = 1
[plugin]
name = "${name}"
version = "${version}"
apiVersion = 1
[runtime]
kind = "node"
entry = "./entry.mjs"
[[contributes.stages]]
id = "echo"
consumes = "ContentNode"
produces = "ContentNode"
`;
}

function binaryManifest(name: string, entry?: string): string {
  const platform = process.platform === "win32" ? "windows" : process.platform;
  const architecture = process.arch === "x64" ? "x86_64" : process.arch === "arm64" ? "aarch64" : process.arch;
  const otherPlatform = platform === "windows" ? "linux" : "windows";
  return `manifestVersion = 1
[plugin]
name = "${name}"
version = "1.0.0"
apiVersion = 1
[runtime]
kind = "binary"
[runtime.platforms]
${entry ? `${platform}-${architecture} = "${entry}"` : `${otherPlatform}-${architecture} = "./runner"`}
[[contributes.stages]]
id = "echo"
consumes = "ContentNode"
produces = "ContentNode"
`;
}

async function plugin(root: string, dir: string, name: string, version?: string): Promise<string> {
  const path = join(root, dir);
  await mkdir(path, { recursive: true });
  await writeFile(join(path, "plugin.toml"), manifest(name, version));
  await writeFile(join(path, "entry.mjs"), "// fixture\n");
  return path;
}

describe("plugin discovery", () => {
  it("uses root precedence, sorts entries, and does not execute entry code", async () => {
    const base = await mkdtemp(join(tmpdir(), "forme-plugin-discovery-"));
    const project = join(base, "project");
    const user = join(base, "user");
    await plugin(project, "z", "@example/echo", "2.0.0");
    await plugin(project, "a", "@example/alpha");
    await plugin(user, "echo", "@example/echo", "1.0.0");
    const found = await discoverPlugins([project, user]);
    expect([...found.keys()]).toEqual(["@example/alpha", "@example/echo"]);
    expect(found.get("@example/echo")?.manifest.plugin.version).toBe("2.0.0");
    expect(found.get("@example/echo")?.manifestHash).toMatch(/^blake2b:[0-9a-f]{64}$/);
  });

  it("deduplicates shared schema snapshots before enforcing the aggregate budget", async () => {
    const root = await mkdtemp(join(tmpdir(), "forme-plugin-shared-schema-"));
    const path = join(root, "shared");
    await mkdir(path);
    await writeFile(join(path, "entry.mjs"), "// fixture\n");
    await writeFile(join(path, "schema.json"), '{"type":"object"}');
    const stages = Array.from({ length: 200 }, (_, index) => [
      "[[contributes.stages]]",
      `id = "stage-${index}"`,
      'consumes = "ContentNode"',
      'produces = "ContentNode"',
      `configSchema = "${index === 199 ? "schema.json" : "./schema.json"}"`,
      "",
    ].join("\n")).join("");
    await writeFile(join(path, "plugin.toml"), [
      "manifestVersion = 1",
      "[plugin]",
      'name = "@example/shared-schema"',
      'version = "1.0.0"',
      "apiVersion = 1",
      "[runtime]",
      'kind = "node"',
      'entry = "./entry.mjs"',
      stages,
    ].join("\n"));
    const found = await discoverPlugins([root]);
    const schemas = found.get("@example/shared-schema")!.configSchemas;
    expect(Object.keys(schemas)).toHaveLength(200);
    expect(schemas["stage-0"]!.bytes).toBe(schemas["stage-199"]!.bytes);
    expect(schemas["stage-0"]!.relativePath).toBe("./schema.json");
    expect(schemas["stage-199"]!.relativePath).toBe("schema.json");
  });

  it("rejects external schemas on signed plugins until package signatures bind auxiliary files", async () => {
    const root = await mkdtemp(join(tmpdir(), "forme-plugin-signed-schema-"));
    const path = await plugin(root, "signed", "@example/signed-schema");
    await writeFile(join(path, "schema.json"), '{"type":"object"}');
    await writeFile(join(path, "plugin.toml"), `${manifest("@example/signed-schema").replace(
      'produces = "ContentNode"',
      'produces = "ContentNode"\nconfigSchema = "./schema.json"',
    )}[signature]
algorithm = "ed25519"
publicKey = "AA=="
signature = "AA=="
signedAt = "2026-05-16T00:00:00Z"
`);
    await expect(discoverPlugins([root])).rejects.toThrow(/signed plugins cannot reference external config schemas/);
  });

  it("rejects duplicate names within one root and compares names without locale state", async () => {
    const root = await mkdtemp(join(tmpdir(), "forme-plugin-duplicate-"));
    await plugin(root, "a", "@example/duplicate");
    await plugin(root, "b", "@example/duplicate");
    await expect(discoverPlugins([root])).rejects.toThrow(/duplicate plugin name/);
    expect(__testing.compareCodePoints("a", "a")).toBe(0);
    expect(__testing.compareCodePoints("a", "b")).toBe(-1);
    expect(__testing.compareCodePoints("b", "a")).toBe(1);
  });

  it("rejects a runtime entry that escapes through a symlink", async () => {
    const base = await mkdtemp(join(tmpdir(), "forme-plugin-escape-"));
    const root = join(base, "root");
    const path = await plugin(root, "evil", "@example/evil");
    const outside = join(base, "outside.mjs");
    await writeFile(outside, "// outside\n");
    await writeFile(join(path, "plugin.toml"), manifest("@example/evil").replace("./entry.mjs", "./link.mjs"));
    await symlink(outside, join(path, "link.mjs"));
    await expect(discoverPlugins([root])).rejects.toBeInstanceOf(PluginHostError);
    await expect(discoverPlugins([root])).rejects.toThrow(/ENTRY_OUTSIDE_PLUGIN/);
  });

  it("rechecks containment against the opened file descriptor", async () => {
    const base = await mkdtemp(join(tmpdir(), "forme-plugin-open-containment-"));
    const root = join(base, "root");
    const outside = join(base, "outside");
    await mkdir(root);
    await mkdir(outside);
    await writeFile(join(outside, "entry.mjs"), "outside");
    await symlink(outside, join(root, "swapped"));
    await expect(readBoundedRegularFile(
      join(root, "swapped", "entry.mjs"), 1024, "runtime entry", root,
    )).rejects.toThrow(/ENTRY_OUTSIDE_PLUGIN/);
  });

  it("ignores absent roots and non-directories", async () => {
    const base = await mkdtemp(join(tmpdir(), "forme-plugin-missing-"));
    await writeFile(join(base, "README"), "not a plugin");
    await expect(discoverPlugins([join(base, "absent"), base])).resolves.toEqual(new Map());
    await expect(discoverPlugins([join(base, "README")])).rejects.toBeInstanceOf(Error);
  });

  it("honours cancellation before discovery and bounded reads", async () => {
    const controller = new AbortController();
    controller.abort(new Error("stop discovery"));
    await expect(discoverPlugins(["/absent"], controller.signal))
      .rejects.toThrow("stop discovery");
    await expect(readBoundedRegularFile(
      fileURLToPath(import.meta.url), 1024 * 1024, "test file", undefined, controller.signal,
    )).rejects.toThrow("stop discovery");
  });

  it("bounds roots and examined directories independently of retained plugins", async () => {
    await expect(discoverPlugins(Array.from({ length: 65 }, (_, index) => `/absent/${index}`)))
      .rejects.toThrow(/64 root limit/);
    const root = await mkdtemp(join(tmpdir(), "forme-plugin-candidates-"));
    await Promise.all(Array.from({ length: 1_025 }, (_, index) => mkdir(join(root, String(index)))));
    await expect(discoverPlugins([root])).rejects.toThrow(/1024 candidate limit/);
  }, 30_000);

  it.each([
    ["invalid manifest", "not = [toml"],
    ["missing entry", manifest("@example/bad").replace("./entry.mjs", "./missing.mjs")],
  ])("rejects %s", async (_label, text) => {
    const base = await mkdtemp(join(tmpdir(), "forme-plugin-invalid-"));
    const path = join(base, "bad");
    await mkdir(path);
    await writeFile(join(path, "plugin.toml"), text);
    if (text.includes("missing.mjs")) await writeFile(join(path, "entry.mjs"), "// unused");
    await expect(discoverPlugins([base])).rejects.toBeInstanceOf(Error);
  });

  it("rejects absolute contained-file references", async () => {
    await expect(resolveContainedFile("/tmp", "/etc/passwd", "entry"))
      .rejects.toThrow(/ENTRY_OUTSIDE_PLUGIN/);
  });

  it("resolves the current binary platform and rejects an absent mapping", async () => {
    const base = await mkdtemp(join(tmpdir(), "forme-plugin-binary-"));
    const good = join(base, "good");
    await mkdir(good);
    await writeFile(join(good, "plugin.toml"), binaryManifest("@example/binary", "./runner"));
    await writeFile(join(good, "runner"), "binary");
    const found = await discoverPlugins([base]);
    expect(found.get("@example/binary")?.entryPath).toBe(await realpath(join(good, "runner")));

    const badRoot = await mkdtemp(join(tmpdir(), "forme-plugin-binary-missing-"));
    const bad = join(badRoot, "bad");
    await mkdir(bad);
    await writeFile(join(bad, "plugin.toml"), binaryManifest("@example/missing-binary"));
    await expect(discoverPlugins([badRoot])).rejects.toThrow(/no entry/);
  });

  it("rejects non-file manifests and runtime entries", async () => {
    const manifestRoot = await mkdtemp(join(tmpdir(), "forme-plugin-manifest-dir-"));
    const manifestDir = join(manifestRoot, "bad");
    await mkdir(join(manifestDir, "plugin.toml"), { recursive: true });
    await expect(discoverPlugins([manifestRoot])).rejects.toThrow(/bounded regular file/);

    const entryRoot = await mkdtemp(join(tmpdir(), "forme-plugin-entry-dir-"));
    const entryDir = join(entryRoot, "bad");
    await mkdir(join(entryDir, "entry.mjs"), { recursive: true });
    await writeFile(join(entryDir, "plugin.toml"), manifest("@example/entry-dir"));
    await expect(discoverPlugins([entryRoot])).rejects.toThrow(/runtime entry must be/);
  });

  it("resolves built-in, contributed, and stream kind descriptors", async () => {
    const base = await mkdtemp(join(tmpdir(), "forme-plugin-kinds-"));
    const path = await plugin(base, "kinds", "@example/kinds");
    await writeFile(join(path, "plugin.toml"), `${manifest("@example/kinds")}
[[contributes.kinds]]
name = "ext:example-kind"
version = "2.0"
schema = "./schema.json"
`);
    await writeFile(join(path, "schema.json"), "{}");
    const found = await discoverPlugins([base]);
    const loaded = found.get("@example/kinds")!;
    expect(descriptorForKindReference("ContentNode", loaded.manifest).name).toBe("ContentNode");
    expect(descriptorForKindReference("ext:example-kind", loaded.manifest)).toEqual({ name: "ext:example-kind", version: "2.0" });
    expect(descriptorForKindReference("Stream<ext:example-kind>", loaded.manifest).name).toBe("Stream");
    expect(() => descriptorForKindReference("Missing", loaded.manifest)).toThrow(/unknown kind/);
  });

  it("normalizes all supported platform names and errno shapes", () => {
    expect(__testing.platformName("win32")).toBe("windows");
    expect(__testing.platformName("darwin")).toBe("darwin");
    expect(__testing.architectureName("x64")).toBe("x86_64");
    expect(__testing.architectureName("arm64")).toBe("aarch64");
    expect(__testing.architectureName("riscv64")).toBe("riscv64");
    expect(__testing.isErrno({ code: "ENOENT" }, "ENOENT")).toBe(true);
    expect(__testing.isErrno(null, "ENOENT")).toBe(false);
    expect(__testing.isErrno("ENOENT", "ENOENT")).toBe(false);
  });
});
