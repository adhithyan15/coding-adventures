/**
 * builder.test.ts — `buildSidebar` integration tests.
 */

import { describe, it, expect } from "vitest";
import { buildSidebar } from "../src/index.js";
import type { PageInput, SidebarEntry } from "../src/index.js";

/** Convenience helper: page with empty frontmatter. */
function p(path: string, fm: Record<string, unknown> = {}): PageInput {
  return { path, frontmatter: fm };
}

/** Drop position/path/kind details for compact structural assertions. */
function shape(entries: readonly SidebarEntry[]): unknown {
  return entries.map((e) => {
    if (e.kind === "page") return { kind: "page", label: e.label };
    return { kind: "group", label: e.label, children: shape(e.children) };
  });
}

// ─────────────────────────────────────────────────────────────────────
// Degenerate inputs
// ─────────────────────────────────────────────────────────────────────

describe("buildSidebar — degenerate inputs", () => {
  it("empty array → []", () => {
    expect(buildSidebar([])).toEqual([]);
  });
  it("single root page", () => {
    const result = buildSidebar([p("intro.md")]);
    expect(result).toEqual([
      { kind: "page", label: "Intro", path: "intro.md", position: null },
    ]);
  });
});

// ─────────────────────────────────────────────────────────────────────
// Ordering
// ─────────────────────────────────────────────────────────────────────

describe("buildSidebar — ordering", () => {
  it("by sidebar_position ascending", () => {
    const result = buildSidebar([
      p("c.md", { sidebar_position: 3 }),
      p("a.md", { sidebar_position: 1 }),
      p("b.md", { sidebar_position: 2 }),
    ]);
    expect(result.map((e) => e.label)).toEqual(["A", "B", "C"]);
  });
  it("alphabetical fallback when no positions", () => {
    const result = buildSidebar([
      p("zebra.md"),
      p("apple.md"),
      p("banana.md"),
    ]);
    expect(result.map((e) => e.label)).toEqual(["Apple", "Banana", "Zebra"]);
  });
  it("positioned entries before unpositioned", () => {
    const result = buildSidebar([
      p("apple.md"),                            // null position
      p("banana.md", { sidebar_position: 1 }),
      p("zebra.md"),                            // null
      p("cherry.md", { sidebar_position: 2 }),
    ]);
    expect(result.map((e) => e.label)).toEqual(["Banana", "Cherry", "Apple", "Zebra"]);
  });
  it("tiebreak among same-position entries is alphabetical", () => {
    const result = buildSidebar([
      p("zulu.md", { sidebar_position: 1 }),
      p("alpha.md", { sidebar_position: 1 }),
    ]);
    expect(result.map((e) => e.label)).toEqual(["Alpha", "Zulu"]);
  });
});

// ─────────────────────────────────────────────────────────────────────
// Labels
// ─────────────────────────────────────────────────────────────────────

describe("buildSidebar — labels", () => {
  it("title overrides slug", () => {
    const result = buildSidebar([p("setup.md", { title: "Initial Setup" })]);
    expect(result[0].label).toBe("Initial Setup");
  });
  it("sidebar_label overrides title", () => {
    const result = buildSidebar([
      p("setup.md", { title: "Initial Setup", sidebar_label: "Setup" }),
    ]);
    expect(result[0].label).toBe("Setup");
  });
  it("empty sidebar_label falls back to title", () => {
    const result = buildSidebar([
      p("setup.md", { title: "Initial Setup", sidebar_label: "" }),
    ]);
    expect(result[0].label).toBe("Initial Setup");
  });
  it("empty title falls back to humanised slug", () => {
    const result = buildSidebar([p("getting-started.md", { title: "" })]);
    expect(result[0].label).toBe("Getting Started");
  });
  it("non-string title is ignored", () => {
    const result = buildSidebar([p("intro.md", { title: 42 })]);
    expect(result[0].label).toBe("Intro");
  });
  it("acronym slug → humanised", () => {
    const result = buildSidebar([p("api.md")]);
    expect(result[0].label).toBe("API");
  });
});

// ─────────────────────────────────────────────────────────────────────
// Drafts
// ─────────────────────────────────────────────────────────────────────

describe("buildSidebar — drafts", () => {
  it("draft: true pages are skipped", () => {
    const result = buildSidebar([
      p("public.md"),
      p("draft.md", { draft: true }),
    ]);
    expect(result.map((e) => e.label)).toEqual(["Public"]);
  });
  it("draft: false pages are NOT skipped", () => {
    const result = buildSidebar([p("public.md", { draft: false })]);
    expect(result).toHaveLength(1);
  });
  it("draft: 'true' (string) is NOT a draft (strict boolean check)", () => {
    const result = buildSidebar([p("public.md", { draft: "true" })]);
    expect(result).toHaveLength(1);
  });
  it("a group whose pages are all drafts collapses (no children → no group)", () => {
    const result = buildSidebar([
      p("guide/a.md", { draft: true }),
      p("guide/b.md", { draft: true }),
      p("intro.md"),
    ]);
    // No "Guide" group should appear.
    expect(shape(result)).toEqual([{ kind: "page", label: "Intro" }]);
  });
});

// ─────────────────────────────────────────────────────────────────────
// Grouping
// ─────────────────────────────────────────────────────────────────────

describe("buildSidebar — grouping", () => {
  it("single-level directory", () => {
    const result = buildSidebar([
      p("guide/setup.md", { sidebar_position: 1 }),
      p("guide/api.md", { sidebar_position: 2 }),
    ]);
    expect(shape(result)).toEqual([
      {
        kind: "group",
        label: "Guide",
        children: [
          { kind: "page", label: "Setup" },
          { kind: "page", label: "API" },
        ],
      },
    ]);
  });
  it("mixed root pages and groups (unpositioned group sinks below positioned pages)", () => {
    // The Guide group has no position (no index.md with sidebar_position),
    // so it sorts at +Infinity — AFTER Conclusion (99).  This is the
    // documented behaviour: positioned entries first (by position
    // ascending), unpositioned entries last (alphabetical among
    // themselves).
    const result = buildSidebar([
      p("intro.md", { sidebar_position: 1 }),
      p("guide/setup.md"),
      p("conclusion.md", { sidebar_position: 99 }),
    ]);
    expect(shape(result)).toEqual([
      { kind: "page", label: "Intro" },
      { kind: "page", label: "Conclusion" },
      { kind: "group", label: "Guide", children: [{ kind: "page", label: "Setup" }] },
    ]);
  });
  it("deeply nested groups", () => {
    const result = buildSidebar([
      p("a/b/c/leaf.md"),
    ]);
    expect(shape(result)).toEqual([
      {
        kind: "group",
        label: "A",
        children: [
          {
            kind: "group",
            label: "B",
            children: [
              {
                kind: "group",
                label: "C",
                children: [{ kind: "page", label: "Leaf" }],
              },
            ],
          },
        ],
      },
    ]);
  });
});

// ─────────────────────────────────────────────────────────────────────
// Index pages
// ─────────────────────────────────────────────────────────────────────

describe("buildSidebar — index pages", () => {
  it("group's index.md becomes the group's destination", () => {
    const result = buildSidebar([
      p("guide/index.md", { title: "User Guide", sidebar_position: 5 }),
      p("guide/setup.md", { sidebar_position: 1 }),
    ]);
    const grp = result[0];
    expect(grp.kind).toBe("group");
    if (grp.kind === "group") {
      expect(grp.label).toBe("User Guide");
      expect(grp.path).toBe("guide/index.md");
      expect(grp.position).toBe(5);
      expect(grp.children).toHaveLength(1);
      expect(grp.children[0].label).toBe("Setup");
    }
  });
  it("group without index has null path", () => {
    const result = buildSidebar([p("guide/a.md")]);
    const grp = result[0];
    if (grp.kind === "group") {
      expect(grp.path).toBeNull();
      expect(grp.position).toBeNull();
    }
  });
  it("index page is NOT listed among children", () => {
    const result = buildSidebar([
      p("guide/index.md"),
      p("guide/a.md"),
      p("guide/b.md"),
    ]);
    const grp = result[0];
    if (grp.kind === "group") {
      expect(grp.children.map((c) => c.label)).toEqual(["A", "B"]);
    }
  });
  it("root index.md becomes a top-level page (or group's own metadata)", () => {
    const result = buildSidebar([
      p("index.md", { title: "Home" }),
      p("intro.md"),
    ]);
    // Root index is on the synthetic root group, which has no
    // representation in the output — only its `children` are
    // returned.  But the index DOES live somewhere; for now it
    // becomes the root-level "Home" entry by having parts=[].
    // Our implementation treats a root index as living on the
    // root TrieNode, so it doesn't appear as a page entry — it
    // only contributes metadata to the implicit root group which
    // we don't surface.  Document this clearly via the test.
    //
    // For the user-facing result, only `intro.md` appears at top
    // level (the root index page is reachable via "/" in the URL
    // structure but isn't a sidebar entry).
    expect(shape(result)).toEqual([{ kind: "page", label: "Intro" }]);
  });
  it("duplicate index for same directory throws", () => {
    expect(() => {
      buildSidebar([
        p("guide/index.md"),
        p("guide/index.mdx"),
      ]);
    }).toThrow(/duplicate index page/);
  });
});

// ─────────────────────────────────────────────────────────────────────
// Duplicate detection
// ─────────────────────────────────────────────────────────────────────

describe("buildSidebar — duplicate detection", () => {
  it("two pages at same path throw", () => {
    expect(() => {
      buildSidebar([p("intro.md"), p("intro.mdx")]);
    }).toThrow(/duplicate page/);
  });
  it("two pages with different extensions at same slug throw", () => {
    expect(() => {
      buildSidebar([p("guide/setup.md"), p("guide/setup.html")]);
    }).toThrow(/duplicate page/);
  });
  it("root path '/' is a slugless non-index page and throws", () => {
    // splitAndClean("/") → [], isIndex stays false (no "index"
    // segment).  insertIntoTrie defensively rejects this — a page
    // with no slug has no place in the tree.
    expect(() => {
      buildSidebar([p("/")]);
    }).toThrow(/no slug/);
  });
});

describe("buildSidebar — root index page", () => {
  it("a root index.md becomes the root group's metadata (not a visible entry)", () => {
    // Index pages contribute their label/path/position to the
    // group they belong to.  The root group itself isn't surfaced
    // in the output (we return root.children), so the root index
    // page's metadata is currently unsurfaced — documented as a
    // v0 simplification.  This test pins the behaviour so any
    // future change is intentional.
    const result = buildSidebar([
      p("index.md", { title: "Welcome" }),
      p("intro.md"),
    ]);
    expect(result.map((e) => e.label)).toEqual(["Intro"]);
  });
});

describe("buildSidebar — compareEntries tie-break on identical labels", () => {
  it("two pages with identical title overrides at same dir sort stably", () => {
    // Two different slugs both get the same title via override.
    // Same null position, same label → compareEntries returns 0,
    // exercises the equal-label branch of the comparator.
    const result = buildSidebar([
      p("a.md", { title: "Shared", sidebar_position: 1 }),
      p("b.md", { title: "Shared", sidebar_position: 1 }),
    ]);
    expect(result).toHaveLength(2);
    expect(result.every((e) => e.label === "Shared")).toBe(true);
  });
});

// ─────────────────────────────────────────────────────────────────────
// Root prefix
// ─────────────────────────────────────────────────────────────────────

describe("buildSidebar — root prefix", () => {
  it("strips matching root (no positions → alphabetical by label)", () => {
    // Both entries are unpositioned, so they sort alphabetical:
    // "Guide" (group) < "Intro" (page).
    const result = buildSidebar(
      [
        p("docs/intro.md"),
        p("docs/guide/setup.md"),
      ],
      { root: "docs" },
    );
    expect(shape(result)).toEqual([
      { kind: "group", label: "Guide", children: [{ kind: "page", label: "Setup" }] },
      { kind: "page", label: "Intro" },
    ]);
  });
  it("skips pages outside root", () => {
    const result = buildSidebar(
      [
        p("docs/intro.md"),
        p("blog/post-1.md"),  // outside docs
      ],
      { root: "docs" },
    );
    expect(result.map((e) => e.label)).toEqual(["Intro"]);
  });
  it("empty root option behaves as no-op", () => {
    const result = buildSidebar([p("intro.md")], { root: "" });
    expect(result).toHaveLength(1);
  });
});

// ─────────────────────────────────────────────────────────────────────
// Frontmatter robustness
// ─────────────────────────────────────────────────────────────────────

describe("buildSidebar — frontmatter robustness", () => {
  it("non-numeric sidebar_position → null", () => {
    const result = buildSidebar([p("a.md", { sidebar_position: "1" as unknown as number })]);
    expect(result[0].position).toBeNull();
  });
  it("NaN sidebar_position → null", () => {
    const result = buildSidebar([p("a.md", { sidebar_position: NaN })]);
    expect(result[0].position).toBeNull();
  });
  it("Infinity sidebar_position → null", () => {
    const result = buildSidebar([p("a.md", { sidebar_position: Infinity })]);
    expect(result[0].position).toBeNull();
  });
  it("negative sidebar_position respected (sorts before positive)", () => {
    const result = buildSidebar([
      p("a.md", { sidebar_position: 1 }),
      p("b.md", { sidebar_position: -5 }),
    ]);
    expect(result.map((e) => e.label)).toEqual(["B", "A"]);
  });
  it("frontmatter with arbitrary unknown keys is harmless", () => {
    const result = buildSidebar([p("a.md", { custom_field: { nested: true }, tags: ["x"] })]);
    expect(result).toHaveLength(1);
    expect(result[0].label).toBe("A");
  });
  it("__proto__ as a frontmatter key is harmless (own-property lookup)", () => {
    const result = buildSidebar([p("a.md", { __proto__: { title: "Polluted" } } as Record<string, unknown>)]);
    expect(result[0].label).toBe("A");
  });
});

// ─────────────────────────────────────────────────────────────────────
// Determinism + immutability
// ─────────────────────────────────────────────────────────────────────

describe("buildSidebar — determinism + immutability", () => {
  it("same input → identical output", () => {
    const input: PageInput[] = [
      p("guide/index.md", { title: "Guide", sidebar_position: 1 }),
      p("guide/setup.md", { sidebar_position: 1 }),
      p("intro.md", { sidebar_position: 0 }),
    ];
    const a = JSON.stringify(buildSidebar(input));
    const b = JSON.stringify(buildSidebar(input));
    expect(a).toBe(b);
  });
  it("does not mutate input array", () => {
    const input = [p("a.md"), p("b.md")];
    const snapshot = JSON.stringify(input);
    buildSidebar(input);
    expect(JSON.stringify(input)).toBe(snapshot);
  });
  it("does not mutate input frontmatter objects", () => {
    const fm = { title: "T", sidebar_position: 1 };
    buildSidebar([{ path: "a.md", frontmatter: fm }]);
    expect(fm).toEqual({ title: "T", sidebar_position: 1 });
  });
  it("output is JSON-safe (no AST, no Date, no symbol)", () => {
    const result = buildSidebar([p("a.md", { title: "Hi", sidebar_position: 1 })]);
    expect(() => JSON.parse(JSON.stringify(result))).not.toThrow();
  });
});

// ─────────────────────────────────────────────────────────────────────
// Realistic scenario
// ─────────────────────────────────────────────────────────────────────

describe("buildSidebar — realistic scenario", () => {
  it("handles a typical docs site", () => {
    const result = buildSidebar([
      p("intro.md", { sidebar_position: 1 }),
      p("guide/index.md", { title: "Guide", sidebar_position: 2 }),
      p("guide/setup.md", { sidebar_position: 1 }),
      p("guide/configuration.md", { sidebar_position: 2 }),
      p("guide/draft-feature.md", { draft: true }),
      p("api/index.md", { title: "API Reference", sidebar_position: 3 }),
      p("api/endpoints.md", { sidebar_position: 1 }),
      p("api/authentication.md", { sidebar_position: 2 }),
      p("changelog.md", { sidebar_position: 99 }),
    ]);
    expect(shape(result)).toEqual([
      { kind: "page", label: "Intro" },
      {
        kind: "group",
        label: "Guide",
        children: [
          { kind: "page", label: "Setup" },
          { kind: "page", label: "Configuration" },
        ],
      },
      {
        kind: "group",
        label: "API Reference",
        children: [
          { kind: "page", label: "Endpoints" },
          { kind: "page", label: "Authentication" },
        ],
      },
      { kind: "page", label: "Changelog" },
    ]);
  });
});
