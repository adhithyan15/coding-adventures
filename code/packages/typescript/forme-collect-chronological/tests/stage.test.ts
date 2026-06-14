/**
 * stage.test.ts — collector integration tests.
 *
 * Verifies the stage value-shape and the full sort + route assignment
 * pipeline with a variety of inputs:
 *   - empty stream
 *   - single post
 *   - multiple posts (descending sort)
 *   - posts without dates land last
 *   - identical dates → sourcePath tiebreak
 *   - explicit slug frontmatter wins over derived slug
 *   - custom dateField / slugField / routeTemplate / name
 *   - missing-date warning is emitted
 */

import { describe, it, expect } from "vitest";
import { Kinds, streamOf, type ContentNode } from "@coding-adventures/forme-types";
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
  type Logger,
} from "@coding-adventures/forme-stage";
import collectChronological from "../src/index.js";

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

let nodeSeq = 0;
function makeNode(opts: {
  sourcePath: string;
  date?: string;
  slug?: string;
  title?: string;
  excerpt?: string;
  extraFrontmatter?: Record<string, string>;
}): ContentNode {
  nodeSeq++;
  const id = `00000000-0000-7000-8000-${String(nodeSeq).padStart(12, "0")}` as ContentNode["identity"];
  const fm: Record<string, string> = { ...(opts.extraFrontmatter ?? {}) };
  if (opts.date  !== undefined) fm.date  = opts.date;
  if (opts.slug  !== undefined) fm.slug  = opts.slug;
  if (opts.title !== undefined) fm.title = opts.title;
  if (opts.excerpt !== undefined) fm.excerpt = opts.excerpt;
  return {
    identity: id,
    revision: ("blake2b:" + "0".repeat(64)) as ContentNode["revision"],
    document: { type: "document", children: [] } as unknown as ContentNode["document"],
    frontmatter: fm,
    route: null,
    assetRefs: [],
    sourcePath: opts.sourcePath,
  };
}

async function* fromArray<T>(items: readonly T[]): AsyncGenerator<T, void, void> {
  for (const item of items) yield item;
}

async function runCollect(nodes: ContentNode[], config: object = {}, ctx: StageContext = makeCtx()) {
  const out = await collectChronological.run(
    fromArray(nodes) as never,
    config as never,
    ctx,
  );
  return out as unknown as {
    name: string;
    entries: Array<{
      identity: string;
      revision: string;
      route: string;
      orderKey: { kind: string; value: string };
      overlay: Record<string, unknown>;
    }>;
    discriminant: string;
    meta: Record<string, unknown>;
  };
}

describe("collectChronological — stage shape", () => {
  it("declares Stream<ContentNode> in / Collection out", () => {
    expect(collectChronological.consumes).toEqual(streamOf(Kinds.ContentNode));
    expect(collectChronological.produces).toEqual(Kinds.Collection);
  });

  it("declares no capabilities (pure transform)", () => {
    expect(collectChronological.capabilities).toEqual([]);
  });

  it("targets kernel apiVersion 1", () => {
    expect(collectChronological.apiVersion).toBe(1);
  });

  it("has a configSchema with all 4 optional fields", () => {
    expect(collectChronological.configSchema).toMatchObject({
      type: "object",
      properties: {
        name:          { type: "string" },
        dateField:     { type: "string" },
        slugField:     { type: "string" },
        routeTemplate: { type: "string" },
      },
    });
  });
});

describe("collectChronological — base behaviour", () => {
  it("empty input → empty Collection.entries", async () => {
    const c = await runCollect([]);
    expect(c.entries).toEqual([]);
    expect(c.name).toBe("posts");
    expect(c.discriminant).toBe("chronological");
  });

  it("single post → one entry with date, slug, route filled", async () => {
    const c = await runCollect([
      makeNode({ sourcePath: "posts/hello.md", date: "2026-05-15", title: "Hello" }),
    ]);
    expect(c.entries.length).toBe(1);
    const e = c.entries[0]!;
    expect(e.route).toBe("/blog/hello.html");
    expect(e.orderKey).toEqual({ kind: "date", value: "2026-05-15" });
    expect(e.overlay.title).toBe("Hello");
    expect(e.overlay.slug).toBe("hello");
    expect(e.overlay.date).toBe("2026-05-15");
  });

  it("multiple posts sort descending by date (newest first)", async () => {
    const c = await runCollect([
      makeNode({ sourcePath: "posts/a.md", date: "2026-01-10" }),
      makeNode({ sourcePath: "posts/b.md", date: "2026-05-15" }),
      makeNode({ sourcePath: "posts/c.md", date: "2026-03-01" }),
    ]);
    expect(c.entries.map((e) => e.overlay.date)).toEqual([
      "2026-05-15", "2026-03-01", "2026-01-10",
    ]);
  });
});

describe("collectChronological — edge cases", () => {
  it("posts without a date land last (after dated posts)", async () => {
    const c = await runCollect([
      makeNode({ sourcePath: "posts/has-date.md", date: "2026-05-15" }),
      makeNode({ sourcePath: "posts/no-date.md" }),
    ]);
    expect(c.entries.map((e) => e.overlay.slug)).toEqual([
      "has-date", "no-date",
    ]);
    expect(c.entries[1]!.orderKey.value).toBe("0000-01-01");
  });

  it("ties on date → sourcePath ascending tiebreak", async () => {
    const c = await runCollect([
      makeNode({ sourcePath: "posts/zzz.md", date: "2026-05-15" }),
      makeNode({ sourcePath: "posts/aaa.md", date: "2026-05-15" }),
      makeNode({ sourcePath: "posts/mmm.md", date: "2026-05-15" }),
    ]);
    expect(c.entries.map((e) => e.overlay.slug)).toEqual(["aaa", "mmm", "zzz"]);
  });

  it("emits a warning via ctx.logger.warn for each dateless post", async () => {
    const warnings: string[] = [];
    const captureLogger: Logger = {
      trace: () => {}, debug: () => {}, info: () => {},
      warn:  (msg: string) => warnings.push(msg),
      error: () => {},
      child: function () { return this; },
    };
    const ctx = makeCtx({ logger: captureLogger });
    await runCollect(
      [
        makeNode({ sourcePath: "posts/missing-1.md" }),
        makeNode({ sourcePath: "posts/missing-2.md" }),
        makeNode({ sourcePath: "posts/ok.md", date: "2026-05-15" }),
      ],
      {},
      ctx,
    );
    expect(warnings.length).toBe(2);
    expect(warnings[0]).toMatch(/missing date/i);
  });

  it("explicit slug frontmatter wins over the derived slug", async () => {
    const c = await runCollect([
      makeNode({ sourcePath: "posts/raw-name.md", date: "2026-01-01", slug: "custom-slug" }),
    ]);
    expect(c.entries[0]!.overlay.slug).toBe("custom-slug");
    expect(c.entries[0]!.route).toBe("/blog/custom-slug.html");
  });

  it("empty title frontmatter falls back to slug", async () => {
    // No title at all in this case — same fallback path.
    const c = await runCollect([
      makeNode({ sourcePath: "posts/no-title.md", date: "2026-01-01" }),
    ]);
    expect(c.entries[0]!.overlay.title).toBe("no-title");
  });

  it("excerpt is included on the overlay when present", async () => {
    const c = await runCollect([
      makeNode({ sourcePath: "posts/p.md", date: "2026-01-01", excerpt: "Hello, world." }),
    ]);
    expect(c.entries[0]!.overlay.excerpt).toBe("Hello, world.");
  });

  it("excerpt is omitted from the overlay when absent", async () => {
    const c = await runCollect([
      makeNode({ sourcePath: "posts/p.md", date: "2026-01-01" }),
    ]);
    expect("excerpt" in c.entries[0]!.overlay).toBe(false);
  });
});

describe("collectChronological — config customisation", () => {
  it("custom collection name", async () => {
    const c = await runCollect([], { name: "essays" });
    expect(c.name).toBe("essays");
  });

  it("custom dateField pulls from a different frontmatter key", async () => {
    const c = await runCollect(
      [
        makeNode({ sourcePath: "p1.md", extraFrontmatter: { published: "2026-05-15" } }),
        makeNode({ sourcePath: "p2.md", extraFrontmatter: { published: "2026-01-01" } }),
      ],
      { dateField: "published" },
    );
    expect(c.entries.map((e) => e.orderKey.value)).toEqual(["2026-05-15", "2026-01-01"]);
  });

  it("custom slugField + custom routeTemplate", async () => {
    const c = await runCollect(
      [makeNode({ sourcePath: "posts/p.md", date: "2026-01-01", extraFrontmatter: { handle: "my-handle" } })],
      { slugField: "handle", routeTemplate: "/essays/{slug}/index.html" },
    );
    expect(c.entries[0]!.route).toBe("/essays/my-handle/index.html");
    expect(c.entries[0]!.overlay.slug).toBe("my-handle");
  });

  it("undefined config behaves like {}", async () => {
    const c = await runCollect(
      [makeNode({ sourcePath: "posts/p.md", date: "2026-01-01" })],
      undefined as unknown as object,
    );
    expect(c.entries[0]!.route).toBe("/blog/p.html");
  });
});

describe("collectChronological — entry shape", () => {
  it("entry carries identity + revision pass-through from the node", async () => {
    const node = makeNode({ sourcePath: "p.md", date: "2026-01-01" });
    const c = await runCollect([node]);
    expect(c.entries[0]!.identity).toBe(node.identity);
    expect(c.entries[0]!.revision).toBe(node.revision);
  });

  it("collection discriminant is 'chronological'", async () => {
    const c = await runCollect([]);
    expect(c.discriminant).toBe("chronological");
  });
});
