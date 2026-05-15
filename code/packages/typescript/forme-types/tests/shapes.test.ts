/**
 * forme-types — Kind shape construction tests
 *
 * Builds a representative value of every kernel kind and verifies
 * basic invariants (e.g. byteLength matches bytes.length).  The bulk
 * of the work is at compile time: the type system rejects any value
 * that doesn't match the spec.
 *
 * These tests serve as living documentation — copy from here when
 * authoring a stage that needs to construct a value of a given kind.
 */

import { describe, it, expect } from "vitest";
import {
  EMPTY_INTERACTIVITY,
  EMPTY_STYLE,
} from "../src/index.js";
import type {
  Asset, AssetRef,
  Collection, CollectionEntry,
  ContentNode, ContentSource,
  DeployArtifact, DeployManifest,
  Document, Feed,
  IslandId, LogicalId,
  PrintForme, RenderedPage, RequestHandler, RevisionId,
  SearchIndex, Stream,
  StyleRuleId,
} from "../src/index.js";

// ─── Helpers ──────────────────────────────────────────────────────────────

const SAMPLE_ID    = "01952c0d-7e63-7000-8000-000000000000" as LogicalId;
const SAMPLE_REV   = "blake2b:cafebabe" as RevisionId;
const SAMPLE_BYTES = new TextEncoder().encode("hello world");

// ─── ContentSource ────────────────────────────────────────────────────────

describe("ContentSource", () => {
  it("constructs a minimal source", () => {
    const src: ContentSource = {
      path: "posts/hello.md",
      bytes: SAMPLE_BYTES,
      mimeType: "text/markdown",
      identity: SAMPLE_ID,
      revision: SAMPLE_REV,
      providerMeta: {},
    };
    expect(src.bytes.byteLength).toBe(11);
    expect(src.mimeType).toBe("text/markdown");
  });

  it("permits a null mimeType when the source can't sniff one", () => {
    const src: ContentSource = {
      path: "blob.bin",
      bytes: new Uint8Array([0, 1, 2]),
      mimeType: null,
      identity: SAMPLE_ID,
      revision: SAMPLE_REV,
      providerMeta: { from: "manual" },
    };
    expect(src.mimeType).toBeNull();
  });
});

// ─── ContentNode ──────────────────────────────────────────────────────────

describe("ContentNode", () => {
  it("wraps a DocumentNode with frontmatter and route", () => {
    const node: ContentNode = {
      identity: SAMPLE_ID,
      revision: SAMPLE_REV,
      document: { type: "document", children: [] },
      frontmatter: { title: "Hello", date: "2026-05-14" },
      route: "/posts/hello",
      assetRefs: [],
      sourcePath: "posts/hello.md",
    };
    expect(node.document.type).toBe("document");
    expect(node.frontmatter.title).toBe("Hello");
  });

  it("permits a null route before a collector assigns one", () => {
    const node: ContentNode = {
      identity: SAMPLE_ID,
      revision: SAMPLE_REV,
      document: { type: "document", children: [] },
      frontmatter: {},
      route: null,
      assetRefs: [],
      sourcePath: "posts/draft.md",
    };
    expect(node.route).toBeNull();
  });

  it("supports asset references with a node-path locator", () => {
    const ref: AssetRef = {
      id: "01952c0d-7e63-7000-8000-000000000111" as LogicalId,
      nodePath: [0, 2, 1],
      role: "image",
    };
    const node: ContentNode = {
      identity: SAMPLE_ID,
      revision: SAMPLE_REV,
      document: { type: "document", children: [] },
      frontmatter: {},
      route: null,
      assetRefs: [ref],
      sourcePath: "posts/with-image.md",
    };
    expect(node.assetRefs[0]?.role).toBe("image");
    expect(node.assetRefs[0]?.nodePath).toEqual([0, 2, 1]);
  });
});

// ─── Collection ───────────────────────────────────────────────────────────

describe("Collection", () => {
  it("orders entries by an OrderKey", () => {
    const entry: CollectionEntry = {
      identity: SAMPLE_ID,
      revision: SAMPLE_REV,
      route: "/posts/hello",
      orderKey: { kind: "date", value: "2026-05-14T00:00:00Z" },
      overlay: { excerpt: "Hi" },
    };
    const c: Collection = {
      name: "posts",
      entries: [entry],
      discriminant: "chronological",
      meta: { count: 1 },
    };
    expect(c.entries.length).toBe(1);
    expect(c.discriminant).toBe("chronological");
  });

  it("supports composite ordering for tie-breaking", () => {
    const entry: CollectionEntry = {
      identity: SAMPLE_ID,
      revision: SAMPLE_REV,
      route: null,
      orderKey: {
        kind: "composite",
        value: [
          { kind: "date", value: "2026-05-14" },
          { kind: "lexicographic", value: "alpha" },
        ],
      },
      overlay: {},
    };
    expect(entry.orderKey.kind).toBe("composite");
  });
});

// ─── Asset ────────────────────────────────────────────────────────────────

describe("Asset", () => {
  it("constructs a raster image asset with dimensions", () => {
    const asset: Asset = {
      identity: SAMPLE_ID,
      revision: SAMPLE_REV,
      role: "image",
      mimeType: "image/png",
      bytes: SAMPLE_BYTES,
      byteLength: SAMPLE_BYTES.byteLength,
      dimensions: { w: 800, h: 600 },
      durationMs: null,
      derivedFrom: null,
      meta: {},
    };
    expect(asset.dimensions?.w).toBe(800);
    expect(asset.derivedFrom).toBeNull();
  });

  it("links derived assets to their original", () => {
    const original = SAMPLE_ID;
    const resized: Asset = {
      identity: "01952c0d-7e63-7000-8000-000000000222" as LogicalId,
      revision: SAMPLE_REV,
      role: "image",
      mimeType: "image/avif",
      bytes: new Uint8Array(),
      byteLength: 0,
      dimensions: { w: 400, h: 300 },
      durationMs: null,
      derivedFrom: original,
      meta: { variant: "thumb" },
    };
    expect(resized.derivedFrom).toBe(original);
  });
});

// ─── Document / RenderedPage / PrintForme ─────────────────────────────────

describe("Document", () => {
  it("composes content + style + interactivity with an assigned route", () => {
    const content: ContentNode = {
      identity: SAMPLE_ID,
      revision: SAMPLE_REV,
      document: { type: "document", children: [] },
      frontmatter: {},
      route: "/posts/hello",
      assetRefs: [],
      sourcePath: "posts/hello.md",
    };
    const doc: Document = {
      identity: SAMPLE_ID,
      revision: SAMPLE_REV,
      content,
      style: EMPTY_STYLE,
      interactivity: EMPTY_INTERACTIVITY,
      route: "/posts/hello",
    };
    expect(doc.style.theme).toBeNull();
    expect(doc.interactivity.islands.length).toBe(0);
  });
});

describe("RenderedPage", () => {
  it("carries html, used-style/island/asset arrays, and source provenance", () => {
    const page: RenderedPage = {
      route: "/posts/hello",
      html: "<!doctype html><html><body>hi</body></html>",
      usedStyle: ["body" as StyleRuleId],
      usedIslands: ["search" as IslandId],
      usedAssets: [],
      meta: {
        title: "Hello",
        description: null,
        canonicalUrl: null,
        openGraph: {},
        structured: [],
        extra: {},
      },
      source: SAMPLE_ID,
    };
    expect(page.usedStyle.length).toBe(1);
    expect(page.usedIslands.length).toBe(1);
  });
});

describe("PrintForme", () => {
  it("describes a print-ready page with explicit geometry", () => {
    const print: PrintForme = {
      source: SAMPLE_ID,
      page: {
        size: { kind: "named", name: "A4" },
        margins: {
          top:    { unit: "mm", value: 25 },
          right:  { unit: "mm", value: 20 },
          bottom: { unit: "mm", value: 25 },
          left:   { unit: "mm", value: 20 },
        },
        orientation: "portrait",
      },
      runningElements: [],
      content: {
        identity: SAMPLE_ID,
        revision: SAMPLE_REV,
        document: { type: "document", children: [] },
        frontmatter: {},
        route: "/print",
        assetRefs: [],
        sourcePath: "src.md",
      },
      style: EMPTY_STYLE,
      usedAssets: [],
    };
    expect(print.page.orientation).toBe("portrait");
    if (print.page.size.kind === "named") {
      expect(print.page.size.name).toBe("A4");
    }
  });

  it("supports custom page sizes via Length", () => {
    const print: PrintForme = {
      source: SAMPLE_ID,
      page: {
        size: {
          kind: "custom",
          w: { unit: "in", value: 8.5 },
          h: { unit: "in", value: 11 },
        },
        margins: {
          top: { unit: "in", value: 1 }, right:  { unit: "in", value: 1 },
          bottom: { unit: "in", value: 1 }, left: { unit: "in", value: 1 },
        },
        orientation: "landscape",
      },
      runningElements: [],
      content: {
        identity: SAMPLE_ID,
        revision: SAMPLE_REV,
        document: { type: "document", children: [] },
        frontmatter: {},
        route: "/print",
        assetRefs: [],
        sourcePath: "src.md",
      },
      style: EMPTY_STYLE,
      usedAssets: [],
    };
    if (print.page.size.kind === "custom") {
      expect(print.page.size.w.value).toBe(8.5);
    }
  });
});

// ─── RequestHandler / SearchIndex / Feed / DeployArtifact ─────────────────

describe("RequestHandler", () => {
  it("describes a Cloudflare Worker handler", () => {
    const h: RequestHandler = {
      routePattern: "/api/echo",
      code: "export default { fetch(req) { return new Response('hi'); } }",
      runtime: { kind: "cloudflare-worker" },
      staticAssets: [],
    };
    expect(h.runtime.kind).toBe("cloudflare-worker");
  });

  it("describes a Node 22+ handler with min version", () => {
    const h: RequestHandler = {
      routePattern: "/api/v2/*",
      code: "module.exports = (req, res) => res.end('hi');",
      runtime: { kind: "node", minVersion: "22.0.0" },
      staticAssets: [SAMPLE_ID],
    };
    if (h.runtime.kind === "node") {
      expect(h.runtime.minVersion).toBe("22.0.0");
    }
  });
});

describe("SearchIndex", () => {
  it("packages indexer name and serialised files", () => {
    const idx: SearchIndex = {
      indexer: "pagefind",
      indexer_version: "1.0.0",
      files: { "pagefind.js": new Uint8Array([0x2f, 0x2f]) },
      manifest: { lang: "en" },
    };
    expect(idx.files["pagefind.js"]?.byteLength).toBe(2);
  });
});

describe("Feed", () => {
  it("packages a feed format and bytes", () => {
    const feed: Feed = {
      format: "rss",
      files: { "/feed.xml": new TextEncoder().encode("<rss></rss>") },
    };
    expect(feed.format).toBe("rss");
  });
});

describe("DeployArtifact", () => {
  it("describes a static-tree deploy with manifest", () => {
    const manifest: DeployManifest = {
      routes: [{
        pattern: "/posts/hello",
        target:  { kind: "file", path: "posts/hello.html" },
        islands: [],
        css:     [],
      }],
      assets: [],
      buildTime: "2026-05-14T12:00:00Z",
      buildId:   SAMPLE_REV,
    };
    const artifact: DeployArtifact = {
      variant: { kind: "dist-tree" },
      files: { "posts/hello.html": new TextEncoder().encode("<!doctype html>") },
      manifest,
    };
    expect(artifact.variant.kind).toBe("dist-tree");
    expect(artifact.manifest.routes.length).toBe(1);
  });

  it("describes a worker-bundle deploy with runtime", () => {
    const artifact: DeployArtifact = {
      variant: { kind: "worker-bundle", runtime: { kind: "cloudflare-worker" } },
      files: {},
      manifest: {
        routes: [],
        assets: [],
        buildTime: "2026-05-14T12:00:00Z",
        buildId:   SAMPLE_REV,
      },
    };
    if (artifact.variant.kind === "worker-bundle") {
      expect(artifact.variant.runtime.kind).toBe("cloudflare-worker");
    }
  });
});

// ─── Stream value type ────────────────────────────────────────────────────

describe("Stream value type", () => {
  it("wraps an iterator factory", async () => {
    async function* gen() { yield 1; yield 2; yield 3; }
    const s: Stream<number> = { iterator: () => gen() };
    const out: number[] = [];
    for await (const v of s.iterator()) out.push(v);
    expect(out).toEqual([1, 2, 3]);
  });
});

// ─── Empty constants ──────────────────────────────────────────────────────

describe("EMPTY_STYLE / EMPTY_INTERACTIVITY", () => {
  it("EMPTY_STYLE has no tokens, no rules, no theme", () => {
    expect(Object.keys(EMPTY_STYLE.tokens).length).toBe(0);
    expect(EMPTY_STYLE.rules.length).toBe(0);
    expect(EMPTY_STYLE.theme).toBeNull();
  });

  it("EMPTY_INTERACTIVITY has no state, bindings, handlers, or islands", () => {
    expect(EMPTY_INTERACTIVITY.state.length).toBe(0);
    expect(EMPTY_INTERACTIVITY.bindings.length).toBe(0);
    expect(EMPTY_INTERACTIVITY.handlers.length).toBe(0);
    expect(EMPTY_INTERACTIVITY.islands.length).toBe(0);
  });
});
