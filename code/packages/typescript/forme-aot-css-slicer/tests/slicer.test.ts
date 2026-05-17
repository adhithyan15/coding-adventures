/**
 * slicer.test.ts — per-page slicing semantics + fingerprinting.
 */

import { describe, it, expect } from "vitest";
import { createHash } from "node:crypto";
import {
  emptyStyleDocument, styleRuleId, sel,
  type StyleDocument,
} from "@coding-adventures/forme-style-ir";
import {
  slicePerPage, defaultScopePrefix,
  type PageSlice,
} from "../src/index.js";

// ─── Fixture ─────────────────────────────────────────────────────────────

function fixture(): StyleDocument {
  return {
    ...emptyStyleDocument(),
    tokens: {
      ...emptyStyleDocument().tokens,
      colors: { text: { kind: "rgb", r: 31, g: 35, b: 40 } },
    },
    rules: [
      {
        id: styleRuleId("body"),
        selector: sel.type("paragraph"),
        properties: [
          { kind: "color", value: { kind: "token-ref", path: "colors.text" } },
        ],
      },
      {
        id: styleRuleId("headline"),
        selector: { kind: "node-type-level", level: 1 },
        properties: [
          { kind: "color", value: { kind: "named", name: "black" } },
        ],
      },
      {
        id: styleRuleId("nav"),
        selector: { kind: "tag", tag: "nav" },
        properties: [
          { kind: "color", value: { kind: "named", name: "tomato" } },
        ],
      },
    ],
  };
}

// ─── Tests ───────────────────────────────────────────────────────────────

describe("slicePerPage — basic slicing", () => {
  it("emits one artefact per page", () => {
    const pages: PageSlice[] = [
      { id: "/a.html", usedRuleIds: [styleRuleId("body")] },
      { id: "/b.html", usedRuleIds: [styleRuleId("body"), styleRuleId("headline")] },
    ];
    const { artefacts } = slicePerPage(fixture(), pages, { activeContexts: [] });
    expect(artefacts.size).toBe(2);
    expect(artefacts.has("/a.html")).toBe(true);
    expect(artefacts.has("/b.html")).toBe(true);
  });

  it("emittedRules reflects per-page usedRuleIds", () => {
    const pages: PageSlice[] = [
      { id: "/a.html", usedRuleIds: [styleRuleId("body")] },
      { id: "/b.html", usedRuleIds: [styleRuleId("body"), styleRuleId("headline"), styleRuleId("nav")] },
    ];
    const { artefacts } = slicePerPage(fixture(), pages, { activeContexts: [] });
    expect(artefacts.get("/a.html")!.emittedRules).toEqual(["body"]);
    expect([...artefacts.get("/b.html")!.emittedRules].sort()).toEqual(["body", "headline", "nav"]);
  });

  it("scope is applied per page (each artefact's CSS has its own #p-… prefix)", () => {
    const pages: PageSlice[] = [
      { id: "/a.html", usedRuleIds: [styleRuleId("body")] },
      { id: "/b.html", usedRuleIds: [styleRuleId("body")] },
    ];
    const { artefacts } = slicePerPage(fixture(), pages, { activeContexts: [] });
    const a = artefacts.get("/a.html")!;
    const b = artefacts.get("/b.html")!;
    expect(a.css).toContain(defaultScopePrefix("/a.html"));
    expect(b.css).toContain(defaultScopePrefix("/b.html"));
    // Different scopes mean the CSS strings differ even though they
    // produce the same unscoped body.
    expect(a.css).not.toBe(b.css);
  });

  it("byteSize matches the actual UTF-8 byte length of the CSS", () => {
    const pages: PageSlice[] = [
      { id: "/a.html", usedRuleIds: [styleRuleId("body")] },
    ];
    const { artefacts } = slicePerPage(fixture(), pages, { activeContexts: [] });
    const a = artefacts.get("/a.html")!;
    expect(a.byteSize).toBe(Buffer.byteLength(a.css, "utf8"));
  });

  it("empty page (no usedRuleIds) produces an empty CSS string", () => {
    const pages: PageSlice[] = [{ id: "/empty.html", usedRuleIds: [] }];
    const { artefacts } = slicePerPage(fixture(), pages, { activeContexts: [] });
    const a = artefacts.get("/empty.html")!;
    expect(a.css).toBe("");
    expect(a.byteSize).toBe(0);
    expect(a.emittedRules).toEqual([]);
  });
});

describe("slicePerPage — content-addressed sha256", () => {
  it("pages with identical usedRuleIds get identical sha256 (dedup-friendly)", () => {
    const pages: PageSlice[] = [
      { id: "/a.html", usedRuleIds: [styleRuleId("body")] },
      { id: "/b.html", usedRuleIds: [styleRuleId("body")] },   // same content
    ];
    const { artefacts } = slicePerPage(fixture(), pages, { activeContexts: [] });
    expect(artefacts.get("/a.html")!.sha256)
      .toBe(artefacts.get("/b.html")!.sha256);
  });

  it("pages with different usedRuleIds get different sha256", () => {
    const pages: PageSlice[] = [
      { id: "/a.html", usedRuleIds: [styleRuleId("body")] },
      { id: "/b.html", usedRuleIds: [styleRuleId("body"), styleRuleId("headline")] },
    ];
    const { artefacts } = slicePerPage(fixture(), pages, { activeContexts: [] });
    expect(artefacts.get("/a.html")!.sha256)
      .not.toBe(artefacts.get("/b.html")!.sha256);
  });

  it("sha256 is over the UNSCOPED CSS bytes", () => {
    // Provide a custom scopePrefix so we can verify the sha256
    // does NOT depend on the scope.
    const pages: PageSlice[] = [
      { id: "/a.html", usedRuleIds: [styleRuleId("body")] },
    ];
    const a = slicePerPage(fixture(), pages, { activeContexts: [] }).artefacts.get("/a.html")!;
    const b = slicePerPage(fixture(), pages, {
      activeContexts: [],
      scopePrefix: () => "#totally-different-scope-9",
    }).artefacts.get("/a.html")!;
    expect(a.sha256).toBe(b.sha256);
    // But the CSS itself differs.
    expect(a.css).not.toBe(b.css);
  });

  it("sha256 is stable across runs (reproducibility — FM03)", () => {
    const pages: PageSlice[] = [{ id: "/a.html", usedRuleIds: [styleRuleId("body")] }];
    const r1 = slicePerPage(fixture(), pages, { activeContexts: [] });
    const r2 = slicePerPage(fixture(), pages, { activeContexts: [] });
    expect(r1.artefacts.get("/a.html")!.sha256)
      .toBe(r2.artefacts.get("/a.html")!.sha256);
  });

  it("sha256 is 64 hex characters", () => {
    const pages: PageSlice[] = [{ id: "/a.html", usedRuleIds: [styleRuleId("body")] }];
    const { artefacts } = slicePerPage(fixture(), pages, { activeContexts: [] });
    expect(artefacts.get("/a.html")!.sha256).toMatch(/^[0-9a-f]{64}$/);
  });
});

describe("defaultScopePrefix", () => {
  it("produces `#p-` + 8 hex chars", () => {
    const scope = defaultScopePrefix("/example.html");
    expect(scope).toMatch(/^#p-[0-9a-f]{8}$/);
  });

  it("is deterministic", () => {
    expect(defaultScopePrefix("/a.html")).toBe(defaultScopePrefix("/a.html"));
  });

  it("different page ids → different scopes (no obvious collisions on tiny corpus)", () => {
    const a = defaultScopePrefix("/a.html");
    const b = defaultScopePrefix("/b.html");
    const c = defaultScopePrefix("/c.html");
    expect(new Set([a, b, c]).size).toBe(3);
  });

  it("first 8 chars match the sha256 hash of the page id", () => {
    const pageId = "/page-with-strange-chars: 你好/\\";
    const scope = defaultScopePrefix(pageId);
    const expectedFirst8 = createHash("sha256").update(pageId, "utf8").digest("hex").slice(0, 8);
    expect(scope).toBe(`#p-${expectedFirst8}`);
  });

  it("survives hostile page ids without sanitisation issues (output is always [0-9a-f]+)", () => {
    // The scope is always [#][p][-] + hex.  Page id can be anything.
    for (const id of ["", " ", "\x00\x1b[31m", "<script>", `\\"; }`, "💥", "/" .repeat(10000)]) {
      const scope = defaultScopePrefix(id);
      expect(scope).toMatch(/^#p-[0-9a-f]{8}$/);
    }
  });
});

describe("slicePerPage — custom scopePrefix", () => {
  it("uses the caller's function in place of the default", () => {
    const pages: PageSlice[] = [
      { id: "/a.html", usedRuleIds: [styleRuleId("body")] },
    ];
    const { artefacts } = slicePerPage(fixture(), pages, {
      activeContexts: [],
      scopePrefix: (id) => `.scope-${id.replace(/[^a-z]/g, "")}`,
    });
    expect(artefacts.get("/a.html")!.css).toContain(".scope-ahtml");
  });

  it("a no-op scope (empty string) produces unscoped output (same as raw translateToCss)", () => {
    const pages: PageSlice[] = [
      { id: "/a.html", usedRuleIds: [styleRuleId("body")] },
    ];
    const { artefacts } = slicePerPage(fixture(), pages, {
      activeContexts: [],
      scopePrefix: () => "",
    });
    const css = artefacts.get("/a.html")!.css;
    // No prefix at all — the rule starts at column 0.
    expect(css.startsWith("paragraph {")).toBe(true);
  });
});

describe("slicePerPage — warnings propagate per page", () => {
  it("a page that uses a rule referencing an unresolved token-ref → warning", () => {
    const broken: StyleDocument = {
      ...fixture(),
      rules: [
        {
          id: styleRuleId("bad"),
          selector: sel.type("p"),
          properties: [
            { kind: "color", value: { kind: "token-ref", path: "colors.nope" } },
          ],
        },
      ],
    };
    const pages: PageSlice[] = [{ id: "/x.html", usedRuleIds: [styleRuleId("bad")] }];
    const { artefacts } = slicePerPage(broken, pages, { activeContexts: [] });
    const x = artefacts.get("/x.html")!;
    expect(x.warnings.length).toBeGreaterThan(0);
  });

  it("warnings on one page do not bleed into another", () => {
    const broken: StyleDocument = {
      ...fixture(),
      rules: [
        ...fixture().rules,
        {
          id: styleRuleId("bad"),
          selector: sel.type("p"),
          properties: [
            { kind: "color", value: { kind: "token-ref", path: "colors.nope" } },
          ],
        },
      ],
    };
    const pages: PageSlice[] = [
      { id: "/good.html", usedRuleIds: [styleRuleId("body")] },
      { id: "/bad.html",  usedRuleIds: [styleRuleId("bad")] },
    ];
    const { artefacts } = slicePerPage(broken, pages, { activeContexts: [] });
    expect(artefacts.get("/good.html")!.warnings).toEqual([]);
    expect(artefacts.get("/bad.html")!.warnings.length).toBeGreaterThan(0);
  });
});

describe("slicePerPage — page iteration order is preserved", () => {
  it("artefacts Map iteration matches input array order", () => {
    const pages: PageSlice[] = [
      { id: "/c.html", usedRuleIds: [styleRuleId("body")] },
      { id: "/a.html", usedRuleIds: [styleRuleId("body")] },
      { id: "/b.html", usedRuleIds: [styleRuleId("body")] },
    ];
    const { artefacts } = slicePerPage(fixture(), pages, { activeContexts: [] });
    expect([...artefacts.keys()]).toEqual(["/c.html", "/a.html", "/b.html"]);
  });
});

describe("slicePerPage — scope isolation (no cross-page selector collisions)", () => {
  it("two pages using the same rule emit selectors prefixed with DIFFERENT scopes", () => {
    const pages: PageSlice[] = [
      { id: "/page-1", usedRuleIds: [styleRuleId("body")] },
      { id: "/page-2", usedRuleIds: [styleRuleId("body")] },
    ];
    const { artefacts } = slicePerPage(fixture(), pages, { activeContexts: [] });
    const a = artefacts.get("/page-1")!.css;
    const b = artefacts.get("/page-2")!.css;

    const scopeA = defaultScopePrefix("/page-1");
    const scopeB = defaultScopePrefix("/page-2");

    expect(a).toContain(scopeA);
    expect(a).not.toContain(scopeB);
    expect(b).toContain(scopeB);
    expect(b).not.toContain(scopeA);
  });
});

describe("slicePerPage — empty pages array", () => {
  it("returns an empty artefacts map", () => {
    const { artefacts } = slicePerPage(fixture(), [], { activeContexts: [] });
    expect(artefacts.size).toBe(0);
  });
});
