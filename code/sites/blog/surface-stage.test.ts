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

const NEWER_ID = "01952c0d-7e63-7000-8000-000000000002" as LogicalId;
const OLDER_ID = "01952c0d-7e63-7000-8000-000000000001" as LogicalId;
const NEWER_REV = `blake2b:${"2".repeat(64)}` as RevisionId;
const OLDER_REV = `blake2b:${"1".repeat(64)}` as RevisionId;

function collection(): Collection {
  return {
    name: "posts",
    discriminant: "chronological",
    meta: {},
    entries: [
      entry(NEWER_ID, NEWER_REV, "/blog/newer.html", "2026-05-15", "Newer", "Newest post."),
      entry(OLDER_ID, OLDER_REV, "/blog/older.html", "2026-05-08", "Older", "Older post."),
    ],
  };
}

function entry(
  identity: LogicalId,
  revision: RevisionId,
  route: string,
  date: string,
  title: string,
  excerpt: string,
) {
  return {
    identity,
    revision,
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

async function render(input: Collection = collection()) {
  const pages = [];
  const output = blogSurface.run(input as never, config as never, context()) as AsyncIterable<any>;
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

  it("attributes every aggregate artifact to all exact contributors", async () => {
    const pages = await render();
    for (const page of pages) {
      expect(page).not.toHaveProperty("source");
      expect(page.provenance.contributors).toEqual([
        { identity: OLDER_ID, revision: OLDER_REV },
        { identity: NEWER_ID, revision: NEWER_REV },
      ]);
      expect(page.provenance.revision).toMatch(/^blake2b:[0-9a-f]{64}$/);
    }
    expect(new Set(pages.map((page) => page.provenance.revision)).size).toBe(1);
  });

  it("gives empty collection outputs deterministic empty provenance", async () => {
    const empty: Collection = { ...collection(), entries: [] };
    const pages = await render(empty);
    expect(pages).toHaveLength(4);
    expect(pages.every((page) => page.provenance.contributors.length === 0)).toBe(true);
    expect(new Set(pages.map((page) => page.provenance.revision)).size).toBe(1);
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
