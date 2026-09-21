import { createHash } from "node:crypto";
import {
  link,
  lstat,
  mkdir,
  mkdtemp,
  readFile,
  readdir,
  rename,
  rm,
  stat,
  symlink,
  writeFile,
} from "node:fs/promises";
import { tmpdir } from "node:os";
import { basename, dirname, join, parse } from "node:path";
import { afterEach, describe, expect, it, vi } from "vitest";
import type { ContentStore } from "@coding-adventures/forme-deploy-runner-core";
import {
  FilesystemPublishError,
  prepareFilesystemPublication,
  publishFilesystemSite,
} from "../src/index.js";

const fsFaults = vi.hoisted(() => ({
  rmNeedle: "",
  rmRemaining: 0,
  renameRules: [] as Array<{ readonly sourceNeedle: string; remaining: number }>,
  afterRealpath: undefined as undefined | ((path: string) => Promise<boolean>),
  beforeLstat: undefined as undefined | ((path: string) => Promise<boolean>),
  afterLstat: undefined as undefined | ((path: string) => Promise<boolean>),
}));

vi.mock("node:fs/promises", async importOriginal => {
  const actual = await importOriginal<typeof import("node:fs/promises")>();
  return {
    ...actual,
    rm: async (...args: Parameters<typeof actual.rm>) => {
      if (fsFaults.rmRemaining > 0 && String(args[0]).includes(fsFaults.rmNeedle)) {
        fsFaults.rmRemaining -= 1;
        throw Object.assign(new Error("injected rm failure"), { code: "EBUSY" });
      }
      return actual.rm(...args);
    },
    rename: async (...args: Parameters<typeof actual.rename>) => {
      const rule = fsFaults.renameRules.find(candidate =>
        candidate.remaining > 0 && String(args[0]).includes(candidate.sourceNeedle));
      if (rule !== undefined) {
        rule.remaining -= 1;
        throw Object.assign(new Error("injected rename failure"), { code: "EBUSY" });
      }
      return actual.rename(...args);
    },
    realpath: async (...args: Parameters<typeof actual.realpath>) => {
      const result = await actual.realpath(...args);
      const callback = fsFaults.afterRealpath;
      if (callback !== undefined && await callback(String(args[0]))) {
        fsFaults.afterRealpath = undefined;
      }
      return result;
    },
    lstat: async (...args: Parameters<typeof actual.lstat>) => {
      const path = String(args[0]);
      const before = fsFaults.beforeLstat;
      if (before !== undefined && await before(path)) fsFaults.beforeLstat = undefined;
      const result = await actual.lstat(...args);
      const after = fsFaults.afterLstat;
      if (after !== undefined && await after(path)) fsFaults.afterLstat = undefined;
      return result;
    },
  };
});

const encoder = new TextEncoder();
const roots: string[] = [];

afterEach(async () => {
  fsFaults.rmNeedle = "";
  fsFaults.rmRemaining = 0;
  fsFaults.renameRules = [];
  fsFaults.afterRealpath = undefined;
  fsFaults.beforeLstat = undefined;
  fsFaults.afterLstat = undefined;
  await Promise.all(roots.splice(0).map(root => rm(root, { recursive: true, force: true })));
});

async function temporarySite(): Promise<string> {
  const parent = await mkdtemp(join(tmpdir(), "forme-fs-adapter-"));
  roots.push(parent);
  return join(parent, "site");
}

const bytes = (value: string): Uint8Array => encoder.encode(value);
const digest = (value: Uint8Array): string => createHash("sha256").update(value).digest("base64");

function manifest(files: Readonly<Record<string, string>>): unknown {
  const entries = Object.fromEntries(Object.entries(files).map(([outputPath, body]) => {
    const content = bytes(body);
    return [outputPath, {
      outputPath,
      contentType: outputPath.endsWith(".html") ? "text/html; charset=utf-8" : "text/plain; charset=utf-8",
      sizeBytes: content.byteLength,
      sha256: digest(content),
      source: "extra",
    }];
  }));
  return {
    version: 1,
    fileCount: Object.keys(entries).length,
    totalSizeBytes: Object.values(files).reduce((total, body) => total + bytes(body).byteLength, 0),
    files: entries,
  };
}

function store(files: Readonly<Record<string, string>>): ContentStore {
  const values = new Map(Object.values(files).map(body => {
    const content = bytes(body);
    return [digest(content), content] as const;
  }));
  return {
    has: async sha256 => values.has(sha256),
    get: async sha256 => {
      const value = values.get(sha256);
      if (value === undefined) throw new Error("missing");
      return value;
    },
    hashes: async function* () { yield* values.keys(); },
  };
}

async function artifacts(root: string): Promise<string[]> {
  const prefix = `.${basename(root)}.forme-`;
  return (await readdir(dirname(root))).filter(entry => entry.startsWith(prefix));
}

describe("publishFilesystemSite", () => {
  it("publishes the complete nested manifest and removes transaction residue", async () => {
    const root = await temporarySite();
    const files = { "index.html": "home", "assets/app.css": "body{}" };

    const result = await publishFilesystemSite({ root, manifest: manifest(files), contentStore: store(files) });

    expect(result).toEqual({ status: "published", fileCount: 2, totalSizeBytes: 10 });
    expect(await readFile(join(root, "index.html"), "utf8")).toBe("home");
    expect(await readFile(join(root, "assets/app.css"), "utf8")).toBe("body{}");
    expect(await artifacts(root)).toEqual([]);
  });

  it("prunes stale files by replacing only the complete owned tree", async () => {
    const root = await temporarySite();
    await writeFile(join(dirname(root), "keep-outside.txt"), "outside");
    const old = { "old.txt": "old", "stale.txt": "stale" };
    await publishFilesystemSite({ root, manifest: manifest(old), contentStore: store(old) });

    const next = { "new.txt": "new" };
    await publishFilesystemSite({ root, manifest: manifest(next), contentStore: store(next) });

    expect(await readdir(root)).toEqual(["new.txt"]);
    expect(await readFile(join(dirname(root), "keep-outside.txt"), "utf8")).toBe("outside");
  });

  it("detects an exact tree without changing its inode or timestamps", async () => {
    const root = await temporarySite();
    const files = { "index.html": "same" };
    await publishFilesystemSite({ root, manifest: manifest(files), contentStore: store(files) });
    const before = await stat(join(root, "index.html"), { bigint: true });

    const result = await publishFilesystemSite({ root, manifest: manifest(files), contentStore: store(files) });
    const after = await stat(join(root, "index.html"), { bigint: true });

    expect(result.status).toBe("unchanged");
    expect(after.ino).toBe(before.ino);
    expect(after.mtimeNs).toBe(before.mtimeNs);
    expect(await artifacts(root)).toEqual([]);
  });

  it("publishes changed bytes even when the old file has the same size", async () => {
    const root = await temporarySite();
    const old = { "index.html": "old!" };
    await publishFilesystemSite({ root, manifest: manifest(old), contentStore: store(old) });

    const next = { "index.html": "new!" };
    const result = await publishFilesystemSite({ root, manifest: manifest(next), contentStore: store(next) });

    expect(result.status).toBe("published");
    expect(await readFile(join(root, "index.html"), "utf8")).toBe("new!");
  });

  it("replaces a tree that has stale empty directories", async () => {
    const root = await temporarySite();
    const files = { "index.html": "same" };
    await publishFilesystemSite({ root, manifest: manifest(files), contentStore: store(files) });
    await mkdir(join(root, "stale", "empty"), { recursive: true });

    const result = await publishFilesystemSite({ root, manifest: manifest(files), contentStore: store(files) });

    expect(result.status).toBe("published");
    expect(await readdir(root)).toEqual(["index.html"]);
  });

  it("leaves the old root untouched when content preparation fails", async () => {
    const root = await temporarySite();
    const old = { "index.html": "old" };
    await publishFilesystemSite({ root, manifest: manifest(old), contentStore: store(old) });

    await expect(publishFilesystemSite({
      root,
      manifest: manifest({ "index.html": "new" }),
      contentStore: store({}),
    })).rejects.toMatchObject({ code: "CONTENT_MISSING" });

    expect(await readFile(join(root, "index.html"), "utf8")).toBe("old");
    expect(await artifacts(root)).toEqual([]);
  });

  it("cancels during staging, removes the stage, and retains the old root", async () => {
    const root = await temporarySite();
    const old = { "index.html": "old" };
    await publishFilesystemSite({ root, manifest: manifest(old), contentStore: store(old) });
    const next = { "a.txt": "a", "b.txt": "b" };
    const values = store(next);
    const controller = new AbortController();
    let reads = 0;
    const cancelling: ContentStore = {
      ...values,
      get: async (sha256, signal) => {
        const value = await values.get(sha256, signal);
        reads += 1;
        if (reads === 1) controller.abort("stop");
        return value;
      },
    };

    await expect(publishFilesystemSite({
      root,
      manifest: manifest(next),
      contentStore: cancelling,
      signal: controller.signal,
    })).rejects.toMatchObject({ code: "CONTENT_ABORTED" });

    expect(await readFile(join(root, "index.html"), "utf8")).toBe("old");
    expect(await artifacts(root)).toEqual([]);
  });

  it("rejects an already aborted publication without leaving residue", async () => {
    const root = await temporarySite();
    const controller = new AbortController();
    controller.abort("stop");
    const files = { "index.html": "new" };

    await expect(publishFilesystemSite({
      root,
      manifest: manifest(files),
      contentStore: store(files),
      signal: controller.signal,
    })).rejects.toMatchObject<Partial<FilesystemPublishError>>({ code: "ABORTED" });

    await expect(lstat(root)).rejects.toMatchObject({ code: "ENOENT" });
    expect(await artifacts(root)).toEqual([]);
  });

  it("rejects an already aborted exact empty publication", async () => {
    const root = await temporarySite();
    await mkdir(root);
    const controller = new AbortController();
    controller.abort("stop");

    await expect(publishFilesystemSite({
      root,
      manifest: manifest({}),
      contentStore: store({}),
      signal: controller.signal,
    })).rejects.toMatchObject<Partial<FilesystemPublishError>>({ code: "ABORTED" });

    expect(await readdir(root)).toEqual([]);
    expect(await artifacts(root)).toEqual([]);
  });

  it("does not return a prepared transaction after cancellation during final stage verification", async () => {
    const root = await temporarySite();
    const controller = new AbortController();
    const files = { "index.html": "new" };
    let stageRootStats = 0;
    fsFaults.afterLstat = async path => {
      if (!basename(path).includes("forme-stage-")) return false;
      stageRootStats += 1;
      if (stageRootStats < 6) return false;
      controller.abort("stop after final staged read");
      return true;
    };

    await expect(prepareFilesystemPublication({
      root,
      manifest: manifest(files),
      contentStore: store(files),
      signal: controller.signal,
    })).rejects.toMatchObject<Partial<FilesystemPublishError>>({ code: "ABORTED" });

    expect(stageRootStats).toBeGreaterThanOrEqual(6);
    expect(await artifacts(root)).toEqual([]);
  });

  it("preserves a content failure and still releases the lock when stage cleanup fails", async () => {
    const root = await temporarySite();
    fsFaults.rmNeedle = ".forme-stage-";
    fsFaults.rmRemaining = 1;

    const failure = await publishFilesystemSite({
      root,
      manifest: manifest({ "index.html": "missing" }),
      contentStore: store({}),
    }).catch(error => error as Error & { code?: string; secondaryErrors?: unknown[] });

    expect(failure.code).toBe("CONTENT_MISSING");
    expect(failure.secondaryErrors).toHaveLength(1);
    expect((await artifacts(root)).some(path => path.includes("forme-stage-"))).toBe(true);
    expect((await artifacts(root)).some(path => path.includes("forme-lock"))).toBe(false);
  });

  it("preserves a commit failure while rollback diagnostics remain secondary", async () => {
    const root = await temporarySite();
    const old = { "index.html": "old" };
    await publishFilesystemSite({ root, manifest: manifest(old), contentStore: store(old) });
    fsFaults.renameRules = [
      { sourceNeedle: ".forme-stage-", remaining: 1 },
      { sourceNeedle: ".forme-backup-", remaining: 1 },
    ];
    const next = { "index.html": "new" };

    const failure = await publishFilesystemSite({
      root,
      manifest: manifest(next),
      contentStore: store(next),
    }).catch(error => error as Error & { code?: string; secondaryErrors?: unknown[] });

    expect(failure.code).toBe("COMMIT_FAILED");
    expect(failure.secondaryErrors?.some(error =>
      error instanceof FilesystemPublishError && error.code === "ROLLBACK_FAILED")).toBe(true);
    expect(await readFile(join(root, "index.html"), "utf8")).toBe("old");
    expect(await artifacts(root)).toEqual([]);
  });
});

describe("filesystem containment", () => {
  it("rejects empty and volume-root publication paths", async () => {
    await expect(publishFilesystemSite({ root: "", manifest: manifest({}), contentStore: store({}) }))
      .rejects.toMatchObject<Partial<FilesystemPublishError>>({ code: "ROOT_UNSAFE" });
    await expect(publishFilesystemSite({ root: parse(process.cwd()).root, manifest: manifest({}), contentStore: store({}) }))
      .rejects.toMatchObject<Partial<FilesystemPublishError>>({ code: "ROOT_UNSAFE" });
  });

  it("validates the manifest before it inspects an unsafe root", async () => {
    const root = await temporarySite();
    const outside = await mkdtemp(join(tmpdir(), "forme-fs-invalid-"));
    roots.push(outside);
    await symlink(outside, root, process.platform === "win32" ? "junction" : "dir");

    await expect(publishFilesystemSite({ root, manifest: { version: 999 }, contentStore: store({}) }))
      .rejects.toThrow(/manifest.version/);
  });

  it("rejects a regular file configured as the publication root", async () => {
    const root = await temporarySite();
    await writeFile(root, "not a directory");

    await expect(publishFilesystemSite({
      root,
      manifest: manifest({ "index.html": "new" }),
      contentStore: store({ "index.html": "new" }),
    })).rejects.toMatchObject<Partial<FilesystemPublishError>>({ code: "ROOT_UNSAFE" });
  });

  it("rejects a publication root whose parent does not exist", async () => {
    const root = join(await temporarySite(), "missing-parent", "site");

    await expect(publishFilesystemSite({
      root,
      manifest: manifest({ "index.html": "new" }),
      contentStore: store({ "index.html": "new" }),
    })).rejects.toMatchObject<Partial<FilesystemPublishError>>({ code: "ROOT_UNSAFE" });
  });

  it("rejects a linked root without modifying its external target", async () => {
    const root = await temporarySite();
    const outside = await mkdtemp(join(tmpdir(), "forme-fs-outside-"));
    roots.push(outside);
    await writeFile(join(outside, "sentinel.txt"), "safe");
    await symlink(outside, root, process.platform === "win32" ? "junction" : "dir");

    await expect(publishFilesystemSite({
      root,
      manifest: manifest({ "index.html": "new" }),
      contentStore: store({ "index.html": "new" }),
    })).rejects.toMatchObject<Partial<FilesystemPublishError>>({ code: "ROOT_UNSAFE" });

    expect(await readFile(join(outside, "sentinel.txt"), "utf8")).toBe("safe");
    expect(await readdir(outside)).toEqual(["sentinel.txt"]);
  });

  it("rejects linked components in an existing tree without following them", async () => {
    const root = await temporarySite();
    const outside = await mkdtemp(join(tmpdir(), "forme-fs-linked-"));
    roots.push(outside);
    await writeFile(join(outside, "sentinel.txt"), "safe");
    await mkdir(root);
    await symlink(outside, join(root, "linked"), process.platform === "win32" ? "junction" : "dir");

    await expect(publishFilesystemSite({
      root,
      manifest: manifest({ "index.html": "new" }),
      contentStore: store({ "index.html": "new" }),
    })).rejects.toMatchObject<Partial<FilesystemPublishError>>({ code: "TARGET_UNSAFE" });

    expect(await readFile(join(outside, "sentinel.txt"), "utf8")).toBe("safe");
  });

  it("fails closed when a nested target entry disappears during scanning", async () => {
    const root = await temporarySite();
    await mkdir(join(root, "nested"), { recursive: true });
    const disappearing = join(root, "nested", "vanish.txt");
    await writeFile(disappearing, "old");
    fsFaults.beforeLstat = async path => {
      if (!path.endsWith("/nested/vanish.txt")) return false;
      await rm(disappearing, { force: true });
      return true;
    };

    await expect(publishFilesystemSite({ root, manifest: manifest({}), contentStore: store({}) }))
      .rejects.toMatchObject<Partial<FilesystemPublishError>>({ code: "TARGET_UNSAFE" });

    expect(await lstat(root)).toMatchObject({});
    expect(await readdir(root)).toEqual(["nested"]);
  });

  it.skipIf(process.platform === "win32")("rejects multiply linked target files", async () => {
    const root = await temporarySite();
    const outside = join(dirname(root), "outside.txt");
    await mkdir(root);
    await writeFile(outside, "same");
    await link(outside, join(root, "index.html"));

    await expect(publishFilesystemSite({
      root,
      manifest: manifest({ "index.html": "same" }),
      contentStore: store({ "index.html": "same" }),
    })).rejects.toMatchObject<Partial<FilesystemPublishError>>({ code: "TARGET_UNSAFE" });

    expect(await readFile(outside, "utf8")).toBe("same");
    expect((await lstat(outside)).nlink).toBe(2);
  });

  it("rejects replacement of the authorized parent filesystem object", async () => {
    const outer = await mkdtemp(join(tmpdir(), "forme-fs-parent-swap-"));
    roots.push(outer);
    const parent = join(outer, "parent");
    const movedParent = join(outer, "original-parent");
    const root = join(parent, "site");
    await mkdir(parent);
    const files = { "index.html": "new" };
    const transaction = await prepareFilesystemPublication({ root, manifest: manifest(files), contentStore: store(files) });
    await rename(parent, movedParent);
    await mkdir(parent);
    await writeFile(join(parent, "sentinel.txt"), "replacement");

    await expect(transaction.commit())
      .rejects.toMatchObject<Partial<FilesystemPublishError>>({ code: "ROOT_UNSAFE" });

    expect(await readFile(join(parent, "sentinel.txt"), "utf8")).toBe("replacement");
    expect((await readdir(parent)).sort()).toEqual(["sentinel.txt"]);
    expect((await readdir(movedParent)).some(path => path.includes("forme-stage-"))).toBe(true);
  });

  it("does not report unchanged after the verified target root is substituted", async () => {
    const root = await temporarySite();
    const files = { "index.html": "same" };
    await publishFilesystemSite({ root, manifest: manifest(files), contentStore: store(files) });
    const originalRoot = `${root}-original`;
    const baseStore = store(files);
    let substituted = false;
    const substitutingStore: ContentStore = {
      ...baseStore,
      get: async (sha256, signal) => {
        const value = await baseStore.get(sha256, signal);
        if (!substituted) {
          substituted = true;
          await rename(root, originalRoot);
          await mkdir(root);
          await writeFile(join(root, "index.html"), "same");
        }
        return value;
      },
    };

    await expect(publishFilesystemSite({ root, manifest: manifest(files), contentStore: substitutingStore }))
      .rejects.toMatchObject<Partial<FilesystemPublishError>>({ code: "TARGET_UNSAFE" });

    expect(await readFile(join(root, "index.html"), "utf8")).toBe("same");
    expect(await readFile(join(originalRoot, "index.html"), "utf8")).toBe("same");
  });

  it("does not delete a replacement staging directory", async () => {
    const root = await temporarySite();
    const files = { "index.html": "new" };
    const transaction = await prepareFilesystemPublication({ root, manifest: manifest(files), contentStore: store(files) });
    const stageName = (await artifacts(root)).find(path => path.includes("forme-stage-"));
    expect(stageName).toBeDefined();
    const stagePath = join(dirname(root), stageName!);
    await rename(stagePath, `${stagePath}-original`);
    await mkdir(stagePath);
    await writeFile(join(stagePath, "sentinel.txt"), "replacement");

    await expect(transaction.commit())
      .rejects.toMatchObject<Partial<FilesystemPublishError>>({ code: "STAGING_UNSAFE" });
    await expect(transaction.rollback())
      .rejects.toMatchObject<Partial<FilesystemPublishError>>({ code: "ROLLBACK_FAILED" });

    expect(await readFile(join(stagePath, "sentinel.txt"), "utf8")).toBe("replacement");
  });

  it("does not move the old root into a substituted backup container", async () => {
    const root = await temporarySite();
    const old = { "index.html": "old" };
    await publishFilesystemSite({ root, manifest: manifest(old), contentStore: store(old) });
    const next = { "index.html": "new" };
    const transaction = await prepareFilesystemPublication({ root, manifest: manifest(next), contentStore: store(next) });
    let replacementPath: string | undefined;
    fsFaults.afterRealpath = async path => {
      if (basename(path) !== basename(root)) return false;
      const backupName = (await artifacts(root)).find(name => name.includes("forme-backup-"));
      if (backupName === undefined) return false;
      replacementPath = join(dirname(root), backupName);
      await rename(replacementPath, `${replacementPath}-original`);
      await mkdir(replacementPath);
      await writeFile(join(replacementPath, "sentinel.txt"), "replacement");
      return true;
    };

    await expect(transaction.commit())
      .rejects.toMatchObject<Partial<FilesystemPublishError>>({ code: "COMMIT_FAILED" });
    await transaction.rollback();

    expect(await readFile(join(root, "index.html"), "utf8")).toBe("old");
    expect(await readFile(join(replacementPath!, "sentinel.txt"), "utf8")).toBe("replacement");
  });

  it("cannot mutate the root after another publication replaces its lock", async () => {
    const root = await temporarySite();
    const files = { "index.html": "new" };
    const first = await prepareFilesystemPublication({ root, manifest: manifest(files), contentStore: store(files) });
    const lockName = (await artifacts(root)).find(path => path.endsWith("forme-lock"));
    expect(lockName).toBeDefined();
    const lockPath = join(dirname(root), lockName!);
    await rename(lockPath, `${lockPath}-original`);
    const second = await prepareFilesystemPublication({ root, manifest: manifest(files), contentStore: store(files) });

    await expect(first.commit())
      .rejects.toMatchObject<Partial<FilesystemPublishError>>({ code: "COMMIT_FAILED" });
    await expect(lstat(root)).rejects.toMatchObject({ code: "ENOENT" });
    await expect(first.rollback())
      .rejects.toMatchObject<Partial<FilesystemPublishError>>({ code: "ROLLBACK_FAILED" });

    await second.commit();
    await second.finalize();
    expect(await readFile(join(root, "index.html"), "utf8")).toBe("new");
  });

  it("bounds existing-tree entry count and nesting depth", async () => {
    const entryRoot = await temporarySite();
    await mkdir(entryRoot);
    await Promise.all(["a", "b", "c"].map(name => writeFile(join(entryRoot, name), name)));
    await expect(publishFilesystemSite({
      root: entryRoot,
      manifest: manifest({}),
      contentStore: store({}),
      scanLimits: { maxEntries: 2 },
    })).rejects.toMatchObject<Partial<FilesystemPublishError>>({ code: "TARGET_UNSAFE" });

    const deepRoot = await temporarySite();
    await mkdir(join(deepRoot, "one", "two", "three"), { recursive: true });
    await expect(publishFilesystemSite({
      root: deepRoot,
      manifest: manifest({}),
      contentStore: store({}),
      scanLimits: { maxDepth: 2 },
    })).rejects.toMatchObject<Partial<FilesystemPublishError>>({ code: "TARGET_UNSAFE" });

    const metadataRoot = await temporarySite();
    await mkdir(metadataRoot);
    await writeFile(join(metadataRoot, "long-name.txt"), "x");
    await expect(publishFilesystemSite({
      root: metadataRoot,
      manifest: manifest({}),
      contentStore: store({}),
      scanLimits: { maxMetadataBytes: 4 },
    })).rejects.toMatchObject<Partial<FilesystemPublishError>>({ code: "TARGET_UNSAFE" });
  });
});

describe("prepared filesystem transaction", () => {
  it("holds an exclusive target lock and releases it on prepared rollback", async () => {
    const root = await temporarySite();
    const next = { "index.html": "new" };
    const first = await prepareFilesystemPublication({ root, manifest: manifest(next), contentStore: store(next) });

    await expect(prepareFilesystemPublication({ root, manifest: manifest(next), contentStore: store(next) }))
      .rejects.toMatchObject<Partial<FilesystemPublishError>>({ code: "TARGET_BUSY" });

    await first.rollback();
    expect(first.state).toBe("rolled-back");
    expect(await artifacts(root)).toEqual([]);
    const second = await prepareFilesystemPublication({ root, manifest: manifest(next), contentStore: store(next) });
    await second.rollback();
  });

  it("does not report an old exact tree as unchanged while a replacement holds the lock", async () => {
    const root = await temporarySite();
    const old = { "index.html": "old" };
    await publishFilesystemSite({ root, manifest: manifest(old), contentStore: store(old) });
    const next = { "index.html": "new" };
    const replacement = await prepareFilesystemPublication({
      root,
      manifest: manifest(next),
      contentStore: store(next),
    });

    await expect(publishFilesystemSite({ root, manifest: manifest(old), contentStore: store(old) }))
      .rejects.toMatchObject<Partial<FilesystemPublishError>>({ code: "TARGET_BUSY" });

    await replacement.rollback();
    expect(await readFile(join(root, "index.html"), "utf8")).toBe("old");
  });

  it("allows an unchanged prepared publication to roll back without writes", async () => {
    const root = await temporarySite();
    const files = { "index.html": "same" };
    await publishFilesystemSite({ root, manifest: manifest(files), contentStore: store(files) });
    const before = await stat(join(root, "index.html"), { bigint: true });
    const transaction = await prepareFilesystemPublication({ root, manifest: manifest(files), contentStore: store(files) });

    expect(transaction.changed).toBe(false);
    await transaction.rollback();

    expect(transaction.state).toBe("rolled-back");
    const after = await stat(join(root, "index.html"), { bigint: true });
    expect(after.ino).toBe(before.ino);
    expect(after.mtimeNs).toBe(before.mtimeNs);
  });

  it("revalidates the root immediately before commit", async () => {
    const root = await temporarySite();
    const outside = await mkdtemp(join(tmpdir(), "forme-fs-commit-link-"));
    roots.push(outside);
    await writeFile(join(outside, "sentinel.txt"), "safe");
    const files = { "index.html": "new" };
    const transaction = await prepareFilesystemPublication({ root, manifest: manifest(files), contentStore: store(files) });
    await symlink(outside, root, process.platform === "win32" ? "junction" : "dir");

    await expect(transaction.commit())
      .rejects.toMatchObject<Partial<FilesystemPublishError>>({ code: "ROOT_UNSAFE" });
    await transaction.rollback();

    expect(await readFile(join(outside, "sentinel.txt"), "utf8")).toBe("safe");
    expect(await artifacts(root)).toEqual([]);
  });

  it("restores the exact old tree when a committed transaction rolls back", async () => {
    const root = await temporarySite();
    const old = { "old.txt": "old" };
    await publishFilesystemSite({ root, manifest: manifest(old), contentStore: store(old) });
    const next = { "new.txt": "new" };
    const transaction = await prepareFilesystemPublication({ root, manifest: manifest(next), contentStore: store(next) });

    expect(transaction.state).toBe("prepared");
    await transaction.commit();
    expect(transaction.state).toBe("committed");
    expect(await readFile(join(root, "new.txt"), "utf8")).toBe("new");

    await transaction.rollback();
    expect(transaction.state).toBe("rolled-back");
    expect(await readFile(join(root, "old.txt"), "utf8")).toBe("old");
    await expect(readFile(join(root, "new.txt"))).rejects.toThrow();
    expect(await artifacts(root)).toEqual([]);
  });

  it("retries rollback cleanup without moving or deleting the restored old site", async () => {
    const root = await temporarySite();
    const old = { "old.txt": "old" };
    await publishFilesystemSite({ root, manifest: manifest(old), contentStore: store(old) });
    const next = { "new.txt": "new" };
    const transaction = await prepareFilesystemPublication({ root, manifest: manifest(next), contentStore: store(next) });
    await transaction.commit();
    fsFaults.rmNeedle = ".forme-rollback-";
    fsFaults.rmRemaining = 1;

    await expect(transaction.rollback())
      .rejects.toMatchObject<Partial<FilesystemPublishError>>({ code: "ROLLBACK_FAILED" });
    expect(await readFile(join(root, "old.txt"), "utf8")).toBe("old");
    await expect(readFile(join(root, "new.txt"))).rejects.toThrow();

    await transaction.rollback();

    expect(transaction.state).toBe("rolled-back");
    expect(await readFile(join(root, "old.txt"), "utf8")).toBe("old");
    expect(await artifacts(root)).toEqual([]);
  });

  it("does not move the published root into a substituted rollback container", async () => {
    const root = await temporarySite();
    const old = { "old.txt": "old" };
    await publishFilesystemSite({ root, manifest: manifest(old), contentStore: store(old) });
    const next = { "new.txt": "new" };
    const transaction = await prepareFilesystemPublication({ root, manifest: manifest(next), contentStore: store(next) });
    await transaction.commit();
    let replacementPath: string | undefined;
    fsFaults.afterRealpath = async path => {
      if (!path.endsWith("forme-lock")) return false;
      const rollbackName = (await artifacts(root)).find(name => name.includes("forme-rollback-"));
      if (rollbackName === undefined) return false;
      replacementPath = join(dirname(root), rollbackName);
      await rename(replacementPath, `${replacementPath}-original`);
      await mkdir(replacementPath);
      await writeFile(join(replacementPath, "sentinel.txt"), "replacement");
      return true;
    };

    await expect(transaction.rollback())
      .rejects.toMatchObject<Partial<FilesystemPublishError>>({ code: "ROLLBACK_FAILED" });

    expect(await readFile(join(root, "new.txt"), "utf8")).toBe("new");
    expect(await readFile(join(replacementPath!, "sentinel.txt"), "utf8")).toBe("replacement");
  });

  it("preserves committed state when the authorized parent disappears during finalize", async () => {
    const outer = await mkdtemp(join(tmpdir(), "forme-fs-finalize-parent-"));
    roots.push(outer);
    const parent = join(outer, "parent");
    const movedParent = join(outer, "moved-parent");
    const root = join(parent, "site");
    await mkdir(parent);
    const old = { "old.txt": "old" };
    await publishFilesystemSite({ root, manifest: manifest(old), contentStore: store(old) });
    const next = { "new.txt": "new" };
    const transaction = await prepareFilesystemPublication({ root, manifest: manifest(next), contentStore: store(next) });
    await transaction.commit();
    await rename(parent, movedParent);

    await expect(transaction.finalize())
      .rejects.toMatchObject<Partial<FilesystemPublishError>>({ code: "CLEANUP_FAILED" });
    expect(transaction.state).toBe("committed");

    await rename(movedParent, parent);
    await transaction.finalize();
    expect(transaction.state).toBe("finalized");
    expect(await readFile(join(root, "new.txt"), "utf8")).toBe("new");
  });

  it("preserves prepared state when the authorized parent disappears during rollback", async () => {
    const outer = await mkdtemp(join(tmpdir(), "forme-fs-rollback-parent-"));
    roots.push(outer);
    const parent = join(outer, "parent");
    const movedParent = join(outer, "moved-parent");
    const root = join(parent, "site");
    await mkdir(parent);
    const files = { "index.html": "new" };
    const transaction = await prepareFilesystemPublication({ root, manifest: manifest(files), contentStore: store(files) });
    await rename(parent, movedParent);

    await expect(transaction.rollback())
      .rejects.toMatchObject<Partial<FilesystemPublishError>>({ code: "ROLLBACK_FAILED" });
    expect(transaction.state).toBe("prepared");

    await rename(movedParent, parent);
    await transaction.rollback();
    expect(transaction.state).toBe("rolled-back");
    expect(await artifacts(root)).toEqual([]);
  });

  it("removes a newly published root when a rootless transaction rolls back", async () => {
    const root = await temporarySite();
    const files = { "index.html": "new" };
    const transaction = await prepareFilesystemPublication({ root, manifest: manifest(files), contentStore: store(files) });
    await transaction.commit();
    await transaction.rollback();
    await expect(lstat(root)).rejects.toMatchObject({ code: "ENOENT" });
    expect(await artifacts(root)).toEqual([]);
  });

  it("finalizes a committed transaction and forbids rollback afterward", async () => {
    const root = await temporarySite();
    const files = { "index.html": "new" };
    const transaction = await prepareFilesystemPublication({ root, manifest: manifest(files), contentStore: store(files) });
    await transaction.commit();
    await transaction.finalize();

    expect(transaction.state).toBe("finalized");
    await expect(transaction.rollback())
      .rejects.toMatchObject<Partial<FilesystemPublishError>>({ code: "INVALID_STATE" });
    expect(await readFile(join(root, "index.html"), "utf8")).toBe("new");
    expect(await artifacts(root)).toEqual([]);
  });

  it("crosses the irreversible boundary when backup deletion starts and fails", async () => {
    const root = await temporarySite();
    const old = { "old.txt": "old" };
    await publishFilesystemSite({ root, manifest: manifest(old), contentStore: store(old) });
    const next = { "new.txt": "new" };
    const transaction = await prepareFilesystemPublication({ root, manifest: manifest(next), contentStore: store(next) });
    await transaction.commit();
    fsFaults.rmNeedle = ".forme-backup-";
    fsFaults.rmRemaining = 1;

    await expect(transaction.finalize())
      .rejects.toMatchObject<Partial<FilesystemPublishError>>({ code: "CLEANUP_FAILED" });

    expect(transaction.state).toBe("finalized");
    await expect(transaction.rollback())
      .rejects.toMatchObject<Partial<FilesystemPublishError>>({ code: "INVALID_STATE" });
    expect(await readFile(join(root, "new.txt"), "utf8")).toBe("new");
    const competing = await prepareFilesystemPublication({
      root,
      manifest: manifest({ "third.txt": "third" }),
      contentStore: store({ "third.txt": "third" }),
    });
    await competing.rollback();
  });

  it("rejects transaction operations outside their state machine", async () => {
    const root = await temporarySite();
    const files = { "index.html": "new" };
    const transaction = await prepareFilesystemPublication({ root, manifest: manifest(files), contentStore: store(files) });

    await expect(transaction.finalize())
      .rejects.toMatchObject<Partial<FilesystemPublishError>>({ code: "INVALID_STATE" });
    await transaction.commit();
    await expect(transaction.commit())
      .rejects.toMatchObject<Partial<FilesystemPublishError>>({ code: "INVALID_STATE" });
    await transaction.rollback();
    await expect(transaction.rollback())
      .rejects.toMatchObject<Partial<FilesystemPublishError>>({ code: "INVALID_STATE" });
  });
});
