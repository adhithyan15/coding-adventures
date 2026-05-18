/**
 * page-emitter.test.ts — per-page artefact emission semantics.
 */

import { describe, it, expect } from "vitest";
import type { CssArtifact } from "@coding-adventures/forme-aot-css-slicer";
import { emitPages, type EmitIO } from "../src/index.js";

// ─── In-memory IO scaffolding ────────────────────────────────────────────

function memIO(): EmitIO & { files: Map<string, string>; dirs: Set<string> } {
  const files = new Map<string, string>();
  const dirs = new Set<string>();
  return {
    files,
    dirs,
    mkdir: async (dir) => { dirs.add(dir); },
    writeFile: async (file, contents) => { files.set(file, contents); },
  };
}

function art(pageId: string, css: string): CssArtifact {
  return {
    pageId,
    css,
    emittedRules: [],
    warnings: [],
    byteSize: Buffer.byteLength(css, "utf8"),
    sha256: "x".repeat(64),
  };
}

function mapOf(...as: CssArtifact[]): Map<string, CssArtifact> {
  return new Map(as.map((a) => [a.pageId, a]));
}

// ─── Tests ───────────────────────────────────────────────────────────────

describe("emitPages — basic CSS write", () => {
  it("writes one CSS file per page", async () => {
    const io = memIO();
    const result = await emitPages("/dist", mapOf(
      art("/index.html", "p { color: red; }"),
      art("/about.html", "p { color: blue; }"),
    ), {}, io);

    expect(result.written.size).toBe(2);
    expect(io.files.get("/dist/index.css")).toBe("p { color: red; }");
    expect(io.files.get("/dist/about.css")).toBe("p { color: blue; }");
  });

  it("includes byteSize per page and totalBytes", async () => {
    const io = memIO();
    const css1 = "a{color:red}";
    const css2 = "b{color:blue}";
    const result = await emitPages("/d", mapOf(
      art("/a", css1),
      art("/b", css2),
    ), {}, io);

    expect(result.written.get("/a")!.byteSize).toBe(css1.length);
    expect(result.written.get("/b")!.byteSize).toBe(css2.length);
    expect(result.totalBytes).toBe(css1.length + css2.length);
  });

  it("empty artefacts map → empty result", async () => {
    const io = memIO();
    const result = await emitPages("/d", new Map(), {}, io);
    expect(result.written.size).toBe(0);
    expect(result.totalBytes).toBe(0);
    expect(io.files.size).toBe(0);
  });

  it("creates parent directories recursively (mkdir recursive)", async () => {
    const io = memIO();
    await emitPages("/dist", mapOf(art("/blog/posts/hello.html", "x")), {}, io);
    expect(io.dirs.has("/dist/blog/posts")).toBe(true);
    expect(io.files.has("/dist/blog/posts/hello.css")).toBe(true);
  });
});

describe("emitPages — route → file path mapping", () => {
  it.each([
    ["/",                  "/dist/index.html",          "/dist/index.css"],
    ["/about",             "/dist/about.html",          "/dist/about.css"],
    ["/about.html",        "/dist/about.html",          "/dist/about.css"],
    ["/blog/",             "/dist/blog/index.html",     "/dist/blog/index.css"],
    ["/blog/post.html",    "/dist/blog/post.html",      "/dist/blog/post.css"],
    ["/blog/post",         "/dist/blog/post.html",      "/dist/blog/post.css"],
    ["/_next/static/x",    "/dist/_next/static/x.html", "/dist/_next/static/x.css"],
  ])("pageId %j → html %j + css %j", async (pageId, htmlPath, cssPath) => {
    const io = memIO();
    await emitPages("/dist", mapOf(art(pageId, ".x{}")), { writeHtml: true }, io);
    expect(io.files.has(cssPath)).toBe(true);
    expect(io.files.has(htmlPath)).toBe(true);
  });
});

describe("emitPages — HTML wrapper", () => {
  it("writeHtml: false (default) emits NO HTML", async () => {
    const io = memIO();
    const result = await emitPages("/d", mapOf(art("/index.html", "x")), {}, io);
    expect(result.written.get("/index.html")!.htmlPath).toBeUndefined();
    expect(io.files.has("/d/index.html")).toBe(false);
  });

  it("writeHtml: true emits a minimal HTML wrapper", async () => {
    const io = memIO();
    await emitPages("/d", mapOf(art("/index.html", "x")), { writeHtml: true }, io);
    const html = io.files.get("/d/index.html");
    expect(html).toContain("<!doctype html>");
    expect(html).toContain("<meta charset=\"utf-8\">");
    expect(html).toContain(`<link rel="stylesheet" href="index.css">`);
    expect(html).toContain("<body>");
    expect(html).toContain("</body>");
  });

  it("htmlBody callback contributes to the body", async () => {
    const io = memIO();
    await emitPages("/d", mapOf(art("/about.html", "x")), {
      writeHtml: true,
      htmlBody: (pageId) => `<h1>page: ${pageId}</h1>`,
    }, io);
    const html = io.files.get("/d/about.html")!;
    expect(html).toContain("<h1>page: /about.html</h1>");
  });

  it("default htmlBody is empty string (still valid HTML)", async () => {
    const io = memIO();
    await emitPages("/d", mapOf(art("/index.html", "x")), { writeHtml: true }, io);
    const html = io.files.get("/d/index.html")!;
    // Just verify it's syntactically reasonable — no body content
    // between the body tags.
    expect(html).toMatch(/<body>\s*\n*\s*<\/body>/);
  });

  it("totalBytes includes HTML wrapper bytes when writeHtml is true", async () => {
    const io = memIO();
    const css = "p{color:red}";
    const result = await emitPages("/d", mapOf(art("/index.html", css)), { writeHtml: true }, io);
    const cssBytes = Buffer.byteLength(css, "utf8");
    const htmlBytes = Buffer.byteLength(io.files.get("/d/index.html")!, "utf8");
    expect(result.totalBytes).toBe(cssBytes + htmlBytes);
    expect(result.written.get("/index.html")!.byteSize).toBe(cssBytes + htmlBytes);
  });
});

describe("emitPages — pageId validation (path traversal defence)", () => {
  it("rejects empty pageId", async () => {
    const io = memIO();
    await expect(emitPages("/d", mapOf(art("", "x")), {}, io)).rejects.toThrow(/non-empty/);
  });

  it("rejects pageId with `..` segment", async () => {
    const io = memIO();
    await expect(emitPages("/d", mapOf(art("/../../etc/passwd", "x")), {}, io)).rejects.toThrow(/\.\./);
  });

  it("rejects pageId with `.` segment", async () => {
    const io = memIO();
    await expect(emitPages("/d", mapOf(art("/./hidden", "x")), {}, io)).rejects.toThrow(/segment/);
  });

  it("rejects pageId with embedded NUL", async () => {
    const io = memIO();
    await expect(emitPages("/d", mapOf(art("/x\x00y", "x")), {}, io)).rejects.toThrow(/forbidden/);
  });

  it("rejects pageId with embedded control character", async () => {
    const io = memIO();
    await expect(emitPages("/d", mapOf(art("/x\x1by", "x")), {}, io)).rejects.toThrow(/forbidden/);
  });

  it("rejects pageId with backslash (Windows ambiguity)", async () => {
    const io = memIO();
    await expect(emitPages("/d", mapOf(art("/foo\\bar", "x")), {}, io)).rejects.toThrow(/forbidden/);
  });

  it("rejects Windows-style absolute path (drive letter)", async () => {
    const io = memIO();
    await expect(emitPages("/d", mapOf(art("C:/Windows/x", "x")), {}, io)).rejects.toThrow(/Windows absolute/);
  });

  it("rejects double-leading-slash absolute path", async () => {
    const io = memIO();
    await expect(emitPages("/d", mapOf(art("//etc/passwd", "x")), {}, io)).rejects.toThrow(/absolute path/);
  });

  it("rejects leading backslash absolute path", async () => {
    const io = memIO();
    await expect(emitPages("/d", mapOf(art("\\Windows\\x", "x")), {}, io)).rejects.toThrow(/absolute path/);
  });
});

describe("emitPages — overwrites pre-existing files", () => {
  it("second emit replaces the first one's CSS content", async () => {
    const io = memIO();
    await emitPages("/d", mapOf(art("/a.html", "v1")), {}, io);
    expect(io.files.get("/d/a.css")).toBe("v1");
    await emitPages("/d", mapOf(art("/a.html", "v2")), {}, io);
    expect(io.files.get("/d/a.css")).toBe("v2");
  });
});

describe("emitPages — IO injection", () => {
  it("uses caller-provided IO exclusively (no implicit fs access)", async () => {
    const io = memIO();
    await emitPages("/dist", mapOf(art("/index.html", "x")), {}, io);
    // All writes landed in the in-memory store.
    expect(io.files.size).toBe(1);
    // dirs are tracked too.
    expect(io.dirs.size).toBe(1);
  });

  it("writes are atomic from the IO's perspective (one mkdir + one writeFile per CSS, plus one writeFile per HTML)", async () => {
    const writes: string[] = [];
    const dirs: string[] = [];
    const io: EmitIO = {
      mkdir: async (dir) => { dirs.push(dir); },
      writeFile: async (file) => { writes.push(file); },
    };
    await emitPages("/dist", mapOf(art("/a.html", "x"), art("/b.html", "y")), { writeHtml: true }, io);
    // 1 mkdir + 1 css write + 1 html write per page = 2 + 2 = 4 calls + 2 mkdirs
    expect(dirs.length).toBe(2);
    expect(writes.length).toBe(4);
  });
});

describe("emitPages — preserves caller's page iteration order", () => {
  it("written Map iterates in input order", async () => {
    const io = memIO();
    const r = await emitPages("/d", mapOf(
      art("/c.html", "c"),
      art("/a.html", "a"),
      art("/b.html", "b"),
    ), {}, io);
    expect([...r.written.keys()]).toEqual(["/c.html", "/a.html", "/b.html"]);
  });
});

describe("emitPages — HTML attribute escaping (defensive)", () => {
  it("CSS href in the link tag is HTML-attribute-escaped", async () => {
    // The href comes from the basename of the CSS path, which
    // itself comes from the (validated) pageId.  Real-world bases
    // are safe-by-construction.  Verify the escape function works
    // for any case where the basename could in theory contain `&`.
    // Since the validator already rejects most dangerous chars, we
    // can only test with characters that ARE permitted in pageIds
    // but technically should be HTML-attr-escaped — like `&` and `<`.
    const io = memIO();
    await emitPages("/d", mapOf(art("/r&d.html", "x")), { writeHtml: true }, io);
    const html = io.files.get("/d/r&d.html")!;
    // The href attribute value should contain `&amp;`, not raw `&`.
    expect(html).toContain(`href="r&amp;d.css"`);
  });
});

describe("emitPages — large set stress (50 pages)", () => {
  it("emits 50 pages without error and reports correct totals", async () => {
    const io = memIO();
    const pages = new Map<string, CssArtifact>();
    for (let i = 0; i < 50; i++) {
      pages.set(`/page-${i}.html`, art(`/page-${i}.html`, `p{color:#${i.toString(16).padStart(6, "0")}}`));
    }
    const r = await emitPages("/d", pages, {}, io);
    expect(r.written.size).toBe(50);
    expect(io.files.size).toBe(50);
  });
});

describe("emitPages — pageId without leading slash", () => {
  it("accepts pageIds without leading slash (route-shaped)", async () => {
    const io = memIO();
    await emitPages("/d", mapOf(art("index.html", "x")), {}, io);
    expect(io.files.has("/d/index.css")).toBe(true);
  });
});
