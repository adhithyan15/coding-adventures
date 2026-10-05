import { afterEach, beforeEach, describe, expect, it } from "vitest";
import { createHash } from "node:crypto";
import { mkdtemp, readFile, readdir, rm, symlink, writeFile } from "node:fs/promises";
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
import { filesystemCache } from "@coding-adventures/forme-cache";
import { computeRevisionId, createOutputProvenance } from "@coding-adventures/forme-identity";
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
  role: Asset["role"] = "image",
  mimeType = "image/png",
): Asset {
  return {
    identity: id,
    revision: "blake2b:00" as never,
    role,
    mimeType,
    bytes,
    byteLength: bytes.byteLength,
    dimensions: null,
    durationMs: null,
    derivedFrom: null,
    meta: { sourcePath },
  };
}

function moduleUse(island: string, id: LogicalId, bytes: Uint8Array) {
  return {
    island: island as never,
    asset: id,
    packageName: "@example/island",
    export: "enhance",
    sha256: sha256Hex(bytes),
  };
}

function page(options: {
  route?: string;
  html?: string;
  usedAssets?: readonly LogicalId[];
  usedIslands?: readonly RenderedPage["usedIslands"][number][];
  islandModules?: NonNullable<RenderedPage["islandModules"]>;
} = {}): RenderedPage {
  return {
    route: options.route ?? "/post/index.html",
    html: options.html ?? `<img src="forme-asset:${ID_A}?width=400#hero">`,
    usedStyle: [],
    usedIslands: options.usedIslands ?? [],
    islandModules: options.islandModules ?? [],
    usedAssets: options.usedAssets ?? [ID_A],
    meta: {
      title: "Post",
      description: null,
      canonicalUrl: null,
      openGraph: {},
      structured: [],
      extra: {},
    },
    provenance: createOutputProvenance([{
        identity: "01952c0d-7e63-7000-8000-000000000001" as LogicalId,
        revision: `blake2b:${"a".repeat(64)}` as never,
      }]),
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
  it("rejects a legacy source-only page before writing output", async () => {
    const { provenance: _provenance, ...legacy } = page({ route: "/legacy.html" });
    await expect(runSite([{
      ...legacy,
      source: "01952c0d-7e63-7000-8000-000000000099",
    } as never], [])).rejects.toThrow(/page.provenance is required/);
    await expect(readFile(join(outDir, "legacy.html"))).rejects.toThrow();
  });

  it("emits only selected island scripts and preserves zero-JavaScript pages", async () => {
    const selected = ID_A;
    const unused = ID_B;
    const selectedBytes = new TextEncoder().encode("export function enhanceCounter() {}\n");
    const unusedBytes = new TextEncoder().encode("export function unused() {}\n");
    const interactive = page({
      route: "/interactive.html",
      html: "<p>fallback</p></body>",
      usedAssets: [selected],
      usedIslands: ["counter" as never],
      islandModules: [moduleUse("counter", selected, selectedBytes)],
    });
    const staticPage = page({
      route: "/static.html",
      html: "<p>static</p></body>",
      usedAssets: [],
    });
    const artifact = await runSite(
      [interactive, staticPage],
      [
        asset(selected, selectedBytes, "islands/counter.js", "script", "text/javascript"),
        asset(unused, unusedBytes, "islands/unused.js", "script", "text/javascript"),
      ],
    );

    const interactiveHtml = new TextDecoder().decode(artifact.files["interactive.html"]!);
    const staticHtml = new TextDecoder().decode(artifact.files["static.html"]!);
    expect(interactiveHtml).toMatch(/<script type="module" src="\/assets\/counter\.[0-9a-f]{64}\.js"><\/script><\/body>/);
    expect(staticHtml).not.toContain("<script");
    expect(Object.keys(artifact.files).some(path => path.includes("unused."))).toBe(false);
    expect(artifact.manifest.assets.map(entry => entry.id)).toEqual([selected]);
    expect(artifact.manifest.routes).toMatchObject([
      { pattern: "/interactive.html", islands: ["counter"] },
      { pattern: "/static.html", islands: [] },
    ]);
  });

  it("rejects mismatched island-module usage and non-script module assets", async () => {
    const mismatched = page({
      usedAssets: [ID_A],
      usedIslands: ["counter" as never],
      islandModules: [],
    });
    await expect(runSite([mismatched], [asset()])).rejects.toThrow(/islandModules must match usedIslands/);

    const wrongRole = page({
      usedAssets: [ID_A],
      usedIslands: ["counter" as never],
      islandModules: [moduleUse("counter", ID_A, new Uint8Array([1, 2, 3]))],
    });
    await expect(runSite([wrongRole], [asset()])).rejects.toThrow(/must reference a script asset/);
  });

  it("deduplicates one selected module asset used by multiple island IDs", async () => {
    const bytes = new TextEncoder().encode("export const enhance = () => {};\n");
    const artifact = await runSite([page({
      html: "<body></body>",
      usedAssets: [ID_A],
      usedIslands: ["first" as never, "second" as never],
      islandModules: [
        moduleUse("first", ID_A, bytes),
        moduleUse("second", ID_A, bytes),
      ],
    })], [asset(ID_A, bytes, "islands/shared.js", "script", "application/javascript")]);
    const html = new TextDecoder().decode(artifact.files["post/index.html"]!);
    expect(html.match(/<script /g)).toHaveLength(1);
    expect(artifact.manifest.routes[0]!.islands).toEqual(["first", "second"]);
  });

  it("rejects executable bytes that differ from the reviewed digest", async () => {
    const reviewed = new Uint8Array([1, 2, 3]);
    const actual = new Uint8Array([1, 2, 4]);
    await expect(runSite([page({
      html: "<body></body>",
      usedAssets: [ID_A],
      usedIslands: ["counter" as never],
      islandModules: [moduleUse("counter", ID_A, reviewed)],
    })], [asset(ID_A, actual, "islands/counter.js", "script", "text/javascript")]))
      .rejects.toThrow(/do not match the reviewed SHA-256/);
  });

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

  it("replays every artifact file after the output tree is deleted", async () => {
    const artifact = await runSite([page()], [asset()]);
    await rm(outDir, { recursive: true, force: true });

    await emitSiteFs.replay!(artifact, { outDir }, context());

    expect(await readFile(join(outDir, "post/index.html"), "utf8"))
      .toContain("/assets/cat.");
    expect((await readdir(join(outDir, "assets"))).length).toBe(1);
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

  it("preserves prototype-named routes as own artifact files", async () => {
    const artifact = await runSite([page({ route: "/__proto__", html: "safe", usedAssets: [] })], []);
    expect(Object.hasOwn(artifact.files, "__proto__")).toBe(true);
    expect(new TextDecoder().decode(artifact.files["__proto__"]!)).toBe("safe");
  });
});

describe("validation and safety", () => {
  it("bounds retained page bytes and aggregate usage before snapshot copies", async () => {
    await expect(runSite([page({
      html: "x".repeat(16 * 1024 * 1024 + 1),
      usedAssets: [],
    })], [])).rejects.toThrow(/UTF-8 limit/);

    const twelveMiB = "y".repeat(12 * 1024 * 1024);
    await expect(runSite([
      page({ route: "/a.html", html: twelveMiB, usedAssets: [] }),
      page({ route: "/b.html", html: twelveMiB, usedAssets: [] }),
      page({ route: "/c.html", html: twelveMiB, usedAssets: [] }),
    ], [])).rejects.toThrow(/site page content.*UTF-8 limit/);

    const manyUses = Array<LogicalId>(32_769).fill(ID_A);
    await expect(runSite([
      page({ route: "/a.html", html: "a", usedAssets: manyUses }),
      page({ route: "/b.html", html: "b", usedAssets: manyUses }),
    ], [])).rejects.toThrow(/site page usage.*entry limit/);

    await expect(runSite([page({
      usedAssets: [],
      islandModules: [{
        ...moduleUse("Search", ID_A, new Uint8Array([1])),
        packageName: `@scope/${"a".repeat(208)}`,
      }] as never,
    })], [])).rejects.toThrow(/reviewed island-module binding/);
  });

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
    await expect(runSite([], [
      asset(ID_A, new Uint8Array([1]), "same.js", "script", "text/javascript"),
      asset(ID_B, new Uint8Array([1]), "same.js", "binary", "text/javascript"),
    ])).rejects.toThrow(/incompatible asset path collision/);
    await expect(runSite([], [
      asset(ID_A, new Uint8Array([1]), "foo.js", "script", "text/javascript"),
      asset(ID_B, new Uint8Array([1]), "FOO.js", "script", "text/javascript"),
    ])).rejects.toThrow(/portable asset path collision/);

    const scriptBytes = new Uint8Array([7]);
    const scriptPath = `assets/foo.${sha256Hex(scriptBytes)}.js`;
    await expect(runSite([page({
      route: `/${scriptPath.replace("assets/", "Assets/")}`,
      html: "<body></body>",
      usedAssets: [ID_A],
      usedIslands: ["counter" as never],
      islandModules: [moduleUse("counter", ID_A, scriptBytes)],
    })], [asset(ID_A, scriptBytes, "foo.js", "script", "text/javascript")]))
      .rejects.toThrow(/collides with output/);
    await expect(runSite([page({
      route: `/${scriptPath}.`,
      html: "page",
      usedAssets: [],
    })], [asset(ID_A, scriptBytes, "foo.js", "script", "text/javascript")]))
      .rejects.toThrow(/not portable across filesystems/);
  });

  it("snapshots bounded page usage without invoking accessors", async () => {
    const hostile = page({ usedAssets: [] }) as unknown as Record<string, unknown>;
    let reads = 0;
    Object.defineProperty(hostile, "usedAssets", { enumerable: true, get: () => { reads++; return []; } });
    await expect(runSite([hostile as never], [])).rejects.toThrow(/must not be an accessor/);
    expect(reads).toBe(0);

    const tooMany = Array.from({ length: 257 }, (_, index) => `island_${index}` as never);
    await expect(runSite([page({ usedAssets: [], usedIslands: tooMany })], []))
      .rejects.toThrow(/at most 256 entries/);
  });

  it("rejects unsafe or malformed replay artifacts", async () => {
    const artifact = await runSite([page()], [asset()]);
    await rm(outDir, { recursive: true, force: true });
    await expect(emitSiteFs.replay!({
      ...artifact,
      files: { "a.html": new Uint8Array([1]), "z/../escape.html": new Uint8Array([2]) },
    }, { outDir }, context())).rejects.toThrow(/normalized portable relative path/);
    await expect(readFile(join(outDir, "a.html"))).rejects.toThrow();
    await expect(emitSiteFs.replay!({
      ...artifact,
      files: { "foo/D:outside/pwn.html": new Uint8Array([1]) },
    }, { outDir }, context())).rejects.toThrow(/normalized portable relative path/);
    await expect(emitSiteFs.replay!({
      ...artifact,
      variant: { kind: "pdf", pageCount: 1 },
    }, { outDir }, context())).rejects.toThrow(/dist-tree DeployArtifact/);
    await expect(emitSiteFs.replay!({
      ...artifact,
      files: { Foo: new Uint8Array([1]), foo: new Uint8Array([2]) },
    }, { outDir }, context())).rejects.toThrow(/collide on portable filesystems/);
    await expect(readFile(join(outDir, "Foo"))).rejects.toThrow();
  });

  it("rejects directory symlinks beneath outDir during replay", async () => {
    const artifact = await runSite([page()], [asset()]);
    const outside = await mkdtemp(join(tmpdir(), "forme-emit-site-outside-"));
    roots.push(outside);
    await symlink(outside, join(outDir, "linked"), process.platform === "win32" ? "junction" : "dir");

    await expect(emitSiteFs.replay!({
      ...artifact,
      files: { "linked/pwn.html": new TextEncoder().encode("outside") },
    }, { outDir }, context())).rejects.toThrow(/real directory/);
    await expect(readFile(join(outside, "pwn.html"))).rejects.toThrow();
  });

  it.skipIf(process.platform === "win32")(
    "rejects final-file symlinks without modifying their targets",
    async () => {
      const artifact = await runSite([page()], [asset()]);
      const outside = await mkdtemp(join(tmpdir(), "forme-emit-site-file-link-"));
      roots.push(outside);
      const target = join(outside, "target.html");
      await writeFile(target, "original");
      await symlink(target, join(outDir, "linked.html"), "file");

      await expect(emitSiteFs.replay!({
        ...artifact,
        files: { "linked.html": new TextEncoder().encode("replacement") },
      }, { outDir }, context())).rejects.toThrow(/symbolic link/);
      expect(await readFile(target, "utf8")).toBe("original");
    },
  );

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

  it("rewrites a large declared asset set in one HTML pass", () => {
    const ids = Array.from({ length: 4_096 }, (_, index) =>
      `01952c0d-7e63-7000-8000-${index.toString(16).padStart(12, "0")}` as LogicalId);
    const paths = new Map(ids.map(id => [id, `/assets/${id}.bin`] as const));
    const original = page({
      usedAssets: ids,
      html: `${"ordinary text ".repeat(10_000)}forme-asset:${ids.at(-1)!}`,
    });
    expect(rewriteAssetPlaceholders(original, paths)).toContain(`/assets/${ids.at(-1)!}.bin`);
  });
});

describe("orchestrator end-to-end", () => {
  it("joins explicit page and asset wires exactly once into a deploy artifact", async () => {
    const pages = defineStage({
      name: "@test/rendered-pages",
      version: "0.1.0",
      apiVersion: 2,
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
      apiVersion: 2,
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

  it("drains sibling page and asset streams without a two-window fan-in deadlock", async () => {
    const count = 256;
    const source = defineStage({
      name: "@test/shared-content",
      version: "0.1.0",
      apiVersion: 2,
      description: "shared source large enough to fill both bounded page windows",
      consumes: Kinds.Void,
      produces: streamOf(Kinds.ContentSource),
      capabilities: [],
      configSchema: null,
      async *run() {
        for (let index = 0; index < count; index++) {
          const path = `post-${index}.md`;
          yield {
            path,
            bytes: new TextEncoder().encode(path),
            mimeType: "text/markdown",
            identity: ID_A,
            revision: "blake2b:00",
            providerMeta: {},
          } as never;
        }
      },
    });
    const pages = defineStage({
      name: "@test/shared-pages",
      version: "0.1.0",
      apiVersion: 2,
      description: "renders every shared source without assets",
      consumes: streamOf(Kinds.ContentSource),
      produces: streamOf(Kinds.RenderedPage),
      capabilities: [],
      configSchema: null,
      async *run(input) {
        let index = 0;
        for await (const _item of input as AsyncIterable<unknown>) {
          yield page({ route: `/post-${index++}.html`, html: "page", usedAssets: [] });
        }
      },
    });
    const assets = defineStage({
      name: "@test/shared-assets",
      version: "0.1.0",
      apiVersion: 2,
      description: "drains the same shared source while producing no assets",
      consumes: streamOf(Kinds.ContentSource),
      produces: streamOf(Kinds.Asset),
      capabilities: [],
      configSchema: null,
      async *run(input) {
        for await (const _item of input as AsyncIterable<unknown>) {
          // This fixture intentionally contains no asset references.
        }
      },
    });
    const orchestrator = createOrchestrator({ logger: silentLogger() });
    const pipeline = await orchestrator.buildPipeline({
      name: "bounded-fan-in",
      settings: {
        storageRoot: ".",
        cacheDir: null,
        reproducibleBuild: true,
        maxConcurrency: 4,
        logLevel: "error",
        bestEffort: false,
        deadlineMs: null,
      },
      stages: [
        { id: "source", stage: source },
        { id: "pages", stage: pages },
        { id: "assets", stage: assets },
        { id: "site", stage: emitSiteFs, config: { outDir } },
      ],
      wires: [
        { from: { id: "source" }, to: { id: "pages" } },
        { from: { id: "source" }, to: { id: "assets" } },
        { from: { id: "pages" }, to: { id: "site" } },
        { from: { id: "assets" }, to: { id: "site", port: "assets" } },
      ],
      outputs: [{ fromInstance: "site", name: "site" }],
    } as never);

    const result = await Promise.race([
      orchestrator.runOnce(pipeline),
      new Promise<never>((_resolve, reject) => setTimeout(
        () => reject(new Error("page/asset fan-in did not settle")),
        10_000,
      )),
    ]);
    await orchestrator.dispose();

    expect(result.outcome).toBe("success");
    expect((result.outputs.site as DeployArtifact).manifest.routes).toHaveLength(count);
  }, 15_000);

  it("restores a deleted site from persistent checkpoints without rerunning producers", async () => {
    const cacheRoot = await mkdtemp(join(tmpdir(), "forme-emit-site-cache-"));
    roots.push(cacheRoot);
    const pageCalls: string[] = [];
    const assetCalls: string[] = [];
    const observedStream = <T>(options: {
      name: string;
      kind: typeof Kinds.RenderedPage | typeof Kinds.Asset;
      value: T;
      identity: LogicalId;
      revision: string;
      calls: string[];
    }) => defineStage({
      name: options.name,
      version: "1.0.0",
      apiVersion: 2,
      description: "fixed observed fixture source",
      consumes: Kinds.Void,
      produces: streamOf(options.kind),
      capabilities: [],
      configSchema: null,
      externalState() {
        const entries = [{
          locator: options.name,
          identity: options.identity,
          revision: options.revision as never,
        }];
        return { version: 1, entries, revision: computeRevisionId({ version: 1, entries }) };
      },
      async *run() {
        options.calls.push("run");
        yield options.value as never;
      },
    });
    const pages = observedStream({
      name: "@test/observed-pages",
      kind: Kinds.RenderedPage,
      value: page(),
      identity: ID_A,
      revision: "blake2b:" + "1".repeat(64),
      calls: pageCalls,
    });
    const assets = observedStream({
      name: "@test/observed-assets",
      kind: Kinds.Asset,
      value: asset(),
      identity: ID_B,
      revision: "blake2b:" + "2".repeat(64),
      calls: assetCalls,
    });
    const pipelineConfig = {
      name: "asset-emission-replay-e2e",
      settings: {
        storageRoot: ".", cacheDir: cacheRoot, reproducibleBuild: true,
        maxConcurrency: null, logLevel: "error", bestEffort: false, deadlineMs: null,
      },
      stages: [
        { id: "pages", stage: pages },
        { id: "assets", stage: assets },
        { id: "site", stage: emitSiteFs, config: { outDir } },
      ],
      wires: [
        { from: { id: "pages" }, to: { id: "site" } },
        { from: { id: "assets" }, to: { id: "site", port: "assets" } },
      ],
      outputs: [{ fromInstance: "site", name: "site" }],
    } as never;
    const runFresh = async () => {
      const orchestrator = createOrchestrator({
        cache: filesystemCache(cacheRoot),
        logger: silentLogger(),
      });
      const result = await orchestrator.runOnce(await orchestrator.buildPipeline(pipelineConfig));
      await orchestrator.dispose();
      return result;
    };

    expect((await runFresh()).stages.map(stage => stage.outcome))
      .toEqual(["success", "success", "success"]);
    await rm(outDir, { recursive: true, force: true });
    expect((await runFresh()).stages.map(stage => stage.outcome))
      .toEqual(["skipped", "skipped", "skipped"]);
    expect(pageCalls).toHaveLength(1);
    expect(assetCalls).toHaveLength(1);
    expect(await readFile(join(outDir, "post/index.html"), "utf8"))
      .toContain("/assets/cat.");
  });
});
