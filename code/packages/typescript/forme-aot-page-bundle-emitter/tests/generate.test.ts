/**
 * generate.test.ts — end-to-end generatePageBundle.
 */

import { describe, it, expect } from "vitest";
import { generatePageBundle } from "../src/index.js";

describe("generatePageBundle — shape", () => {
  it("null config throws", () => {
    expect(() => generatePageBundle(null as unknown as never))
      .toThrow(/config must be a non-null object/);
  });
  it("non-array pages throws", () => {
    expect(() => generatePageBundle({ pages: "x" } as unknown as never))
      .toThrow(/pages must be an array/);
  });
  it("empty pages → minimal manifest", () => {
    const out = generatePageBundle({ pages: [] });
    expect(JSON.parse(out)).toEqual({ version: 1, routes: {} });
    expect(out.endsWith("\n")).toBe(true);
  });
});

describe("generatePageBundle — single page", () => {
  it("minimal /", () => {
    const out = generatePageBundle({ pages: [{ route: "/", html: "<!doctype html>" }] });
    const m = JSON.parse(out);
    expect(m.version).toBe(1);
    expect(m.routes["/"]).toMatchObject({
      route: "/",
      outputPath: "index.html",
      contentType: "text/html; charset=utf-8",
      sizeBytes: 15, // "<!doctype html>" = 15 bytes
    });
    expect(m.routes["/"].sha256).toMatch(/^[A-Za-z0-9+/]+=*$/);
  });
  it("custom contentType", () => {
    const m = JSON.parse(generatePageBundle({
      pages: [{ route: "/feed.xml", html: "<?xml?>", contentType: "application/rss+xml" }],
    }));
    expect(m.routes["/feed.xml"].contentType).toBe("application/rss+xml");
  });
  it("lastmod present in entry only when provided", () => {
    const without = JSON.parse(generatePageBundle({
      pages: [{ route: "/", html: "" }],
    }));
    const withLm = JSON.parse(generatePageBundle({
      pages: [{ route: "/", html: "", lastmod: "2026-05-19" }],
    }));
    expect("lastmod" in without.routes["/"]).toBe(false);
    expect(withLm.routes["/"].lastmod).toBe("2026-05-19");
  });
});

describe("generatePageBundle — multiple pages", () => {
  it("routes sorted alphabetically regardless of input order", () => {
    const out = generatePageBundle({
      pages: [
        { route: "/zebra", html: "z" },
        { route: "/about", html: "a" },
        { route: "/posts/x", html: "p" },
        { route: "/", html: "i" },
      ],
    });
    const lines = out.split("\n");
    const routeOrder = lines.filter((l) => l.includes(`"route":`));
    expect(routeOrder[0]).toContain(`"route": "/"`);
    expect(routeOrder[1]).toContain(`"route": "/about"`);
    expect(routeOrder[2]).toContain(`"route": "/posts/x"`);
    expect(routeOrder[3]).toContain(`"route": "/zebra"`);
  });
  it("duplicate routes throw", () => {
    expect(() => generatePageBundle({
      pages: [
        { route: "/", html: "a" },
        { route: "/", html: "b" },
      ],
    })).toThrow(/duplicates an earlier entry: "\/"/);
  });
});

describe("generatePageBundle — baseUrl", () => {
  it("included when provided", () => {
    const m = JSON.parse(generatePageBundle({
      baseUrl: "https://example.com",
      pages: [{ route: "/", html: "" }],
    }));
    expect(m.baseUrl).toBe("https://example.com");
  });
  it("omitted when absent", () => {
    const m = JSON.parse(generatePageBundle({ pages: [{ route: "/", html: "" }] }));
    expect("baseUrl" in m).toBe(false);
  });
  it("javascript: baseUrl throws", () => {
    expect(() => generatePageBundle({
      baseUrl: "javascript:alert(1)",
      pages: [],
    })).toThrow(/baseUrl must be http\(s\)/);
  });
});

describe("generatePageBundle — route validation", () => {
  it("traversal /a/.. throws", () => {
    expect(() => generatePageBundle({
      pages: [{ route: "/a/..", html: "" }],
    })).toThrow(/path traversal/);
  });
  it("protocol-relative //x throws", () => {
    expect(() => generatePageBundle({
      pages: [{ route: "//x", html: "" }],
    })).toThrow(/protocol-relative/);
  });
  it("backslash /\\x throws", () => {
    expect(() => generatePageBundle({
      pages: [{ route: "/\\x", html: "" }],
    })).toThrow(/backslash variant/);
  });
  it("absolute https://... throws (must start with /)", () => {
    expect(() => generatePageBundle({
      pages: [{ route: "https://evil.com", html: "" }],
    })).toThrow(/must start with "\/"/);
  });
});

describe("generatePageBundle — page field validation", () => {
  it("non-string html throws", () => {
    expect(() => generatePageBundle({
      pages: [{ route: "/", html: 42 as unknown as string }],
    })).toThrow(/pages\[0\]\.html must be a string/);
  });
  it("non-string contentType throws", () => {
    expect(() => generatePageBundle({
      pages: [{ route: "/", html: "", contentType: 42 as unknown as string }],
    })).toThrow(/contentType must be a string/);
  });
  it("non-string lastmod throws", () => {
    expect(() => generatePageBundle({
      pages: [{ route: "/", html: "", lastmod: 42 as unknown as string }],
    })).toThrow(/lastmod must be a string/);
  });
  it("null page entry throws", () => {
    expect(() => generatePageBundle({
      pages: [null as unknown as never],
    })).toThrow(/pages\[0\] must be a non-null object/);
  });
});

describe("generatePageBundle — output format", () => {
  it("2-space indented JSON", () => {
    const out = generatePageBundle({ pages: [{ route: "/", html: "" }] });
    expect(out).toContain('\n  "version": 1');
  });
  it("trailing newline", () => {
    const out = generatePageBundle({ pages: [] });
    expect(out.endsWith("\n")).toBe(true);
    expect(out.endsWith("\n\n")).toBe(false);
  });
  it("top-level key order: version → baseUrl → routes", () => {
    const out = generatePageBundle({
      baseUrl: "https://x",
      pages: [{ route: "/", html: "" }],
    });
    const vIdx = out.indexOf(`"version"`);
    const bIdx = out.indexOf(`"baseUrl"`);
    const rIdx = out.indexOf(`"routes"`);
    expect(vIdx).toBeLessThan(bIdx);
    expect(bIdx).toBeLessThan(rIdx);
  });
  it("entry key order: route → outputPath → contentType → sizeBytes → sha256 → lastmod", () => {
    const out = generatePageBundle({
      pages: [{ route: "/", html: "", lastmod: "2026-05-19" }],
    });
    const order = ["route", "outputPath", "contentType", "sizeBytes", "sha256", "lastmod"];
    let lastIdx = -1;
    for (const key of order) {
      const idx = out.indexOf(`"${key}"`);
      expect(idx).toBeGreaterThan(lastIdx);
      lastIdx = idx;
    }
  });
});

describe("generatePageBundle — determinism", () => {
  it("same input → byte-identical output", () => {
    const cfg = {
      baseUrl: "https://example.com",
      pages: [
        { route: "/", html: "<!doctype html>" },
        { route: "/about", html: "<!doctype html>...about" },
      ],
    };
    expect(generatePageBundle(cfg)).toBe(generatePageBundle(cfg));
  });
  it("reordering input does not change output", () => {
    const a = generatePageBundle({
      pages: [
        { route: "/a", html: "a" },
        { route: "/b", html: "b" },
      ],
    });
    const b = generatePageBundle({
      pages: [
        { route: "/b", html: "b" },
        { route: "/a", html: "a" },
      ],
    });
    expect(a).toBe(b);
  });
  it("does not mutate input", () => {
    const pages = [{ route: "/", html: "x" }];
    const before = JSON.stringify(pages);
    generatePageBundle({ pages });
    expect(JSON.stringify(pages)).toBe(before);
  });
});

describe("generatePageBundle — content hashing", () => {
  it("different html → different sha256", () => {
    const a = JSON.parse(generatePageBundle({ pages: [{ route: "/", html: "a" }] }));
    const b = JSON.parse(generatePageBundle({ pages: [{ route: "/", html: "b" }] }));
    expect(a.routes["/"].sha256).not.toBe(b.routes["/"].sha256);
  });
  it("same html → same sha256 across runs", () => {
    const a = JSON.parse(generatePageBundle({ pages: [{ route: "/", html: "abc" }] }));
    const b = JSON.parse(generatePageBundle({ pages: [{ route: "/", html: "abc" }] }));
    expect(a.routes["/"].sha256).toBe(b.routes["/"].sha256);
  });
  it("sizeBytes counts UTF-8 bytes (not chars)", () => {
    const m = JSON.parse(generatePageBundle({ pages: [{ route: "/", html: "café" }] }));
    expect(m.routes["/"].sizeBytes).toBe(5); // c+a+f+é(2 bytes)
  });
});

describe("generatePageBundle — full real-world example", () => {
  it("multi-page site with baseUrl + feed", () => {
    const out = generatePageBundle({
      baseUrl: "https://example.com",
      pages: [
        { route: "/", html: "<!doctype html><title>Home</title>" },
        { route: "/about", html: "<!doctype html><title>About</title>" },
        { route: "/posts/first", html: "<!doctype html><title>First</title>", lastmod: "2026-05-19" },
        { route: "/feed.xml", html: "<?xml version=\"1.0\"?><rss/>", contentType: "application/rss+xml" },
      ],
    });
    const m = JSON.parse(out);
    expect(m.version).toBe(1);
    expect(m.baseUrl).toBe("https://example.com");
    expect(Object.keys(m.routes)).toEqual(["/", "/about", "/feed.xml", "/posts/first"]);
    expect(m.routes["/"].outputPath).toBe("index.html");
    expect(m.routes["/about"].outputPath).toBe("about/index.html");
    expect(m.routes["/posts/first"].outputPath).toBe("posts/first/index.html");
    expect(m.routes["/feed.xml"].outputPath).toBe("feed.xml");
    expect(m.routes["/feed.xml"].contentType).toBe("application/rss+xml");
    expect(m.routes["/posts/first"].lastmod).toBe("2026-05-19");
  });
});
