/**
 * index-renderer.test.ts — end-to-end renderIndexPage behaviour.
 */

import { describe, it, expect } from "vitest";
import { renderIndexPage, type IndexItem } from "../src/index.js";

const ITEMS: IndexItem[] = [
  { id: "1", title: "First Post",  url: "/posts/first",  pubDate: "2026-01-15T00:00:00Z", category: "Code", summary: "Hello" },
  { id: "2", title: "Second Post", url: "/posts/second", pubDate: "2026-02-20T00:00:00Z", category: "Life", summary: "World" },
  { id: "3", title: "Third Post",  url: "/posts/third",  pubDate: "2025-12-01T00:00:00Z", category: "Code", summary: "!" },
];

describe("renderIndexPage — flat list (groupBy: none default)", () => {
  it("renders <ul class=forme-index> with one <li> per item", () => {
    const html = renderIndexPage(ITEMS);
    expect(html.startsWith(`<ul class="forme-index">`)).toBe(true);
    expect(html.endsWith(`</ul>`)).toBe(true);
    const liCount = (html.match(/<li>/g) ?? []).length;
    expect(liCount).toBe(3);
  });

  it("each <li> includes <a href=...>title</a>", () => {
    const html = renderIndexPage(ITEMS);
    expect(html).toContain(`<a href="/posts/first">First Post</a>`);
    expect(html).toContain(`<a href="/posts/second">Second Post</a>`);
  });

  it("items sorted pubDate-desc by default (newest first)", () => {
    const html = renderIndexPage(ITEMS);
    const firstIdx  = html.indexOf("First Post");
    const secondIdx = html.indexOf("Second Post");
    const thirdIdx  = html.indexOf("Third Post");
    // Second (newest) → First → Third.
    expect(secondIdx).toBeLessThan(firstIdx);
    expect(firstIdx).toBeLessThan(thirdIdx);
  });

  it("empty items → empty <ul>", () => {
    expect(renderIndexPage([])).toBe(`<ul class="forme-index"></ul>`);
  });
});

describe("renderIndexPage — sortBy modes", () => {
  it("pubDate-asc reverses order", () => {
    const html = renderIndexPage(ITEMS, { sortBy: "pubDate-asc" });
    const a = html.indexOf("Third Post");
    const b = html.indexOf("Second Post");
    expect(a).toBeLessThan(b);
  });

  it("title-asc sorts alphabetically by title", () => {
    const html = renderIndexPage(ITEMS, { sortBy: "title-asc" });
    const f = html.indexOf("First Post");
    const s = html.indexOf("Second Post");
    const t = html.indexOf("Third Post");
    expect(f).toBeLessThan(s);
    expect(s).toBeLessThan(t);
  });
});

describe("renderIndexPage — grouping", () => {
  it("groupBy: category emits <section><h2>...</h2><ul>...</ul></section>", () => {
    const html = renderIndexPage(ITEMS, { groupBy: "category" });
    expect(html).toContain(`<section class="forme-index-group">`);
    expect(html).toContain(`<h2>Code</h2>`);
    expect(html).toContain(`<h2>Life</h2>`);
  });

  it("groupBy: year groups by 4-digit year", () => {
    const html = renderIndexPage(ITEMS, { groupBy: "year" });
    expect(html).toContain(`<h2>2026</h2>`);
    expect(html).toContain(`<h2>2025</h2>`);
  });

  it("groupBy: month groups by YYYY-MM", () => {
    const html = renderIndexPage(ITEMS, { groupBy: "month" });
    expect(html).toContain(`<h2>2026-02</h2>`);
    expect(html).toContain(`<h2>2026-01</h2>`);
    expect(html).toContain(`<h2>2025-12</h2>`);
  });
});

describe("renderIndexPage — display toggles", () => {
  it("showDate emits <time datetime=iso>iso</time>", () => {
    const html = renderIndexPage(ITEMS, { showDate: true });
    expect(html).toContain(`<time datetime="2026-01-15T00:00:00Z">2026-01-15T00:00:00Z</time>`);
  });

  it("dateFormat callback rewrites the visible text but datetime attr stays raw ISO", () => {
    const html = renderIndexPage([ITEMS[0]!], {
      showDate: true,
      dateFormat: (iso) => `pretty-${iso.slice(0, 10)}`,
    });
    expect(html).toContain(`<time datetime="2026-01-15T00:00:00Z">pretty-2026-01-15</time>`);
  });

  it("showSummary emits <p class=summary>...</p>", () => {
    const html = renderIndexPage(ITEMS, { showSummary: true });
    expect(html).toContain(`<p class="summary">Hello</p>`);
  });

  it("showSummary skips items with no summary (no empty <p>)", () => {
    const noSummary: IndexItem[] = [
      { id: "x", title: "X", url: "/x" },
    ];
    const html = renderIndexPage(noSummary, { showSummary: true });
    expect(html).not.toContain(`<p class="summary"`);
  });

  it("showDate skips items with no pubDate (no empty <time>)", () => {
    const noDate: IndexItem[] = [
      { id: "x", title: "X", url: "/x" },
    ];
    const html = renderIndexPage(noDate, { showDate: true });
    expect(html).not.toContain(`<time`);
  });
});

describe("renderIndexPage — HTML escaping", () => {
  it("escapes title", () => {
    const evil: IndexItem[] = [
      { id: "x", title: `<script>alert("xss")</script>`, url: "/x" },
    ];
    const html = renderIndexPage(evil);
    expect(html).toContain(`&lt;script&gt;alert(&quot;xss&quot;)&lt;/script&gt;`);
    expect(html).not.toContain(`<script>alert`);
  });

  it("escapes summary", () => {
    const evil: IndexItem[] = [
      { id: "x", title: "T", url: "/x", summary: `<img src=x onerror=alert(1)>` },
    ];
    const html = renderIndexPage(evil, { showSummary: true });
    expect(html).toContain(`&lt;img src=x onerror=alert(1)&gt;`);
  });

  it("escapes URL when interpolated into href attribute", () => {
    const evil: IndexItem[] = [
      { id: "x", title: "T", url: `https://example.com/?a=1&b=2"` },
    ];
    const html = renderIndexPage(evil);
    expect(html).toContain(`href="https://example.com/?a=1&amp;b=2&quot;"`);
  });

  it("escapes attacker-controlled category in <h2>", () => {
    const evil: IndexItem[] = [
      { id: "x", title: "T", url: "/x", category: `<script>alert(1)</script>` },
    ];
    const html = renderIndexPage(evil, { groupBy: "category" });
    expect(html).toContain(`<h2>&lt;script&gt;alert(1)&lt;/script&gt;</h2>`);
    expect(html).not.toContain(`<h2><script>`);
  });
});

describe("renderIndexPage — URL validation", () => {
  it("rejects javascript: in item.url", () => {
    const evil: IndexItem[] = [
      { id: "x", title: "T", url: "javascript:alert(1)" },
    ];
    expect(() => renderIndexPage(evil)).toThrow(/absolute http\(s\) or root-relative/);
  });

  it("rejects data: in item.url", () => {
    const evil: IndexItem[] = [
      { id: "x", title: "T", url: "data:text/html,<script>" },
    ];
    expect(() => renderIndexPage(evil)).toThrow(/absolute http\(s\) or root-relative/);
  });

  it("rejects protocol-relative //host", () => {
    const evil: IndexItem[] = [
      { id: "x", title: "T", url: "//example.com" },
    ];
    expect(() => renderIndexPage(evil)).toThrow(/absolute http\(s\) or root-relative/);
  });

  it("validates URLs BEFORE rendering (no partial output)", () => {
    const mixed: IndexItem[] = [
      { id: "good", title: "G", url: "/g" },
      { id: "bad",  title: "B", url: "javascript:alert(1)" },
    ];
    try {
      renderIndexPage(mixed);
      expect.fail("expected throw");
    } catch (e) {
      expect((e as Error).message).toMatch(/javascript:/);
    }
  });
});

describe("renderIndexPage — reproducibility", () => {
  it("same input → byte-identical output", () => {
    expect(renderIndexPage(ITEMS)).toBe(renderIndexPage(ITEMS));
  });

  it("reshuffled items → same output (deterministic sort + id tiebreaker)", () => {
    const a = renderIndexPage(ITEMS);
    const b = renderIndexPage([ITEMS[2]!, ITEMS[0]!, ITEMS[1]!]);
    expect(a).toBe(b);
  });
});
