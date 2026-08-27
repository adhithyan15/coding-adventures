import { describe, expect, it } from "vitest";
import type { Collection, LogicalId, RevisionId } from "@coding-adventures/forme-types";
import {
  createCancellationTokenSource,
  deniedEnvApi,
  deniedFilesystemApi,
  deniedNetworkApi,
  deniedShellApi,
  deniedStorageApi,
  inMemoryCache,
  inMemoryEventBus,
  noOpTelemetryEmitter,
  silentLogger,
  systemClock,
} from "@coding-adventures/forme-stage";
import blogSurface from "./surface-stage.ts";

const config = {
  siteTitle: "Coding Adventures",
  siteDescription: "Small systems, built from first principles.",
  siteUrl: "https://example.com/coding-adventures",
  indexRoute: "/blog/index.html",
  rssRoute: "/blog/rss.xml",
  atomRoute: "/blog/atom.xml",
  sitemapRoute: "/blog/sitemap.xml",
};

function collection(): Collection {
  return {
    name: "posts",
    discriminant: "chronological",
    meta: {},
    entries: [
      entry("newer", "/blog/newer.html", "2026-05-15", "Newer", "Newest post."),
      entry("older", "/blog/older.html", "2026-05-08", "Older", "Older post."),
    ],
  };
}

function entry(id: string, route: string, date: string, title: string, excerpt: string) {
  return {
    identity: id as LogicalId,
    revision: `blake2b:${"0".repeat(64)}` as RevisionId,
    route,
    orderKey: { kind: "date" as const, value: date },
    overlay: { title, excerpt, date },
  };
}

function context() {
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
  };
}

async function render() {
  const pages = [];
  const output = blogSurface.run(collection() as never, config as never, context()) as AsyncIterable<any>;
  for await (const page of output) pages.push(page);
  return pages;
}

describe("blog surface", () => {
  it("emits the four collection-derived routes", async () => {
    expect((await render()).map((page) => page.route)).toEqual([
      "/blog/index.html",
      "/blog/rss.xml",
      "/blog/atom.xml",
      "/blog/sitemap.xml",
    ]);
  });

  it("uses public project-page URLs and keeps newest-first ordering", async () => {
    const [index, rss, atom, sitemap] = await render();
    const newer = "https://example.com/coding-adventures/blog/newer.html";
    const older = "https://example.com/coding-adventures/blog/older.html";

    for (const output of [index.html, rss.html, atom.html, sitemap.html]) {
      expect(output.indexOf(newer)).toBeGreaterThan(-1);
      expect(output.indexOf(older)).toBeGreaterThan(output.indexOf(newer));
    }
    expect(index.html).toContain('rel="canonical" href="https://example.com/coding-adventures/blog/index.html"');
    expect(index.html).toContain('type="application/rss+xml"');
    expect(index.html).toContain('type="application/atom+xml"');
    expect(rss.html).toContain("<lastBuildDate>Fri, 15 May 2026 00:00:00 +0000</lastBuildDate>");
    expect(atom.html).toContain("<updated>2026-05-15T00:00:00Z</updated>");
  });
});
