import { access, link, mkdtemp, open, readFile, readdir, rm, symlink, writeFile } from "node:fs/promises";
import { constants } from "node:fs";
import { execFileSync } from "node:child_process";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { afterEach, describe, expect, it, vi } from "vitest";
import { executeDeploy, materializeDeployInput } from "../src/index.js";

const roots: string[] = [];
const encoder = new TextEncoder();

afterEach(async () => {
  vi.unstubAllEnvs();
  vi.unstubAllGlobals();
  await Promise.all(roots.splice(0).map(root => rm(root, { recursive: true, force: true })));
});

async function temporaryRoot(): Promise<string> {
  const root = await mkdtemp(join(tmpdir(), "forme-cli-deploy-"));
  roots.push(root);
  return root;
}

describe("deploy input packaging", () => {
  it("merges named dist-tree outputs into one strict digest store and writes the manifest last", async () => {
    const root = await temporaryRoot();
    const result = await materializeDeployInput({
      pages: { variant: { kind: "dist-tree" }, files: { "index.html": encoder.encode("home") } },
      assets: [{ variant: { kind: "dist-tree" }, files: { "assets/app.css": encoder.encode("body{}") } }],
    }, ".forme/deploy-input", root);

    const manifest = JSON.parse(await readFile(result.manifestPath, "utf8")) as {
      fileCount: number;
      files: Record<string, { sha256: string }>;
    };
    expect(manifest.fileCount).toBe(2);
    expect(Object.keys(manifest.files)).toEqual(["assets/app.css", "index.html"]);
    for (const file of Object.values(manifest.files)) {
      const key = Buffer.from(file.sha256, "base64").toString("base64url");
      await access(join(result.contentDirectory, `${key}.bin`));
    }

    const replaced = await materializeDeployInput({
      pages: { variant: { kind: "dist-tree" }, files: { "next.html": encoder.encode("next") } },
    }, ".forme/deploy-input", root);
    const nextManifest = JSON.parse(await readFile(replaced.manifestPath, "utf8")) as { files: Record<string, unknown> };
    expect(Object.keys(nextManifest.files)).toEqual(["next.html"]);
    const contentEntries = await readdir(replaced.contentDirectory);
    expect(contentEntries).toHaveLength(1);
  });

  it("rejects output collisions and deploy-input paths outside the project", async () => {
    const root = await temporaryRoot();
    const artifact = { variant: { kind: "dist-tree" }, files: { "index.html": encoder.encode("home") } };
    await expect(materializeDeployInput({ one: artifact, two: artifact }, "input", root))
      .rejects.toThrow(/collide/);
    await expect(materializeDeployInput({ one: artifact }, "../outside", root))
      .rejects.toThrow(/child/);
  });

  it("rejects a linked deploy-input parent before deleting or writing outside the project", async () => {
    const root = await temporaryRoot();
    const outside = await temporaryRoot();
    await symlink(outside, join(root, "linked"), process.platform === "win32" ? "junction" : "dir");
    const artifact = { variant: { kind: "dist-tree" }, files: { "index.html": encoder.encode("home") } };
    await expect(materializeDeployInput({ site: artifact }, "linked/deploy-input", root))
      .rejects.toThrow(/real directories/);
    await expect(access(join(outside, "deploy-input"))).rejects.toThrow();
  });
});

describe("executeDeploy", () => {
  it("performs target-aware filesystem dry-run without writes, then publishes", async () => {
    const root = await temporaryRoot();
    const packaged = await materializeDeployInput({
      site: { variant: { kind: "dist-tree" }, files: { "index.html": encoder.encode("home") } },
    }, "input", root);
    const target = join(root, "published");
    const config = join(root, "fs.json");
    await writeFile(config, JSON.stringify({ root: target }));
    const base = {
      cwd: root,
      manifestPath: packaged.manifestPath,
      content: { kind: "directory" as const, path: packaged.contentDirectory },
      target: "fs" as const,
      targetConfigPath: config,
      retryLimit: 0,
    };

    const dry = JSON.parse(await executeDeploy({ ...base, dryRun: true })) as { summary: { created: number } };
    expect(dry.summary.created).toBe(1);
    await expect(access(target)).rejects.toThrow();

    const published = JSON.parse(await executeDeploy({ ...base, dryRun: false })) as {
      summary: { totalBytesWritten: number };
    };
    expect(published.summary.totalBytesWritten).toBe(4);
    expect(await readFile(join(target, "index.html"), "utf8")).toBe("home");
  });

  it("authenticates the GET-only GitHub dry-run boundary when the fixed token is available", async () => {
    const root = await temporaryRoot();
    const packaged = await materializeDeployInput({
      site: { variant: { kind: "dist-tree" }, files: { "index.html": encoder.encode("home") } },
    }, "input", root);
    const targetConfig = join(root, "github-pages.json");
    await writeFile(targetConfig, JSON.stringify({
      owner: "octo",
      repository: "site",
      ref: "heads/gh-pages",
      deploymentOwner: "blog",
      destination: "blog",
      tokenEnv: "GITHUB_TOKEN",
    }));
    let authorization: string | null = null;
    vi.stubEnv("GITHUB_TOKEN", "read-token");
    vi.stubGlobal("fetch", vi.fn(async (_input: string | URL | Request, init?: RequestInit) => {
      authorization = new Headers(init?.headers).get("Authorization");
      return new Response('{"message":"stop after observing the request"}', {
        status: 403,
        headers: { "Content-Type": "application/json" },
      });
    }));

    await expect(executeDeploy({
      cwd: root,
      manifestPath: packaged.manifestPath,
      content: { kind: "directory", path: packaged.contentDirectory },
      target: "github-pages",
      targetConfigPath: targetConfig,
      retryLimit: 0,
      dryRun: true,
    })).rejects.toThrow(/403/);
    expect(authorization).toBe("Bearer read-token");
  });

  it("rejects unexpected directory-store entries without reading them", async () => {
    const root = await temporaryRoot();
    const packaged = await materializeDeployInput({
      site: { variant: { kind: "dist-tree" }, files: { "index.html": encoder.encode("home") } },
    }, "input", root);
    await writeFile(join(packaged.contentDirectory, "unexpected.bin"), "not manifest content");
    const targetConfig = join(root, "fs.json");
    await writeFile(targetConfig, JSON.stringify({ root: join(root, "published") }));

    await expect(executeDeploy({
      cwd: root,
      manifestPath: packaged.manifestPath,
      content: { kind: "directory", path: packaged.contentDirectory },
      target: "fs",
      targetConfigPath: targetConfig,
      retryLimit: 0,
      dryRun: true,
    })).rejects.toThrow(/exactly match/);
  });

  it("accepts an exact canonical bundle and rejects trailing data", async () => {
    const root = await temporaryRoot();
    const packaged = await materializeDeployInput({
      site: { variant: { kind: "dist-tree" }, files: { "index.html": encoder.encode("home") } },
    }, "input", root);
    const manifest = JSON.parse(await readFile(packaged.manifestPath, "utf8")) as {
      files: Record<string, { sha256: string }>;
    };
    const digest = manifest.files["index.html"]!.sha256;
    const body = Buffer.from("home");
    const count = Buffer.alloc(4); count.writeUInt32BE(1);
    const length = Buffer.alloc(8); length.writeBigUInt64BE(BigInt(body.byteLength));
    const bundleBytes = Buffer.concat([
      Buffer.from("FORME-CONTENT-BUNDLE-V1\n"), count, Buffer.from(digest, "base64"), length, body,
    ]);
    const bundle = join(root, "content.forme-bundle");
    const targetConfig = join(root, "fs.json");
    await writeFile(bundle, bundleBytes);
    await writeFile(targetConfig, JSON.stringify({ root: join(root, "published") }));
    const invocation = {
      cwd: root,
      manifestPath: packaged.manifestPath,
      content: { kind: "bundle" as const, path: bundle },
      target: "fs" as const,
      targetConfigPath: targetConfig,
      retryLimit: 0,
      dryRun: true,
    };
    await expect(executeDeploy(invocation)).resolves.toContain('"status": "success"');
    await writeFile(bundle, Buffer.concat([bundleBytes, Buffer.from("extra")]));
    await expect(executeDeploy(invocation)).rejects.toThrow(/trailing/);
  });

  it("reads a manifest-exact canonical-base64 map from an explicit descriptor", async () => {
    const root = await temporaryRoot();
    const packaged = await materializeDeployInput({
      site: { variant: { kind: "dist-tree" }, files: { "index.html": encoder.encode("home") } },
    }, "input", root);
    const manifest = JSON.parse(await readFile(packaged.manifestPath, "utf8")) as {
      files: Record<string, { sha256: string }>;
    };
    const digest = manifest.files["index.html"]!.sha256;
    const inlinePath = join(root, "inline.json");
    await writeFile(inlinePath, JSON.stringify({ [digest]: Buffer.from("home").toString("base64") }));
    const handle = await open(inlinePath, "r");
    const targetConfig = join(root, "fs.json");
    await writeFile(targetConfig, JSON.stringify({ root: join(root, "published") }));
    try {
      await expect(executeDeploy({
        cwd: root,
        manifestPath: packaged.manifestPath,
        content: { kind: "inline", fd: handle.fd },
        target: "fs",
        targetConfigPath: targetConfig,
        retryLimit: 0,
        dryRun: true,
      })).resolves.toContain('"status": "success"');
    } finally { await handle.close(); }
  });

  it.skipIf(process.platform === "win32")("rejects a blocking inline FIFO without closing the caller's descriptor", async () => {
    const root = await temporaryRoot();
    const packaged = await materializeDeployInput({
      site: { variant: { kind: "dist-tree" }, files: { "index.html": encoder.encode("home") } },
    }, "input", root);
    const fifo = join(root, "content.fifo");
    execFileSync("mkfifo", [fifo]);
    const handle = await open(fifo, constants.O_RDWR);
    const targetConfig = join(root, "fs.json");
    await writeFile(targetConfig, JSON.stringify({ root: join(root, "published") }));
    const pending = executeDeploy({
      cwd: root,
      manifestPath: packaged.manifestPath,
      content: { kind: "inline", fd: handle.fd },
      target: "fs",
      targetConfigPath: targetConfig,
      retryLimit: 0,
      dryRun: true,
    });
    try {
      await expect(pending).rejects.toThrow(/regular file/);
      await expect(handle.stat()).resolves.toBeDefined();
    } finally {
      await handle.close();
    }
  });

  it("rejects duplicate JSON keys in bounded target configuration", async () => {
    const root = await temporaryRoot();
    const packaged = await materializeDeployInput({
      site: { variant: { kind: "dist-tree" }, files: { "index.html": encoder.encode("home") } },
    }, "input", root);
    const targetConfig = join(root, "fs.json");
    await writeFile(targetConfig, '{"root":"one","root":"two"}');
    await expect(executeDeploy({
      cwd: root,
      manifestPath: packaged.manifestPath,
      content: { kind: "directory", path: packaged.contentDirectory },
      target: "fs",
      targetConfigPath: targetConfig,
      retryLimit: 0,
      dryRun: true,
    })).rejects.toThrow(/duplicate key/);
  });

  it("rejects multiply linked configuration files", async () => {
    const root = await temporaryRoot();
    const packaged = await materializeDeployInput({
      site: { variant: { kind: "dist-tree" }, files: { "index.html": encoder.encode("home") } },
    }, "input", root);
    const original = join(root, "original-fs.json");
    const targetConfig = join(root, "linked-fs.json");
    await writeFile(original, JSON.stringify({ root: join(root, "published") }));
    await link(original, targetConfig);
    await expect(executeDeploy({
      cwd: root,
      manifestPath: packaged.manifestPath,
      content: { kind: "directory", path: packaged.contentDirectory },
      target: "fs",
      targetConfigPath: targetConfig,
      retryLimit: 0,
      dryRun: true,
    })).rejects.toThrow(/unlinked regular file/);
  });
});
