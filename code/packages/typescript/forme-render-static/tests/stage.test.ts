/**
 * stage.test.ts — render stage integration tests.
 *
 * Verifies stage shape and the full end-to-end transform.  Inputs
 * are constructed via the real parser (gfm-parser → ContentNode-ish
 * objects) so the round-trip exercises actual production code rather
 * than hand-rolled mock ASTs.
 */

import { describe, it, expect } from "vitest";
import {
  Kinds,
  streamOf,
  type ContentNode,
  type OutputProvenance,
} from "@coding-adventures/forme-types";
import {
  createCancellationTokenSource,
  inMemoryCache,
  inMemoryEventBus,
  noOpTelemetryEmitter,
  silentLogger,
  systemClock,
  deniedEnvApi,
  deniedFilesystemApi,
  deniedNetworkApi,
  deniedShellApi,
  deniedStorageApi,
  type StageContext,
} from "@coding-adventures/forme-stage";
import { parse as parseGfm } from "@coding-adventures/gfm-parser";
import renderStatic from "../src/index.js";

function makeCtx(overrides: Partial<StageContext> = {}): StageContext {
  return {
    logger: silentLogger(),
    cancellation: createCancellationTokenSource().token,
    time: systemClock(),
    cache: inMemoryCache(),
    telemetry: noOpTelemetryEmitter(),
    storage: deniedStorageApi(),
    network: deniedNetworkApi(),
    env: deniedEnvApi(),
    filesystem: deniedFilesystemApi(),
    shell: deniedShellApi(),
    events: inMemoryEventBus(),
    ...overrides,
  };
}

let seq = 0;
function makeNode(opts: {
  sourcePath: string;
  markdown: string;
  frontmatter?: Record<string, string>;
  route?: string | null;
}): ContentNode {
  seq++;
  const id = `00000000-0000-7000-8000-${String(seq).padStart(12, "0")}` as ContentNode["identity"];
  return {
    identity: id,
    revision: ("blake2b:" + "0".repeat(64)) as ContentNode["revision"],
    document: parseGfm(opts.markdown) as unknown as ContentNode["document"],
    frontmatter: opts.frontmatter ?? {},
    route: opts.route === undefined ? `/routed/${seq}.html` : opts.route,
    assetRefs: [],
    sourcePath: opts.sourcePath,
  };
}

async function* fromArray<T>(items: readonly T[]): AsyncGenerator<T, void, void> {
  for (const item of items) yield item;
}

async function runRender(
  nodes: ContentNode[],
  config: object = {},
  ctx: StageContext = makeCtx(),
) {
  const out: unknown[] = [];
  const iter = renderStatic.run(fromArray(nodes) as never, config as never, ctx) as AsyncIterable<unknown>;
  for await (const v of iter) out.push(v);
  return out as Array<{
    route: string;
    html: string;
    source: string;
    provenance: OutputProvenance;
    meta: { title: string; description: string | null; canonicalUrl: string | null };
    usedStyle: readonly unknown[];
    usedIslands: readonly unknown[];
    usedAssets: readonly unknown[];
  }>;
}

describe("renderStatic — stage shape", () => {
  it("declares Stream<ContentNode> in / Stream<RenderedPage> out", () => {
    expect(renderStatic.consumes).toEqual(streamOf(Kinds.ContentNode));
    expect(renderStatic.produces).toEqual(streamOf(Kinds.RenderedPage));
  });

  it("declares no capabilities", () => {
    expect(renderStatic.capabilities).toEqual([]);
  });

  it("targets apiVersion 1", () => {
    expect(renderStatic.apiVersion).toBe(1);
  });

  it("has a configSchema covering public metadata", () => {
    expect(renderStatic.configSchema).toMatchObject({
      type: "object",
      properties: {
        siteTitle:     { type: "string" },
        siteUrl:       { type: "string" },
        siteHomeRoute: { type: "string" },
        rssRoute:      { type: "string" },
        atomRoute:     { type: "string" },
      },
    });
  });
});

describe("renderStatic — single-node rendering", () => {
  it("emits one RenderedPage per input ContentNode", async () => {
    const out = await runRender([
      makeNode({ sourcePath: "posts/hello.md", markdown: "# Hello\n\nWorld.\n" }),
      makeNode({ sourcePath: "posts/two.md",   markdown: "# Two\n" }),
    ]);
    expect(out.length).toBe(2);
  });

  it("HTML is a self-contained document", async () => {
    const [page] = await runRender([
      makeNode({ sourcePath: "posts/hello.md", markdown: "# Hello\n\nBody.\n" }),
    ]);
    expect(page!.html).toMatch(/^<!DOCTYPE html>/);
    expect(page!.html).toContain("<h1>Hello</h1>");
    expect(page!.html).toContain("<p>Body.</p>");
    expect(page!.html).toContain("<style>");
  });

  it("uses the canonical ContentNode route", async () => {
    const [page] = await runRender(
      [makeNode({
        sourcePath: "posts/routed.md",
        markdown: "# x\n",
        route: "/canonical/from-router.html",
      })],
    );
    expect(page!.route).toBe("/canonical/from-router.html");
  });

  it("rejects an unrouted ContentNode with an actionable diagnostic", async () => {
    const node = makeNode({
      sourcePath: "posts/unrouted.md",
      markdown: "# x\n",
      route: null,
    });
    await expect(runRender([node])).rejects.toThrow(
      `forme-render-static: ContentNode ${node.identity} (posts/unrouted.md) has no route; add forme-router upstream`,
    );
  });

  it("title from frontmatter wins over h1", async () => {
    const [page] = await runRender([
      makeNode({
        sourcePath: "posts/p.md",
        markdown: "# From H1\n",
        frontmatter: { title: "From Frontmatter" },
      }),
    ]);
    expect(page!.meta.title).toBe("From Frontmatter");
    expect(page!.html).toContain("<title>From Frontmatter</title>");
  });

  it("title falls back to first h1 when frontmatter is absent", async () => {
    const [page] = await runRender([
      makeNode({ sourcePath: "posts/p.md", markdown: "# From H1\n\nbody\n" }),
    ]);
    expect(page!.meta.title).toBe("From H1");
  });

  it("title falls back to slug when no h1 either", async () => {
    const [page] = await runRender([
      makeNode({ sourcePath: "posts/no-heading.md", markdown: "body only\n" }),
    ]);
    expect(page!.meta.title).toBe("no-heading");
  });

  it("carries the input logical and revision IDs through provenance", async () => {
    const node = makeNode({ sourcePath: "p.md", markdown: "# x\n" });
    const [page] = await runRender([node]);
    expect(page!.source).toBe(node.identity);
    expect(page!.provenance.contributors).toEqual([{
      identity: node.identity,
      revision: node.revision,
    }]);
    expect(page!.provenance.revision).toMatch(/^blake2b:[0-9a-f]{64}$/);
  });

  it("usedStyle / usedIslands / usedAssets are all empty in v0", async () => {
    const [page] = await runRender([
      makeNode({ sourcePath: "p.md", markdown: "# x\n" }),
    ]);
    expect(page!.usedStyle).toEqual([]);
    expect(page!.usedIslands).toEqual([]);
    expect(page!.usedAssets).toEqual([]);
  });

  it("leaves optional metadata null without public-site config", async () => {
    const [page] = await runRender([
      makeNode({ sourcePath: "p.md", markdown: "# x\n" }),
    ]);
    expect(page!.meta.description).toBeNull();
  });

  it("emits project-page-safe canonical, description, and feed discovery metadata", async () => {
    const [page] = await runRender([
      makeNode({
        sourcePath: "p.md",
        markdown: "# Post\n",
        route: "/blog/post.html",
        frontmatter: { excerpt: 'A useful "post".' },
      }),
    ], {
      siteTitle: "Coding Adventures",
      siteUrl: "https://example.com/coding-adventures/",
      siteHomeRoute: "/blog/index.html",
      rssRoute: "/blog/rss.xml",
      atomRoute: "/blog/atom.xml",
    });

    expect(page!.meta.description).toBe('A useful "post".');
    expect(page!.meta.canonicalUrl)
      .toBe("https://example.com/coding-adventures/blog/post.html");
    expect(page!.html).toContain('<meta name="description" content="A useful &quot;post&quot;.">');
    expect(page!.html).toContain('<link rel="canonical" href="https://example.com/coding-adventures/blog/post.html">');
    expect(page!.html).toContain('type="application/rss+xml"');
    expect(page!.html).toContain('href="https://example.com/coding-adventures/blog/atom.xml"');
    expect(page!.html).toContain('<header><a href="https://example.com/coding-adventures/blog/index.html">');
  });
});

describe("renderStatic — siteTitle config", () => {
  it("renders a header when siteTitle is provided", async () => {
    const [page] = await runRender(
      [makeNode({ sourcePath: "p.md", markdown: "# x\n" })],
      { siteTitle: "My Blog" },
    );
    expect(page!.html).toContain(`<header><a href="/">My Blog</a></header>`);
  });

  it("omits header when siteTitle is empty or unset", async () => {
    const [page] = await runRender([
      makeNode({ sourcePath: "p.md", markdown: "# x\n" }),
    ]);
    expect(page!.html).not.toContain("<header>");
  });
});

describe("renderStatic — markdown coverage", () => {
  it("renders headings, paragraphs, lists, code, blockquote", async () => {
    const md = [
      "# Heading 1",
      "",
      "Paragraph with *emphasis* and **strong**.",
      "",
      "- item 1",
      "- item 2",
      "",
      "```ts",
      "const x: number = 1;",
      "```",
      "",
      "> A blockquote.",
      "",
    ].join("\n");
    const [page] = await runRender([makeNode({ sourcePath: "p.md", markdown: md })]);
    expect(page!.html).toContain("<h1>Heading 1</h1>");
    expect(page!.html).toContain("<em>emphasis</em>");
    expect(page!.html).toContain("<strong>strong</strong>");
    expect(page!.html).toContain("<li>item 1</li>");
    expect(page!.html).toMatch(/<code class="language-ts">const x: number = 1;\n<\/code>/);
    expect(page!.html).toContain("<blockquote>");
  });
});

describe("renderStatic — cancellation", () => {
  it("throws when cancellation is requested mid-stream", async () => {
    const cs = createCancellationTokenSource();
    const ctx = makeCtx({ cancellation: cs.token });
    cs.cancel("test");
    const nodes = [makeNode({ sourcePath: "p.md", markdown: "# x\n" })];
    await expect(runRender(nodes, {}, ctx)).rejects.toThrow();
  });
});

describe("renderStatic — empty stream", () => {
  it("emits nothing when given no input", async () => {
    const out = await runRender([]);
    expect(out).toEqual([]);
  });
});
