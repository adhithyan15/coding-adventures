/**
 * build.test.ts — smoke + integration tests for the demo driver.
 *
 * Strategy: rather than re-test every upstream package's contract
 * (those have their own >95% suites), we assert that this driver
 * actually composes them end-to-end correctly:
 *
 *   - Reading the bundled corpus produces the expected page
 *     count and stable order.
 *   - Building the bundle produces a route per page + sidebar +
 *     search manifest + search shards.
 *   - The rendered HTML for at least one page contains the
 *     expected fingerprint of every upstream package's output
 *     (heading anchors, code-block decoration, sidebar
 *     highlight, etc.).
 *   - Writing the bundle creates a real `dist/` directory
 *     with `index.html`, nested routes, and search files.
 *   - Round-trips through `generatePageBundle` (the FM00
 *     downstream consumer) cleanly.
 *   - safeJoin rejects path-escape attacks.
 */

import { describe, it, expect, beforeAll } from "vitest";
import * as fs from "node:fs/promises";
import * as path from "node:path";
import { generatePageBundle } from "@coding-adventures/forme-aot-page-bundle-emitter";

import { build, routeFor, titleOf, injectHeadingIds, normaliseSidebarPaths, buildIdFor, type MarkdownFile } from "../src/build.js";
import { readCorpus, writeBundle, safeJoin, validateOutDir } from "../src/main.js";
import { plainText } from "../src/plain-text.js";

const REPO_ROOT = path.resolve(__dirname, "..");
const CORPUS = path.join(REPO_ROOT, "corpus");

// ─────────────────────────────────────────────────────────────────────
// routeFor
// ─────────────────────────────────────────────────────────────────────

describe("routeFor", () => {
  it("turns index.md into /", () => expect(routeFor("index.md")).toBe("/"));
  it("turns getting-started.md into /getting-started", () =>
    expect(routeFor("getting-started.md")).toBe("/getting-started"));
  it("turns guide/installation.md into /guide/installation", () =>
    expect(routeFor("guide/installation.md")).toBe("/guide/installation"));
  it("turns guide/index.md into /guide", () =>
    expect(routeFor("guide/index.md")).toBe("/guide"));
  it("strips leading ./", () =>
    expect(routeFor("./api/reference.md")).toBe("/api/reference"));
  it("strips leading /", () =>
    expect(routeFor("/api/reference.md")).toBe("/api/reference"));
  it("strips .mdx extension", () =>
    expect(routeFor("page.mdx")).toBe("/page"));
  it("preserves non-.md extensions", () =>
    expect(routeFor("file.json")).toBe("/file.json"));
});

// ─────────────────────────────────────────────────────────────────────
// titleOf
// ─────────────────────────────────────────────────────────────────────

describe("titleOf", () => {
  it("prefers frontmatter.title when present", () =>
    expect(titleOf({ title: "Hello" }, "any.md")).toBe("Hello"));
  it("humanises basename when frontmatter has no title", () =>
    expect(titleOf({}, "getting-started.md")).toBe("Getting Started"));
  it("falls back when title is empty string", () =>
    expect(titleOf({ title: "" }, "x.md")).toBe("X"));
  it("falls back when title is non-string", () =>
    expect(titleOf({ title: 5 as unknown as string }, "x.md")).toBe("X"));
  it("handles nested paths", () =>
    expect(titleOf({}, "guide/installation.md")).toBe("Installation"));
  it("handles underscored names", () =>
    expect(titleOf({}, "advanced_topics.md")).toBe("Advanced Topics"));
});

// ─────────────────────────────────────────────────────────────────────
// injectHeadingIds
// ─────────────────────────────────────────────────────────────────────

describe("injectHeadingIds", () => {
  it("injects ids in document order", () => {
    const html = "<h1>A</h1>\n<p>x</p>\n<h2>B</h2>\n<h3>C</h3>\n";
    const anchors = [{ id: "a" }, { id: "b" }, { id: "c" }];
    expect(injectHeadingIds(html, anchors)).toBe(
      `<h1 id="a">A</h1>\n<p>x</p>\n<h2 id="b">B</h2>\n<h3 id="c">C</h3>\n`,
    );
  });

  it("leaves extra <h*> tags untouched when anchors run out", () => {
    const html = "<h1>A</h1>\n<h2>B</h2>\n";
    const anchors = [{ id: "only" }];
    expect(injectHeadingIds(html, anchors)).toBe(
      `<h1 id="only">A</h1>\n<h2>B</h2>\n`,
    );
  });

  it("escapes HTML-special chars in id values (defence-in-depth)", () => {
    const html = "<h1>A</h1>\n";
    const anchors = [{ id: `a"&<>` }];
    expect(injectHeadingIds(html, anchors)).toBe(
      `<h1 id="a&quot;&amp;&lt;&gt;">A</h1>\n`,
    );
  });
});

// ─────────────────────────────────────────────────────────────────────
// normaliseSidebarPaths
// ─────────────────────────────────────────────────────────────────────

describe("normaliseSidebarPaths", () => {
  it("rewrites file-path entries to URL routes", () => {
    const input = [
      { kind: "page" as const, label: "Intro", path: "intro.md" },
      { kind: "group" as const, label: "Guide", path: "guide/index.md", children: [
        { kind: "page" as const, label: "Setup", path: "guide/setup.md" },
      ] },
    ];
    const out = normaliseSidebarPaths(input);
    expect(out[0]!.path).toBe("/intro");
    expect(out[1]!.path).toBe("/guide");
    expect(out[1]!.children![0]!.path).toBe("/guide/setup");
  });

  it("preserves null paths (groups without an index page)", () => {
    const input = [{ kind: "group" as const, label: "Misc", path: null, children: [] }];
    const out = normaliseSidebarPaths(input);
    expect(out[0]!.path).toBe(null);
  });
});

// ─────────────────────────────────────────────────────────────────────
// build — pipeline integration
// ─────────────────────────────────────────────────────────────────────

describe("build — full pipeline", () => {
  let files: readonly MarkdownFile[];
  let bundle: ReturnType<typeof build>;

  beforeAll(async () => {
    files = await readCorpus(CORPUS);
    bundle = build(files, {
      siteTitle: "Acme Docs",
      githubUrl: "https://github.com/example/acme",
      copyright: "© 2026 Acme",
    });
  });

  it("reads exactly the 6 corpus pages", () => {
    expect(files.length).toBe(6);
  });

  it("reads pages in stable sorted order", () => {
    expect(files.map((f) => f.path)).toEqual([
      "api/reference.md",
      "faq.md",
      "getting-started.md",
      "guide/configuration.md",
      "guide/installation.md",
      "index.md",
    ]);
  });

  it("produces one route per page", () => {
    const pageRoutes = bundle.pages
      .map((p) => p.route)
      .filter((r) => !r.startsWith("/search/") && r !== "/sidebar.json");
    expect(pageRoutes.sort()).toEqual([
      "/",
      "/api/reference",
      "/faq",
      "/getting-started",
      "/guide/configuration",
      "/guide/installation",
    ]);
  });

  it("emits a sidebar.json with the right top-level entries", () => {
    // sidebar-builder treats the root index.md as the implicit root of
    // the tree (its frontmatter `sidebar_label` is therefore NOT a
    // top-level entry — the home page is reached via the brand link in
    // the header).  Top-level entries are: non-index root pages +
    // every subdirectory group.
    const sidebar = bundle.pages.find((p) => p.route === "/sidebar.json");
    expect(sidebar).toBeDefined();
    const parsed = JSON.parse(sidebar!.html) as Array<{ label: string; kind: "page" | "group" }>;
    const labels = parsed.map((e) => e.label);
    expect(labels).toContain("Getting Started");
    expect(labels).toContain("FAQ");
    expect(labels).toContain("Guide");
    expect(labels).toContain("API");
  });

  it("normalises sidebar paths to URL routes (so aria-current matches)", () => {
    const sidebar = bundle.pages.find((p) => p.route === "/sidebar.json")!;
    const parsed = JSON.parse(sidebar.html) as Array<{ path: string | null; children?: Array<{ path: string | null }> }>;
    // Every leaf page's path should be a URL route, not a .md file path.
    for (const entry of parsed) {
      if (entry.path !== null) {
        expect(entry.path.startsWith("/")).toBe(true);
        expect(entry.path.endsWith(".md")).toBe(false);
      }
      if (entry.children) {
        for (const child of entry.children) {
          if (child.path !== null) {
            expect(child.path.startsWith("/")).toBe(true);
            expect(child.path.endsWith(".md")).toBe(false);
          }
        }
      }
    }
  });

  it("emits a search manifest listing every page", () => {
    const manifestEntry = bundle.pages.find((p) => p.route === "/search/manifest.json");
    expect(manifestEntry).toBeDefined();
    const m = JSON.parse(manifestEntry!.html) as { pages: string[]; shardKeys: string[] };
    expect(m.pages).toContain("/");
    expect(m.pages).toContain("/getting-started");
    expect(m.pages).toContain("/guide/installation");
    expect(m.shardKeys.length).toBeGreaterThan(0);
  });

  it("emits one search shard per shardKey in the manifest", () => {
    const manifestEntry = bundle.pages.find((p) => p.route === "/search/manifest.json")!;
    const m = JSON.parse(manifestEntry.html) as { shardKeys: string[] };
    for (const key of m.shardKeys) {
      const shardEntry = bundle.pages.find((p) => p.route === `/search/${key}.json`);
      expect(shardEntry, `shard /search/${key}.json missing`).toBeDefined();
    }
  });

  it("contains a heading anchor in a page's HTML (heading-anchors pipeline)", () => {
    const home = bundle.pages.find((p) => p.route === "/")!;
    // heading-anchors emits id="..." on every <h*>
    expect(home.html).toMatch(/<h[1-6][^>]*id="[^"]+"/);
  });

  it("contains the page-shell chrome (header, sidebar, footer)", () => {
    const home = bundle.pages.find((p) => p.route === "/")!;
    expect(home.html).toMatch(/<header[\s>]/);
    expect(home.html).toMatch(/<nav[^>]+class="[^"]*sidebar/);
    expect(home.html).toMatch(/<footer[\s>]/);
  });

  it("highlights the current page in the sidebar via aria-current", () => {
    // `/` itself isn't in the sidebar (root index.md is the implicit
    // root, not a sidebar entry — see the test above).  A page that
    // IS in the sidebar — like /getting-started — should be highlighted
    // when it's the current page.
    const gs = bundle.pages.find((p) => p.route === "/getting-started")!;
    expect(gs.html).toMatch(/aria-current="page"/);
  });

  it("renders a fenced code block as <pre><code>", () => {
    const gs = bundle.pages.find((p) => p.route === "/getting-started")!;
    expect(gs.html).toMatch(/<pre><code/);
  });

  it("injects the theme CSS into <head>", () => {
    const home = bundle.pages.find((p) => p.route === "/")!;
    expect(home.html).toMatch(/<style>[^<]*\.sidebar/);
  });

  it("uses the site title in <title> and the brand", () => {
    const home = bundle.pages.find((p) => p.route === "/")!;
    expect(home.html).toContain("Acme Docs");
  });

  it("feeds cleanly into generatePageBundle (round-trip)", () => {
    const manifestJson = generatePageBundle(bundle);
    const manifest = JSON.parse(manifestJson) as { version: 1; routes: Record<string, unknown> };
    expect(manifest.version).toBe(1);
    // Every emitted PageEntry should be in the routes table.
    for (const p of bundle.pages) {
      expect(manifest.routes[p.route]).toBeDefined();
    }
  });
});

// ─────────────────────────────────────────────────────────────────────
// writeBundle — disk output
// ─────────────────────────────────────────────────────────────────────

describe("writeBundle", () => {
  let tmpDir: string;

  beforeAll(async () => {
    tmpDir = await fs.mkdtemp(path.join(REPO_ROOT, ".tmp-write-"));
    const files = await readCorpus(CORPUS);
    const bundle = build(files, {
      siteTitle: "Acme Docs",
      copyright: "© 2026 Acme",
    });
    await writeBundle(bundle, tmpDir);
  });

  it("writes index.html at the output root", async () => {
    const stat = await fs.stat(path.join(tmpDir, "index.html"));
    expect(stat.isFile()).toBe(true);
  });

  it("writes nested route output at <route>/index.html", async () => {
    const stat = await fs.stat(path.join(tmpDir, "guide/installation/index.html"));
    expect(stat.isFile()).toBe(true);
  });

  it("writes search/manifest.json", async () => {
    const body = await fs.readFile(path.join(tmpDir, "search/manifest.json"), "utf8");
    const parsed = JSON.parse(body) as { pages: string[] };
    expect(parsed.pages.length).toBeGreaterThan(0);
  });

  it("writes sidebar.json", async () => {
    const stat = await fs.stat(path.join(tmpDir, "sidebar.json"));
    expect(stat.isFile()).toBe(true);
  });
});

// ─────────────────────────────────────────────────────────────────────
// buildIdFor
// ─────────────────────────────────────────────────────────────────────

describe("buildIdFor", () => {
  it("returns a 12-char hex string", () => {
    const id = buildIdFor("some bundle source");
    expect(id).toMatch(/^[0-9a-f]{12}$/);
  });

  it("is deterministic for the same input", () => {
    expect(buildIdFor("hello")).toBe(buildIdFor("hello"));
  });

  it("differs when the bundle source changes (cache-bust correctness)", () => {
    const a = buildIdFor("bundle version A");
    const b = buildIdFor("bundle version B");
    expect(a).not.toBe(b);
  });
});

// ─────────────────────────────────────────────────────────────────────
// plainText
// ─────────────────────────────────────────────────────────────────────

describe("plainText", () => {
  it("extracts text from a flat node (text field)", () => {
    expect(plainText({ type: "text", text: "hello" })).toBe("hello");
  });

  it("extracts text from a flat node (value field — the IR's actual choice)", () => {
    // commonmark-parser's TextNode uses `value`, not `text`.
    // Both names supported defensively.
    expect(plainText({ type: "text", value: "hello" })).toBe("hello");
  });

  it("recurses into children and joins with spaces", () => {
    const ast = {
      type: "doc",
      children: [
        { type: "p", children: [{ type: "text", value: "hello" }, { type: "text", value: "world" }] },
      ],
    };
    expect(plainText(ast)).toBe("hello world");
  });

  it("extracts text from a realistic commonmark-parser AST", () => {
    // Shape mirrors what commonmark-parser actually emits — nested
    // paragraphs with text children using `value`.  This is the
    // regression test for the bug where plainText returned "" for
    // every page because it only checked `text` and never `value`.
    const ast = {
      type: "document",
      children: [
        { type: "paragraph", children: [
          { type: "text", value: "Hello world" },
        ]},
        { type: "heading", level: 2, children: [
          { type: "text", value: "Install" },
        ]},
      ],
    };
    expect(plainText(ast)).toBe("Hello world Install");
  });

  it("returns empty string for null / undefined / non-object", () => {
    expect(plainText(null)).toBe("");
    expect(plainText(undefined)).toBe("");
    expect(plainText(5)).toBe("");
    expect(plainText("string")).toBe("");  // text field, not the node itself
  });

  it("respects MAX_DEPTH on a cyclic AST without stack overflow", () => {
    // Build a cyclic node: parent.children = [parent] forever.
    const cyclic: { type: string; text: string; children?: unknown[] } = {
      type: "p",
      text: "loop",
    };
    cyclic.children = [cyclic];
    // Should not throw — the depth cap protects us.
    expect(() => plainText(cyclic)).not.toThrow();
  });
});

// ─────────────────────────────────────────────────────────────────────
// safeJoin + validateOutDir
// ─────────────────────────────────────────────────────────────────────

describe("safeJoin", () => {
  it("joins a normal relative path under the base", () => {
    expect(safeJoin("/tmp/site", "page/index.html")).toBe(path.join("/tmp/site", "page/index.html"));
  });
  it("rejects ../ escape", () => {
    expect(() => safeJoin("/tmp/site", "../etc/passwd")).toThrow(/escapes/);
  });
  it("rejects absolute paths that resolve outside", () => {
    expect(() => safeJoin("/tmp/site", "/etc/passwd")).toThrow(/escapes/);
  });
  it("rejects prefix-string false-match (outD vs outDir)", () => {
    expect(() => safeJoin("/tmp/outD", "../outDir/file")).toThrow(/escapes/);
  });
});

describe("validateOutDir", () => {
  it("accepts a normal dist directory inside cwd", () => {
    expect(() => validateOutDir("/tmp/site/dist", "/tmp/site")).not.toThrow();
  });
  it("accepts cwd itself (edge case but legal)", () => {
    expect(() => validateOutDir("/tmp/site", "/tmp/site")).not.toThrow();
  });
  it("rejects empty string", () => {
    expect(() => validateOutDir("", "/tmp")).toThrow();
  });
  it("rejects non-string", () => {
    expect(() => validateOutDir(5 as unknown as string, "/tmp")).toThrow();
  });
  it("rejects /etc", () => {
    expect(() => validateOutDir("/etc", "/etc")).toThrow(/system directory/);
  });
  it("rejects / (root)", () => {
    expect(() => validateOutDir("/", "/")).toThrow(/system directory/);
  });
  it("rejects paths outside cwd", () => {
    expect(() => validateOutDir("/tmp/site/dist", "/home/user/proj")).toThrow(/inside the working directory/);
  });
  it("rejects parent-of-cwd output dir", () => {
    expect(() => validateOutDir("/tmp", "/tmp/site")).toThrow(/inside the working directory/);
  });
  it("rejects Windows C:\\ root (case-insensitive)", () => {
    expect(() => validateOutDir("c:\\", "/tmp")).toThrow(/system directory/);
  });
  it("rejects C:\\Windows (case-insensitive)", () => {
    expect(() => validateOutDir("C:\\WINDOWS", "/tmp")).toThrow(/system directory/);
  });
  it("rejects prefix-string false-match (cwd /tmp/outD vs out /tmp/outDir)", () => {
    expect(() => validateOutDir("/tmp/outDir/dist", "/tmp/outD")).toThrow(/inside the working directory/);
  });
});
