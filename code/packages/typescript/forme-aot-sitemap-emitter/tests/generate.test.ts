/**
 * generate.test.ts — end-to-end generateSitemap.
 */

import { describe, it, expect } from "vitest";
import { generateSitemap, type SitemapEntry } from "../src/index.js";

describe("generateSitemap — XML envelope", () => {
  it("starts with <?xml version=...?>", () => {
    const xml = generateSitemap([{ url: "/" }], "https://example.com");
    expect(xml.startsWith(`<?xml version="1.0" encoding="UTF-8"?>`)).toBe(true);
  });

  it("contains urlset with sitemap.org namespace", () => {
    const xml = generateSitemap([{ url: "/" }], "https://example.com");
    expect(xml).toContain(`<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">`);
    expect(xml).toContain(`</urlset>`);
  });

  it("empty entries → valid empty sitemap", () => {
    const xml = generateSitemap([], "https://example.com");
    expect(xml).toContain(`<urlset`);
    expect(xml).toContain(`</urlset>`);
    expect(xml).not.toContain(`<url>`);
  });
});

describe("generateSitemap — entry rendering", () => {
  it("minimal entry: only <loc>", () => {
    const xml = generateSitemap([{ url: "/" }], "https://example.com");
    expect(xml).toContain(`<loc>https://example.com/</loc>`);
    expect(xml).not.toContain(`<lastmod>`);
    expect(xml).not.toContain(`<changefreq>`);
    expect(xml).not.toContain(`<priority>`);
  });

  it("full entry with all optional fields", () => {
    const xml = generateSitemap([{
      url: "/about", lastmod: "2026-05-19", changefreq: "monthly", priority: 0.8,
    }], "https://example.com");
    expect(xml).toContain(`<loc>https://example.com/about</loc>`);
    expect(xml).toContain(`<lastmod>2026-05-19</lastmod>`);
    expect(xml).toContain(`<changefreq>monthly</changefreq>`);
    expect(xml).toContain(`<priority>0.8</priority>`);
  });

  it("child order: loc → lastmod → changefreq → priority", () => {
    const xml = generateSitemap([{
      url: "/", lastmod: "2026-05-19", changefreq: "daily", priority: 1.0,
    }], "https://example.com");
    const locIdx = xml.indexOf("<loc>");
    const modIdx = xml.indexOf("<lastmod>");
    const freqIdx = xml.indexOf("<changefreq>");
    const priIdx = xml.indexOf("<priority>");
    expect(locIdx).toBeLessThan(modIdx);
    expect(modIdx).toBeLessThan(freqIdx);
    expect(freqIdx).toBeLessThan(priIdx);
  });

  it("absolute http(s) URL passes through verbatim", () => {
    const xml = generateSitemap([
      { url: "https://other.example/x" },
    ], "https://base.example");
    expect(xml).toContain(`<loc>https://other.example/x</loc>`);
    expect(xml).not.toContain(`<loc>https://base.example/https`);
  });

  it("mixed absolute + relative entries", () => {
    const xml = generateSitemap([
      { url: "/" },
      { url: "https://other.example/x" },
      { url: "/about" },
    ], "https://base.example");
    expect(xml).toContain(`<loc>https://base.example/</loc>`);
    expect(xml).toContain(`<loc>https://other.example/x</loc>`);
    expect(xml).toContain(`<loc>https://base.example/about</loc>`);
  });

  it("baseUrl with trailing slash is normalised (no //path)", () => {
    const xml = generateSitemap([{ url: "/about" }], "https://example.com/");
    expect(xml).toContain(`<loc>https://example.com/about</loc>`);
    expect(xml).not.toContain(`<loc>https://example.com//about</loc>`);
  });
});

describe("generateSitemap — URL validation throws BEFORE emitting", () => {
  it("javascript: in any entry throws TypeError", () => {
    expect(() => generateSitemap([
      { url: "/" },
      { url: "javascript:alert(1)" },
      { url: "/about" },
    ], "https://b")).toThrow(/must be http\(s\)/);
  });

  it("data:", () => {
    expect(() => generateSitemap([{ url: "data:text/html,x" }], "https://b"))
      .toThrow(/http\(s\)/);
  });

  it("file:", () => {
    expect(() => generateSitemap([{ url: "file:///etc" }], "https://b"))
      .toThrow(/http\(s\)/);
  });

  it("protocol-relative //host", () => {
    expect(() => generateSitemap([{ url: "//evil.com" }], "https://b"))
      .toThrow(/http\(s\)/);
  });

  it("bad baseUrl rejected first", () => {
    expect(() => generateSitemap([{ url: "/" }], "javascript:alert(1)"))
      .toThrow(/baseUrl must be http\(s\)/);
  });
});

describe("generateSitemap — changefreq validation", () => {
  it("rejects 'often'", () => {
    expect(() => generateSitemap([
      { url: "/", changefreq: "often" },
    ], "https://b")).toThrow(/one of/);
  });

  it("rejects 'never-ever'", () => {
    expect(() => generateSitemap([
      { url: "/", changefreq: "never-ever" },
    ], "https://b")).toThrow(/one of/);
  });

  it("accepts every allowlist value (parametrised)", () => {
    const values = ["always", "hourly", "daily", "weekly", "monthly", "yearly", "never"];
    for (const v of values) {
      const xml = generateSitemap([{ url: "/", changefreq: v }], "https://b");
      expect(xml).toContain(`<changefreq>${v}</changefreq>`);
    }
  });
});

describe("generateSitemap — priority clamping", () => {
  it("0.8 emitted verbatim", () => {
    const xml = generateSitemap([{ url: "/", priority: 0.8 }], "https://b");
    expect(xml).toContain(`<priority>0.8</priority>`);
  });

  it("above 1 clamps to 1.0", () => {
    const xml = generateSitemap([{ url: "/", priority: 5 }], "https://b");
    expect(xml).toContain(`<priority>1.0</priority>`);
  });

  it("negative clamps to 0.0", () => {
    const xml = generateSitemap([{ url: "/", priority: -1 }], "https://b");
    expect(xml).toContain(`<priority>0.0</priority>`);
  });

  it("NaN → 0.5 (spec default)", () => {
    const xml = generateSitemap([{ url: "/", priority: NaN }], "https://b");
    expect(xml).toContain(`<priority>0.5</priority>`);
  });
});

describe("generateSitemap — XML escaping", () => {
  it("escapes ampersand in lastmod (defensive — real ISO has none)", () => {
    const xml = generateSitemap([
      { url: "/", lastmod: "2026-05-19&extra" },
    ], "https://b");
    expect(xml).toContain(`<lastmod>2026-05-19&amp;extra</lastmod>`);
  });

  it("escapes URL with ampersand + quotes in absolute entry", () => {
    const xml = generateSitemap([
      { url: `https://other.example/?a=1&b=2"` },
    ], "https://b");
    // Note: this URL is structurally weird (quotes in URL); we
    // pass it through but XML-escape the special chars.
    expect(xml).toContain(`<loc>https://other.example/?a=1&amp;b=2&quot;</loc>`);
  });

  it("strips XML 1.0 invalid C0 chars from lastmod", () => {
    const xml = generateSitemap([
      { url: "/", lastmod: "2026\x00-05-19" },
    ], "https://b");
    expect(xml).toContain(`<lastmod>2026-05-19</lastmod>`);
  });
});

describe("generateSitemap — purity / determinism", () => {
  it("does not mutate input entries", () => {
    const entries: SitemapEntry[] = [{ url: "/", priority: 0.8 }];
    const before = JSON.stringify(entries);
    generateSitemap(entries, "https://b");
    expect(JSON.stringify(entries)).toBe(before);
  });

  it("same input → byte-identical output", () => {
    const entries: SitemapEntry[] = [
      { url: "/", priority: 1.0 },
      { url: "/about", changefreq: "monthly" },
    ];
    expect(generateSitemap(entries, "https://b")).toBe(generateSitemap(entries, "https://b"));
  });

  it("preserves caller's entry order", () => {
    const xml = generateSitemap([
      { url: "/c" }, { url: "/a" }, { url: "/b" },
    ], "https://b");
    const cIdx = xml.indexOf(`<loc>https://b/c</loc>`);
    const aIdx = xml.indexOf(`<loc>https://b/a</loc>`);
    const bIdx = xml.indexOf(`<loc>https://b/b</loc>`);
    expect(cIdx).toBeLessThan(aIdx);
    expect(aIdx).toBeLessThan(bIdx);
  });
});

describe("generateSitemap — fail-fast (no partial output)", () => {
  it("if entry N is invalid, NO XML is emitted at all", () => {
    try {
      generateSitemap([
        { url: "/" },
        { url: "/about" },
        { url: "javascript:bad" },
      ], "https://b");
      expect.fail("expected throw");
    } catch (e) {
      // No way to assert "nothing was emitted" since generateSitemap
      // returns a string, but the throw is what proves it.
      expect((e as Error).message).toMatch(/javascript/);
    }
  });

  it("if changefreq is invalid, NO XML emitted", () => {
    try {
      generateSitemap([{ url: "/", changefreq: "often" }], "https://b");
      expect.fail("expected throw");
    } catch (e) {
      expect((e as Error).message).toMatch(/one of/);
    }
  });
});

describe("generateSitemap — stress test", () => {
  it("100 entries emitted without error", () => {
    const entries: SitemapEntry[] = [];
    for (let i = 0; i < 100; i++) {
      entries.push({ url: `/post-${i}`, changefreq: "weekly", priority: 0.5 });
    }
    const xml = generateSitemap(entries, "https://example.com");
    expect((xml.match(/<url>/g) ?? []).length).toBe(100);
  });
});
