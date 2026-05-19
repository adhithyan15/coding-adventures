/**
 * generate.test.ts — end-to-end generateFeedDiscoveryLinks.
 */

import { describe, it, expect } from "vitest";
import { generateFeedDiscoveryLinks } from "../src/index.js";

describe("generateFeedDiscoveryLinks — single link", () => {
  it("minimal href-only entry uses rss+xml default", () => {
    expect(generateFeedDiscoveryLinks({ href: "/feed.xml" }))
      .toBe(`<link rel="alternate" type="application/rss+xml" href="/feed.xml">`);
  });

  it("full entry with title", () => {
    expect(generateFeedDiscoveryLinks({ href: "/feed.xml", title: "My Blog" }))
      .toBe(`<link rel="alternate" type="application/rss+xml" title="My Blog" href="/feed.xml">`);
  });

  it("explicit atom type", () => {
    expect(generateFeedDiscoveryLinks({
      href: "/atom.xml", type: "application/atom+xml", title: "Atom",
    }))
      .toBe(`<link rel="alternate" type="application/atom+xml" title="Atom" href="/atom.xml">`);
  });

  it("JSON Feed type", () => {
    expect(generateFeedDiscoveryLinks({
      href: "/feed.json", type: "application/json", title: "JSON Feed",
    }))
      .toBe(`<link rel="alternate" type="application/json" title="JSON Feed" href="/feed.json">`);
  });

  it("absolute https href", () => {
    expect(generateFeedDiscoveryLinks({ href: "https://example.com/feed.xml" }))
      .toBe(`<link rel="alternate" type="application/rss+xml" href="https://example.com/feed.xml">`);
  });
});

describe("generateFeedDiscoveryLinks — array of links", () => {
  it("multiple feeds joined by newline", () => {
    const out = generateFeedDiscoveryLinks([
      { href: "/feed.xml", type: "application/rss+xml", title: "RSS" },
      { href: "/atom.xml", type: "application/atom+xml", title: "Atom" },
    ]);
    expect(out).toBe([
      `<link rel="alternate" type="application/rss+xml" title="RSS" href="/feed.xml">`,
      `<link rel="alternate" type="application/atom+xml" title="Atom" href="/atom.xml">`,
    ].join("\n"));
  });

  it("preserves caller's order", () => {
    const out = generateFeedDiscoveryLinks([
      { href: "/c.xml" }, { href: "/a.xml" }, { href: "/b.xml" },
    ]);
    const cIdx = out.indexOf("/c.xml");
    const aIdx = out.indexOf("/a.xml");
    const bIdx = out.indexOf("/b.xml");
    expect(cIdx).toBeLessThan(aIdx);
    expect(aIdx).toBeLessThan(bIdx);
  });

  it("single-element array same as single object", () => {
    const single = generateFeedDiscoveryLinks({ href: "/feed.xml" });
    const arr = generateFeedDiscoveryLinks([{ href: "/feed.xml" }]);
    expect(arr).toBe(single);
  });

  it("empty array → empty string", () => {
    expect(generateFeedDiscoveryLinks([])).toBe("");
  });
});

describe("generateFeedDiscoveryLinks — attribute order", () => {
  it("rel → type → title → href", () => {
    const out = generateFeedDiscoveryLinks({
      href: "/feed.xml", type: "application/atom+xml", title: "Test",
    });
    const relIdx = out.indexOf("rel=");
    const typeIdx = out.indexOf("type=");
    const titleIdx = out.indexOf("title=");
    const hrefIdx = out.indexOf("href=");
    expect(relIdx).toBeLessThan(typeIdx);
    expect(typeIdx).toBeLessThan(titleIdx);
    expect(titleIdx).toBeLessThan(hrefIdx);
  });

  it("no title → rel → type → href", () => {
    const out = generateFeedDiscoveryLinks({ href: "/feed.xml" });
    expect(out).not.toContain("title=");
    const relIdx = out.indexOf("rel=");
    const typeIdx = out.indexOf("type=");
    const hrefIdx = out.indexOf("href=");
    expect(relIdx).toBeLessThan(typeIdx);
    expect(typeIdx).toBeLessThan(hrefIdx);
  });
});

describe("generateFeedDiscoveryLinks — HTML escaping", () => {
  it("escapes ampersand in href", () => {
    const out = generateFeedDiscoveryLinks({ href: "https://example.com/?a=1&b=2" });
    expect(out).toContain(`href="https://example.com/?a=1&amp;b=2"`);
  });

  it("escapes title with quotes", () => {
    const out = generateFeedDiscoveryLinks({
      href: "/feed.xml", title: `He said "feed"`,
    });
    expect(out).toContain(`title="He said &quot;feed&quot;"`);
  });

  it("escapes title with angle brackets (XSS attempt)", () => {
    const out = generateFeedDiscoveryLinks({
      href: "/feed.xml", title: `<script>alert("xss")</script>`,
    });
    expect(out).toContain(`title="&lt;script&gt;alert(&quot;xss&quot;)&lt;/script&gt;"`);
    expect(out).not.toContain("<script>alert");
  });

  it("strips control bytes from title", () => {
    const out = generateFeedDiscoveryLinks({
      href: "/feed.xml", title: "My\x00Blog",
    });
    expect(out).toContain(`title="MyBlog"`);
  });
});

describe("generateFeedDiscoveryLinks — URL validation", () => {
  it("javascript: href throws", () => {
    expect(() => generateFeedDiscoveryLinks({ href: "javascript:alert(1)" }))
      .toThrow(/http\(s\)/);
  });

  it("data: href throws", () => {
    expect(() => generateFeedDiscoveryLinks({ href: "data:text/xml,x" }))
      .toThrow(/http\(s\)/);
  });

  it("file: href throws", () => {
    expect(() => generateFeedDiscoveryLinks({ href: "file:///etc/passwd" }))
      .toThrow(/http\(s\)/);
  });

  it("protocol-relative href throws", () => {
    expect(() => generateFeedDiscoveryLinks({ href: "//evil.com" })).toThrow(/http\(s\)/);
  });

  it("backslash-variant href throws", () => {
    expect(() => generateFeedDiscoveryLinks({ href: "/\\evil.com" })).toThrow(/http\(s\)/);
  });
});

describe("generateFeedDiscoveryLinks — type validation", () => {
  it("rss+xml accepted (explicit)", () => {
    const out = generateFeedDiscoveryLinks({ href: "/feed.xml", type: "application/rss+xml" });
    expect(out).toContain(`type="application/rss+xml"`);
  });

  it("rdf+xml (deprecated RSS 1.0) rejected", () => {
    expect(() => generateFeedDiscoveryLinks({
      href: "/feed.xml", type: "application/rdf+xml",
    })).toThrow(/one of/);
  });

  it("text/xml rejected", () => {
    expect(() => generateFeedDiscoveryLinks({
      href: "/feed.xml", type: "text/xml",
    })).toThrow(/one of/);
  });

  it("case-sensitive: 'APPLICATION/RSS+XML' rejected", () => {
    expect(() => generateFeedDiscoveryLinks({
      href: "/feed.xml", type: "APPLICATION/RSS+XML",
    })).toThrow(/one of/);
  });
});

describe("generateFeedDiscoveryLinks — input shape validation", () => {
  it("null link in array throws", () => {
    expect(() => generateFeedDiscoveryLinks([null as unknown as never]))
      .toThrow(/non-null object/);
  });

  it("non-string title throws", () => {
    expect(() => generateFeedDiscoveryLinks({
      href: "/feed.xml", title: 42 as unknown as string,
    })).toThrow(/title must be a string/);
  });

  it("error message identifies bad input index", () => {
    try {
      generateFeedDiscoveryLinks([
        { href: "/good.xml" },
        { href: "javascript:bad" },
      ]);
      expect.fail("expected throw");
    } catch (e) {
      expect((e as Error).message).toMatch(/http\(s\)/);
    }
  });
});

describe("generateFeedDiscoveryLinks — fail-fast", () => {
  it("bad link in mid-array throws without partial output", () => {
    expect(() => generateFeedDiscoveryLinks([
      { href: "/good.xml" },
      { href: "/", type: "text/xml" as unknown as never },
      { href: "/also-good.xml" },
    ])).toThrow(/one of/);
  });
});

describe("generateFeedDiscoveryLinks — purity / determinism", () => {
  it("same input → byte-identical output", () => {
    const input = { href: "/feed.xml", title: "My Blog" };
    expect(generateFeedDiscoveryLinks(input)).toBe(generateFeedDiscoveryLinks(input));
  });

  it("does not mutate input", () => {
    const link = { href: "/feed.xml", title: "X" };
    const before = JSON.stringify(link);
    generateFeedDiscoveryLinks(link);
    expect(JSON.stringify(link)).toBe(before);
  });
});

describe("generateFeedDiscoveryLinks — full real-world example", () => {
  it("blog with RSS + Atom + JSON Feed", () => {
    const out = generateFeedDiscoveryLinks([
      { href: "/feed.xml",  type: "application/rss+xml",  title: "RSS" },
      { href: "/atom.xml",  type: "application/atom+xml", title: "Atom" },
      { href: "/feed.json", type: "application/json",     title: "JSON Feed" },
    ]);
    expect(out).toBe([
      `<link rel="alternate" type="application/rss+xml" title="RSS" href="/feed.xml">`,
      `<link rel="alternate" type="application/atom+xml" title="Atom" href="/atom.xml">`,
      `<link rel="alternate" type="application/json" title="JSON Feed" href="/feed.json">`,
    ].join("\n"));
  });
});
