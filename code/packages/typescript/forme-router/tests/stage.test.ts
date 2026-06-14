/**
 * stage.test.ts — exercise the router stage end-to-end against a
 * fixture stream of ContentNodes.  No orchestrator wrapping; we
 * invoke the stage's `run` method directly and consume its
 * AsyncIterable result.
 */

import { describe, it, expect } from "vitest";
import { createCancellationTokenSource, silentLogger } from "@coding-adventures/forme-stage";
import type { ContentNode, JsonValue } from "@coding-adventures/forme-types";
import router from "../src/index.js";

// ─── Helpers ────────────────────────────────────────────────────────

function node(opts: {
  sourcePath: string;
  frontmatter?: Record<string, JsonValue>;
  route?: string | null;
}): ContentNode {
  return {
    identity: "01952c0d-7e63-7000-8000-000000000000" as never,
    revision: "blake2b:00" as never,
    document: { type: "document", children: [] } as never,
    frontmatter: opts.frontmatter ?? {},
    route: opts.route ?? null,
    assetRefs: [],
    sourcePath: opts.sourcePath,
  };
}

async function* nodeStream(nodes: ContentNode[]): AsyncIterable<ContentNode> {
  for (const n of nodes) yield n;
}

function buildCtx(): Parameters<typeof router["run"]>[2] {
  const cs = createCancellationTokenSource();
  return {
    logger: silentLogger(),
    cancellation: cs.token,
  } as never;  // we only use logger + cancellation in this stage
}

async function collect<T>(it: AsyncIterable<T>): Promise<T[]> {
  const out: T[] = [];
  for await (const v of it) out.push(v);
  return out;
}

// ─── Tests ──────────────────────────────────────────────────────────

describe("router stage — happy paths", () => {
  it("derives route from sourcePath when frontmatter has no slug", async () => {
    const input = nodeStream([node({ sourcePath: "posts/hello.md" })]);
    const out = await collect(
      (router.run as Function)(input, {}, buildCtx()),
    );
    expect(out).toHaveLength(1);
    expect((out[0] as ContentNode).route).toBe("/blog/hello.html");
  });

  it("derives route from frontmatter.slug when present", async () => {
    const input = nodeStream([node({
      sourcePath: "posts/whatever.md",
      frontmatter: { slug: "intro" },
    })]);
    const out = await collect((router.run as Function)(input, {}, buildCtx()));
    expect((out[0] as ContentNode).route).toBe("/blog/intro.html");
  });

  it("preserves identity and revision unchanged", async () => {
    const original = node({ sourcePath: "posts/x.md" });
    const out = await collect((router.run as Function)(
      nodeStream([original]),
      {},
      buildCtx(),
    ));
    expect((out[0] as ContentNode).identity).toBe(original.identity);
    expect((out[0] as ContentNode).revision).toBe(original.revision);
  });

  it("preserves frontmatter and assetRefs", async () => {
    const original = node({
      sourcePath: "posts/x.md",
      frontmatter: { title: "Hi", date: "2026-05-16" },
    });
    const out = await collect((router.run as Function)(
      nodeStream([original]),
      {},
      buildCtx(),
    ));
    const result = out[0] as ContentNode;
    expect(result.frontmatter).toEqual(original.frontmatter);
    expect(result.assetRefs).toBe(original.assetRefs);
  });

  it("honours custom routeTemplate", async () => {
    const input = nodeStream([node({ sourcePath: "x.md" })]);
    const out = await collect((router.run as Function)(
      input,
      { routeTemplate: "/posts/{slug}/" },
      buildCtx(),
    ));
    expect((out[0] as ContentNode).route).toBe("/posts/x/");
  });

  it("honours custom slugField", async () => {
    const input = nodeStream([node({
      sourcePath: "posts/whatever.md",
      frontmatter: { permalink: "fancy-url" },
    })]);
    const out = await collect((router.run as Function)(
      input,
      { slugField: "permalink" },
      buildCtx(),
    ));
    expect((out[0] as ContentNode).route).toBe("/blog/fancy-url.html");
  });

  it("emits one output per input", async () => {
    const inputs = nodeStream([
      node({ sourcePath: "a.md" }),
      node({ sourcePath: "b.md" }),
      node({ sourcePath: "c.md" }),
    ]);
    const out = await collect((router.run as Function)(inputs, {}, buildCtx()));
    expect(out).toHaveLength(3);
    expect((out[0] as ContentNode).route).toBe("/blog/a.html");
    expect((out[1] as ContentNode).route).toBe("/blog/b.html");
    expect((out[2] as ContentNode).route).toBe("/blog/c.html");
  });

  it("preserves source order", async () => {
    const paths = ["z.md", "a.md", "m.md"];
    const inputs = nodeStream(paths.map((p) => node({ sourcePath: p })));
    const out = await collect((router.run as Function)(inputs, {}, buildCtx()));
    expect((out as ContentNode[]).map((n) => n.route))
      .toEqual(["/blog/z.html", "/blog/a.html", "/blog/m.html"]);
  });

  it("empty input → empty output", async () => {
    const out = await collect((router.run as Function)(
      nodeStream([]),
      {},
      buildCtx(),
    ));
    expect(out).toEqual([]);
  });
});

describe("router stage — slug fallback edge cases", () => {
  it("ignores non-string slug frontmatter", async () => {
    const input = nodeStream([node({
      sourcePath: "posts/from-path.md",
      frontmatter: { slug: 42 as never },
    })]);
    const out = await collect((router.run as Function)(input, {}, buildCtx()));
    // Falls through to sourcePath-derived slug.
    expect((out[0] as ContentNode).route).toBe("/blog/from-path.html");
  });

  it("ignores empty-string slug frontmatter", async () => {
    const input = nodeStream([node({
      sourcePath: "posts/from-path.md",
      frontmatter: { slug: "" },
    })]);
    const out = await collect((router.run as Function)(input, {}, buildCtx()));
    expect((out[0] as ContentNode).route).toBe("/blog/from-path.html");
  });

  it("ignores missing slug field", async () => {
    const input = nodeStream([node({
      sourcePath: "posts/x.md",
      frontmatter: {},
    })]);
    const out = await collect((router.run as Function)(input, {}, buildCtx()));
    expect((out[0] as ContentNode).route).toBe("/blog/x.html");
  });
});

describe("router stage — cancellation", () => {
  it("throws when cancellation is signalled before the first item", async () => {
    const cs = createCancellationTokenSource();
    cs.cancel("test");
    const ctx = { logger: silentLogger(), cancellation: cs.token } as never;
    const out = (router.run as Function)(
      nodeStream([node({ sourcePath: "a.md" })]),
      {},
      ctx,
    );
    await expect(collect(out)).rejects.toThrow();
  });

  it("aborts mid-stream when cancellation fires", async () => {
    const cs = createCancellationTokenSource();
    const ctx = { logger: silentLogger(), cancellation: cs.token } as never;
    const out = (router.run as Function)(
      nodeStream([
        node({ sourcePath: "a.md" }),
        node({ sourcePath: "b.md" }),
        node({ sourcePath: "c.md" }),
      ]),
      {},
      ctx,
    );
    const collected: ContentNode[] = [];
    try {
      for await (const v of out as AsyncIterable<ContentNode>) {
        collected.push(v);
        // Cancel after first emission
        if (collected.length === 1) cs.cancel("midway");
      }
      throw new Error("should have thrown");
    } catch (err) {
      // After cancel(), the next throwIfCancelled() throws.  We
      // got at most 1 emission before cancellation kicked in.
      // CancellationError carries the reason string ("midway") as its
      // message — we just verify SOMETHING was thrown and that fewer
      // than the full set of items were emitted.
      expect(collected.length).toBeLessThan(3);
      expect(err).toBeDefined();
    }
  });
});

describe("router stage — metadata", () => {
  it("declares the expected shape", () => {
    expect(router.name).toBe("@coding-adventures/forme-router");
    expect(router.version).toBe("0.1.0");
    expect(router.apiVersion).toBe(1);
    expect(router.consumes.name).toBe("Stream");
    expect(router.produces.name).toBe("Stream");
    expect(router.capabilities).toEqual([]);
    expect(router.configSchema).not.toBeNull();
  });
});
