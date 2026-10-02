import { link, mkdir, mkdtemp, readFile, rm, symlink, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { afterEach, describe, expect, it } from "vitest";
import { createProductCapabilityApis, createProjectStorage } from "../src/capability-apis.js";

const roots: string[] = [];
afterEach(async () => {
  delete process.env.FORME_CAPABILITY_TEST;
  await Promise.all(roots.splice(0).map(root => rm(root, { recursive: true, force: true })));
});

async function temporaryRoot(prefix: string): Promise<string> {
  const root = await mkdtemp(join(tmpdir(), prefix));
  roots.push(root);
  return root;
}

describe("product capability APIs", () => {
  it("provides bounded contained project storage", async () => {
    const root = await temporaryRoot("forme-storage-");
    const storage = createProjectStorage(root);
    await storage.write("posts/a.md", new TextEncoder().encode("hello"));
    expect(new TextDecoder().decode(await storage.readBounded("posts/a.md", 5))).toBe("hello");
    await expect(storage.readBounded("posts/a.md", 4)).rejects.toThrow(/limit/);
    expect(await storage.exists("posts/a.md")).toBe(true);
    expect(await storage.stat("posts/a.md")).toMatchObject({ size: 5, type: "file" });
    const entries = [];
    for await (const entry of storage.list("posts")) entries.push(entry);
    expect(entries).toEqual([{ path: "posts/a.md", type: "file" }]);
    await storage.remove("posts/a.md");
    expect(await storage.exists("posts/a.md")).toBe(false);
  });

  it("rejects traversal and linked storage paths", async () => {
    const root = await temporaryRoot("forme-storage-root-");
    const outside = await temporaryRoot("forme-storage-outside-");
    await writeFile(join(outside, "secret"), "secret");
    await mkdir(join(root, "posts"));
    await symlink(outside, join(root, "posts", "linked"), process.platform === "win32" ? "junction" : "dir");
    const storage = createProjectStorage(root);
    await expect(storage.read("../secret")).rejects.toThrow(/unsafe|relative/);
    await expect(storage.read("posts/linked/secret")).rejects.toThrow(/escapes|symbolic/);
    await expect(storage.write("posts/linked/out", new Uint8Array())).rejects.toThrow(/real directory/);
  });

  it("does not truncate hard-linked write targets before rejecting them", async () => {
    const root = await temporaryRoot("forme-storage-hardlink-");
    const outside = await temporaryRoot("forme-storage-hardlink-outside-");
    const source = join(outside, "source");
    await writeFile(source, "preserve me");
    await link(source, join(root, "target"));

    const storage = createProjectStorage(root);
    await expect(storage.write("target", new TextEncoder().encode("replace"))).rejects.toThrow(/regular file/);
    expect(await readFile(source, "utf8")).toBe("preserve me");
  });

  it("hides host-owned plugin state from storage and broad filesystem grants", async () => {
    const root = await temporaryRoot("forme-storage-reserved-");
    const pluginRoot = join(root, "forme-plugins");
    const plugin = join(pluginRoot, "example");
    await mkdir(plugin, { recursive: true });
    await writeFile(join(plugin, "grants.toml"), "grant");
    const apis = createProductCapabilityApis(root, [pluginRoot]);
    const storage = apis.storage!;

    await expect(storage.read("forme-plugins/example/grants.toml")).rejects.toThrow(/host-reserved/);
    await expect(storage.readBounded("forme-plugins/example/grants.toml", 32)).rejects.toThrow(/host-reserved/);
    await expect(storage.exists("forme-plugins/example/grants.toml")).rejects.toThrow(/host-reserved/);
    await expect(storage.write("forme-plugins/example/grants.toml", new Uint8Array())).rejects.toThrow(/host-reserved/);
    await expect(storage.remove("forme-plugins/example/grants.toml")).rejects.toThrow(/host-reserved/);
    await expect(storage.stat("forme-plugins")).rejects.toThrow(/host-reserved/);
    await expect(async () => {
      for await (const _entry of storage.list("forme-plugins")) { /* exhaust */ }
    }).rejects.toThrow(/host-reserved/);
    expect(() => storage.watch("forme-plugins")).toThrow(/host-reserved/);

    const visible = [];
    for await (const entry of storage.list(".")) visible.push(entry.path);
    expect(visible).not.toContain("forme-plugins");

    const grantsPath = join(plugin, "grants.toml");
    await expect(apis.filesystem!.readAbsolute(grantsPath)).rejects.toThrow(/host-reserved/);
    await expect(apis.filesystem!.readAbsoluteBounded(grantsPath, 32)).rejects.toThrow(/host-reserved/);
    await expect(apis.filesystem!.writeAbsolute(grantsPath, new Uint8Array())).rejects.toThrow(/host-reserved/);
    expect(await readFile(grantsPath, "utf8")).toBe("grant");

    const nested = createProjectStorage(pluginRoot, [pluginRoot]);
    await expect(nested.stat(".")).rejects.toThrow(/host-reserved/);
    await expect(nested.write("replacement", new Uint8Array())).rejects.toThrow(/host-reserved/);
  });

  it("provides product-mediated environment, network, and absolute-file backends", async () => {
    const root = await temporaryRoot("forme-capabilities-");
    const file = join(root, "absolute.txt");
    const apis = createProductCapabilityApis(root);

    process.env.FORME_CAPABILITY_TEST = "visible";
    expect(await apis.env!.get("FORME_CAPABILITY_TEST")).toBe("visible");
    expect(await apis.env!.getOrThrow("FORME_CAPABILITY_TEST")).toBe("visible");
    await expect(apis.env!.getOrThrow("FORME_CAPABILITY_MISSING")).rejects.toThrow(/not set/);

    const response = await apis.network!.fetch("data:text/plain,hello");
    expect(await response.text()).toBe("hello");

    await apis.filesystem!.writeAbsolute(file, new TextEncoder().encode("absolute"));
    expect(new TextDecoder().decode(await apis.filesystem!.readAbsolute(file))).toBe("absolute");
    expect(new TextDecoder().decode(await apis.filesystem!.readAbsoluteBounded(file, 8))).toBe("absolute");
    await expect(apis.filesystem!.readAbsoluteBounded(file, 7)).rejects.toThrow(/limit/);
    expect(await apis.filesystem!.homeDir()).not.toBe("");
    expect(await apis.filesystem!.tempDir()).not.toBe("");
  });
});
