import { afterEach, beforeEach, describe, expect, it } from "vitest";
import { createHash } from "node:crypto";
import { mkdtemp, readFile, readdir, rm } from "node:fs/promises";
import { join } from "node:path";
import { tmpdir } from "node:os";
import {
  Kinds,
  streamOf,
  type Asset,
  type DeployArtifact,
  type LogicalId,
  type RenderedPage,
} from "@coding-adventures/forme-types";
import {
  createCancellationTokenSource,
  frozenClock,
  silentLogger,
} from "@coding-adventures/forme-stage";
import { defineStage } from "@coding-adventures/forme-stage";
import { createOrchestrator } from "@coding-adventures/forme-orchestrator";
import emitSiteFs, {
  fingerprintedAssetFilename,
  rewriteAssetPlaceholders,
  sha256Hex,
} from "../src/index.js";

const ID_A = "01952c0d-7e63-7000-8000-000000000041" as LogicalId;
const ID_B = "01952c0d-7e63-7000-8000-000000000042" as LogicalId;
const roots: string[] = [];
let outDir: string;

beforeEach(async () => {
  outDir = await mkdtemp(join(tmpdir(), "forme-emit-site-"));
  roots.push(outDir);
});

afterEach(async () => {
  await Promise.all(roots.splice(0).map(root => rm(root, { recursive: true, force: true })));
});

function asset(
  id: LogicalId = ID_A,
  bytes: Uint8Array = new Uint8Array([1, 2, 3]),
  sourcePath = "images/cat.png",
): Asset {
  return {
    identity: id,
    revision: "blake2b:00" as never,
    role: "image",
    mimeType: "image/png",
    bytes,
    byteLength: bytes.byteLength,
    dimensions: null,
    durationMs: null,
    derivedFrom: null,
    meta: { sourcePath },
  };
}

function page(options: {
  route?: string;
  html?: string;
  usedAssets?: readonly LogicalId[];
} = {}): RenderedPage {
  return {
    route: options.route ?? "/post/index.html",
    html: options.html ?? `<img src="forme-asset:${ID_A}?width=400#hero">`,
    usedStyle: [],
    usedIslands: [],
    usedAssets: options.usedAssets ?? [ID_A],
    meta: {
      title: "Post",
      description: null,
      canonicalUrl: null,
      openGraph: {},
      structured: [],
      extra: {},
    },
    source: "01952c0d-7e63-7000-8000-000000000001" as LogicalId,
  };
}

async function* values<T>(...items: T[]): AsyncIterable<T> {
  yield* items;
}

function context(cancelled = false): Parameters<typeof emitSiteFs.run>[2] {
  const cancellation = createCancellationTokenSource();
  if (cancelled) cancellation.cancel("test cancellation");
  return {
    logger: silentLogger(),
    cancellation: cancellation.token,
    time: frozenClock({ timestamp: Date.parse("2026-08-28T00:00:00.000Z") }),
  } as never;
}

async function runSite(
  pages: readonly RenderedPage[],
  assets: readonly Asset[],
  config: unknown = { outDir },
  cancelled = false,
): Promise<DeployArtifact> {
  return await (emitSiteFs.run as Function)(
    { default: values(...pages), assets: values(...assets) },
    config,
    context(cancelled),
  ) as DeployArtifact;
}

describe("emitSiteFs contract", () => {
  it("declares typed page and asset fan-in", () => {
    expect(emitSiteFs.consumes).toEqual(streamOf(Kinds.RenderedPage));
    expect(emitSiteFs.inputPorts).toEqual({ assets: streamOf(Kinds.Asset) });
    expect(emitSiteFs.produces).toEqual(Kinds.DeployArtifact);
    expect(emitSiteFs.capabilities).toEqual(["filesystem:write"]);
  });
});

describe("fingerprinted static-site emission", () => {
  it("rewrites placeholders, preserves suffixes, writes bytes, and records assets", async () => {
    const bytes = new Uint8Array([0x89, 0x50, 0x4e, 0x47]);
    const digest = createHash("sha256").update(bytes).digest("hex");
    const filename = `cat.${digest}.png`;
    const artifact = await runSite([page()], [asset(ID_A, bytes)]);

    expect(await readFile(join(outDir, "post/index.html"), "utf8"))
      .toBe(`<img src="/assets/${filename}?width=400#hero">`);
    expect(new Uint8Array(await readFile(join(outDir, "assets", filename)))).toEqual(bytes);
    expect(new TextDecoder().decode(artifact.files["post/index.html"]!))
      .toContain(`/assets/${filename}?width=400#hero`);
    expect(artifact.files[`assets/${filename}`]).toEqual(bytes);
    expect(artifact.manifest.assets).toEqual([{
      id: ID_A,
      path: `assets/${filename}`,
      mime: "image/png",
      sha256: digest,
    }]);
    expect(artifact.manifest.routes).toMatchObject([{
      pattern: "/post/index.html",
      target: { kind: "file", path: "post/index.html" },
    }]);
    expect(artifact.manifest.buildTime).toBe("2026-08-28T00:00:00.000Z");
    expect(artifact.manifest.buildId).toMatch(/^blake2b:[0-9a-f]{64}$/);
  });

  it("uses a portable custom asset directory and URI-encodes public segments", async () => {
    const bytes = new Uint8Array([9]);
    const digest = sha256Hex(bytes);
    const artifact = await runSite(
      [page()],
      [asset(ID_A, bytes, "images/cat photo.png")],
      { outDir, assetDir: "static/media files", publicPathPrefix: "/coding adventures" },
    );
    const path = `static/media files/cat photo.${digest}.png`;
    expect(artifact.manifest.assets[0]!.path).toBe(path);
    expect(new TextDecoder().decode(artifact.files["post/index.html"]!))
      .toContain(`/coding%20adventures/static/media%20files/cat%20photo.${digest}.png?width=400#hero`);
  });

  it("retains unrelated URLs and rejects undeclared or missing placeholders", async () => {
    const ordinary = page({
      html: `<a href="https://example.com/a.png">external</a><img src="data:image/png;base64,AA==">`,
      usedAssets: [],
    });
    const emitted = await runSite([ordinary], []);
    expect(new TextDecoder().decode(emitted.files["post/index.html"]!)).toBe(ordinary.html);

    await expect(runSite([page({ usedAssets: [ID_B] })], [asset()]))
      .rejects.toThrow(/references missing asset/);
    await expect(runSite([page({ usedAssets: [] })], [asset()]))
      .rejects.toThrow(/undeclared or malformed/);
  });

  it("deduplicates identical file outputs while retaining logical manifest entries", async () => {
    const bytes = new Uint8Array([7, 7]);
    const artifact = await runSite(
      [page({ usedAssets: [ID_A, ID_B], html: `<img src="forme-asset:${ID_A}"><img src="forme-asset:${ID_B}">` })],
      [asset(ID_B, bytes), asset(ID_A, bytes)],
    );
    expect(artifact.manifest.assets.map(entry => entry.id)).toEqual([ID_A, ID_B]);
    expect(Object.keys(artifact.files).filter(path => path.startsWith("assets/"))).toHaveLength(1);
  });

  it("is deterministic across page and asset stream order", async () => {
    const first = await runSite(
      [page({ route: "/b.html" }), page({ route: "/a.html" })],
      [asset(ID_B, new Uint8Array([2]), "b.png"), asset(ID_A, new Uint8Array([1]), "a.png")],
    );
    await rm(outDir, { recursive: true, force: true });
    outDir = await mkdtemp(join(tmpdir(), "forme-emit-site-repeat-"));
    roots.push(outDir);
    const second = await runSite(
      [page({ route: "/a.html" }), page({ route: "/b.html" })],
      [asset(ID_A, new Uint8Array([1]), "a.png"), asset(ID_B, new Uint8Array([2]), "b.png")],
    );
    expect(second.manifest.buildId).toBe(first.manifest.buildId);
  });
});

describe("validation and safety", () => {
  it("validates config, source paths, sha256 helpers, and byte lengths", async () => {
    expect(fingerprintedAssetFilename("images/cat.png", "a".repeat(64)))
      .toBe(`cat.${"a".repeat(64)}.png`);
    expect(() => fingerprintedAssetFilename("../cat.png", "a".repeat(64))).toThrow(/portable path/);
    expect(() => fingerprintedAssetFilename("cat.png", "bad")).toThrow(/sha256/);
    await expect(runSite([], [], {})).rejects.toThrow(/config.outDir/);
    await expect(runSite([], [], { outDir, assetDir: "../assets" })).rejects.toThrow(/config.assetDir/);
    await expect(runSite([], [], { outDir, assetDir: "C:/assets" })).rejects.toThrow(/config.assetDir/);
    for (const publicPathPrefix of ["coding-adventures", "//host", "/../escape", "/trailing/"]) {
      await expect(runSite([], [], { outDir, publicPathPrefix }))
        .rejects.toThrow(/config.publicPathPrefix/);
    }
    await expect(runSite([], [{ ...asset(), byteLength: 99 }])).rejects.toThrow(/byteLength/);
    await expect(runSite([], [{ ...asset(), meta: {} }])).rejects.toThrow(/meta.sourcePath/);
    await expect(runSite([], [asset(ID_A, new Uint8Array([1]), "C:/cat.png")]))
      .rejects.toThrow(/portable path/);
  });

  it("rejects duplicate identities, route traversal, and page/asset collisions", async () => {
    await expect(runSite([], [asset(), asset()])).rejects.toThrow(/duplicate asset identity/);
    await expect(runSite([page({ route: "/../../escape.html" })], [asset()]))
      .rejects.toThrow(/escape outDir/);
    const digest = sha256Hex(asset().bytes);
    await expect(runSite([
      page({ route: `/assets/cat.${digest}.png`, html: "page", usedAssets: [] }),
    ], [asset()])).rejects.toThrow(/collides with output/);
  });

  it("checks cancellation before materialization and leaves the output empty", async () => {
    await expect(runSite([page()], [asset()], { outDir }, true))
      .rejects.toThrow("test cancellation");
    expect(await readdir(outDir)).toEqual([]);
  });

  it("rewrites only declared renderer placeholders", () => {
    const original = page({ html: `prefix forme-asset:${ID_A}#icon suffix` });
    expect(rewriteAssetPlaceholders(original, new Map([[ID_A, "/assets/a.svg"]])))
      .toBe("prefix /assets/a.svg#icon suffix");
  });
});

describe("orchestrator end-to-end", () => {
  it("joins explicit page and asset wires exactly once into a deploy artifact", async () => {
    const pages = defineStage({
      name: "@test/rendered-pages",
      version: "0.1.0",
      apiVersion: 1,
      description: "fixture pages",
      consumes: Kinds.Void,
      produces: streamOf(Kinds.RenderedPage),
      capabilities: [],
      configSchema: null,
      async *run() { yield page(); },
    });
    const assets = defineStage({
      name: "@test/assets",
      version: "0.1.0",
      apiVersion: 1,
      description: "fixture assets",
      consumes: Kinds.Void,
      produces: streamOf(Kinds.Asset),
      capabilities: [],
      configSchema: null,
      async *run() { yield asset(); },
    });
    const orchestrator = createOrchestrator({ logger: silentLogger() });
    const pipeline = await orchestrator.buildPipeline({
      name: "asset-emission-e2e",
      settings: {
        storageRoot: ".",
        cacheDir: null,
        reproducibleBuild: true,
        maxConcurrency: null,
        logLevel: "error",
        bestEffort: false,
        deadlineMs: null,
      },
      stages: [
        { id: "pages", stage: pages, config: null },
        { id: "assets", stage: assets, config: null },
        { id: "site", stage: emitSiteFs, config: { outDir } },
      ],
      wires: [
        { from: { id: "pages" }, to: { id: "site" } },
        { from: { id: "assets" }, to: { id: "site", port: "assets" } },
      ],
      outputs: [{ fromInstance: "site", name: "site" }],
    } as never);
    const result = await orchestrator.runOnce(pipeline);
    await orchestrator.dispose();

    expect(result.outcome).toBe("success");
    const artifact = result.outputs.site as DeployArtifact;
    expect(artifact.manifest.routes).toHaveLength(1);
    expect(artifact.manifest.assets).toHaveLength(1);
    expect(new TextDecoder().decode(artifact.files["post/index.html"]!))
      .toMatch(/src="\/assets\/cat\.[0-9a-f]{64}\.png\?width=400#hero"/);
  });
});
