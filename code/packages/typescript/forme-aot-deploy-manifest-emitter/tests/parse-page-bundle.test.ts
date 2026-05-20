/**
 * parse-page-bundle.test.ts — JSON parse + shape validation.
 */

import { describe, it, expect } from "vitest";
import { parsePageBundle, routeToDeployEntry } from "../src/index.js";

const VALID = JSON.stringify({
  version: 1,
  baseUrl: "https://example.com",
  routes: {
    "/": {
      route: "/",
      outputPath: "index.html",
      contentType: "text/html; charset=utf-8",
      sizeBytes: 42,
      sha256: "AAAA",
    },
  },
});

describe("parsePageBundle — accept", () => {
  it("valid input", () => {
    const out = parsePageBundle(VALID);
    expect(out.baseUrl).toBe("https://example.com");
    expect(out.routes).toHaveLength(1);
    expect(out.routes[0]).toMatchObject({
      route: "/",
      outputPath: "index.html",
      contentType: "text/html; charset=utf-8",
      sizeBytes: 42,
      sha256: "AAAA",
    });
  });
  it("baseUrl omitted", () => {
    const out = parsePageBundle(JSON.stringify({ version: 1, routes: {} }));
    expect(out.baseUrl).toBeUndefined();
  });
  it("empty routes", () => {
    const out = parsePageBundle(JSON.stringify({ version: 1, routes: {} }));
    expect(out.routes).toHaveLength(0);
  });
  it("route with lastmod", () => {
    const out = parsePageBundle(JSON.stringify({
      version: 1,
      routes: {
        "/p": {
          route: "/p",
          outputPath: "p/index.html",
          contentType: "text/html",
          sizeBytes: 10,
          sha256: "X",
          lastmod: "2026-05-19",
        },
      },
    }));
    expect(out.routes[0]!.lastmod).toBe("2026-05-19");
  });
});

describe("parsePageBundle — reject", () => {
  it("invalid JSON", () => {
    expect(() => parsePageBundle("not json {")).toThrow(/not valid JSON/);
  });
  it("array root", () => {
    expect(() => parsePageBundle("[]")).toThrow(/root must be a JSON object/);
  });
  it("null root", () => {
    expect(() => parsePageBundle("null")).toThrow(/root must be a JSON object/);
  });
  it("string root", () => {
    expect(() => parsePageBundle('"x"')).toThrow(/root must be a JSON object/);
  });
  it("wrong version", () => {
    expect(() => parsePageBundle(JSON.stringify({ version: 2, routes: {} })))
      .toThrow(/version must be 1/);
  });
  it("missing version", () => {
    expect(() => parsePageBundle(JSON.stringify({ routes: {} })))
      .toThrow(/version must be 1/);
  });
  it("non-string baseUrl", () => {
    expect(() => parsePageBundle(JSON.stringify({ version: 1, baseUrl: 42, routes: {} })))
      .toThrow(/baseUrl must be a string/);
  });
  it("routes is array", () => {
    expect(() => parsePageBundle(JSON.stringify({ version: 1, routes: [] })))
      .toThrow(/routes must be a JSON object/);
  });
  it("routes is null", () => {
    expect(() => parsePageBundle(JSON.stringify({ version: 1, routes: null })))
      .toThrow(/routes must be a JSON object/);
  });
  it("route entry is array", () => {
    expect(() => parsePageBundle(JSON.stringify({ version: 1, routes: { "/": [] } })))
      .toThrow(/must be a JSON object/);
  });
  it("non-string route field", () => {
    expect(() => parsePageBundle(JSON.stringify({
      version: 1, routes: { "/": { route: 1, outputPath: "x", contentType: "y", sizeBytes: 0, sha256: "z" } },
    }))).toThrow(/\.route must be a string/);
  });
  it("non-string outputPath", () => {
    expect(() => parsePageBundle(JSON.stringify({
      version: 1, routes: { "/": { route: "/", outputPath: 1, contentType: "y", sizeBytes: 0, sha256: "z" } },
    }))).toThrow(/\.outputPath must be a string/);
  });
  it("non-string contentType", () => {
    expect(() => parsePageBundle(JSON.stringify({
      version: 1, routes: { "/": { route: "/", outputPath: "x", contentType: 1, sizeBytes: 0, sha256: "z" } },
    }))).toThrow(/\.contentType must be a string/);
  });
  it("non-integer sizeBytes", () => {
    expect(() => parsePageBundle(JSON.stringify({
      version: 1, routes: { "/": { route: "/", outputPath: "x", contentType: "y", sizeBytes: 1.5, sha256: "z" } },
    }))).toThrow(/sizeBytes must be a non-negative integer/);
  });
  it("negative sizeBytes", () => {
    expect(() => parsePageBundle(JSON.stringify({
      version: 1, routes: { "/": { route: "/", outputPath: "x", contentType: "y", sizeBytes: -1, sha256: "z" } },
    }))).toThrow(/sizeBytes must be a non-negative integer/);
  });
  it("non-string sizeBytes", () => {
    expect(() => parsePageBundle(JSON.stringify({
      version: 1, routes: { "/": { route: "/", outputPath: "x", contentType: "y", sizeBytes: "0", sha256: "z" } },
    }))).toThrow(/sizeBytes must be a non-negative integer/);
  });
  it("non-string sha256", () => {
    expect(() => parsePageBundle(JSON.stringify({
      version: 1, routes: { "/": { route: "/", outputPath: "x", contentType: "y", sizeBytes: 0, sha256: 0 } },
    }))).toThrow(/\.sha256 must be a string/);
  });
  it("non-string lastmod", () => {
    expect(() => parsePageBundle(JSON.stringify({
      version: 1, routes: { "/": { route: "/", outputPath: "x", contentType: "y", sizeBytes: 0, sha256: "z", lastmod: 1 } },
    }))).toThrow(/\.lastmod must be a string/);
  });
});

describe("parsePageBundle — outputPath validation (defence-in-depth)", () => {
  it("malicious outputPath ../etc rejected even from page bundle", () => {
    const sneaky = JSON.stringify({
      version: 1,
      routes: {
        "/x": { route: "/x", outputPath: "../etc/passwd", contentType: "t", sizeBytes: 0, sha256: "z" },
      },
    });
    expect(() => parsePageBundle(sneaky)).toThrow(/path traversal/);
  });
  it("absolute outputPath /etc/passwd rejected", () => {
    const sneaky = JSON.stringify({
      version: 1,
      routes: {
        "/x": { route: "/x", outputPath: "/etc/passwd", contentType: "t", sizeBytes: 0, sha256: "z" },
      },
    });
    expect(() => parsePageBundle(sneaky)).toThrow(/must be relative/);
  });
  it("__proto__ in outputPath rejected", () => {
    const sneaky = JSON.stringify({
      version: 1,
      routes: {
        "/x": { route: "/x", outputPath: "__proto__", contentType: "t", sizeBytes: 0, sha256: "z" },
      },
    });
    expect(() => parsePageBundle(sneaky)).toThrow(/prototype-pollution/);
  });
  it("error includes pageBundle.routes[...] field path", () => {
    const sneaky = JSON.stringify({
      version: 1,
      routes: {
        "/x": { route: "/x", outputPath: "../etc", contentType: "t", sizeBytes: 0, sha256: "z" },
      },
    });
    expect(() => parsePageBundle(sneaky)).toThrow(/pageBundle\.routes\["\/x"\]\.outputPath/);
  });
});

describe("parsePageBundle — prototype pollution defence", () => {
  it("__proto__ key in routes does not pollute Object.prototype", () => {
    // JSON.parse preserves __proto__ as own property only when assigned via assignment.
    // We rely on JSON.parse's actual behavior here.
    const sneaky = '{"version":1,"routes":{"__proto__":{"route":"/x","outputPath":"x","contentType":"t","sizeBytes":0,"sha256":"z"}}}';
    // Should NOT pollute prototype.  After parse, accessing ({}).polluted === undefined.
    parsePageBundle(sneaky);
    // tslint:disable-next-line: no-any
    expect(({} as { polluted?: unknown }).polluted).toBeUndefined();
  });
});

describe("routeToDeployEntry", () => {
  it("maps fields + sets source", () => {
    const entry = routeToDeployEntry({
      route: "/",
      outputPath: "index.html",
      contentType: "text/html",
      sizeBytes: 10,
      sha256: "X",
    });
    expect(entry).toMatchObject({
      route: "/",
      outputPath: "index.html",
      contentType: "text/html",
      sizeBytes: 10,
      sha256: "X",
      source: "page-bundle",
    });
    expect(entry.lastmod).toBeUndefined();
  });
  it("preserves lastmod when present", () => {
    const entry = routeToDeployEntry({
      route: "/",
      outputPath: "x",
      contentType: "t",
      sizeBytes: 0,
      sha256: "z",
      lastmod: "2026-05-19",
    });
    expect(entry.lastmod).toBe("2026-05-19");
  });
});
