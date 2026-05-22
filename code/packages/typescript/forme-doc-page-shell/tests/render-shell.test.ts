/**
 * render-shell.test.ts — main renderPageShell integration tests.
 */

import { describe, it, expect } from "vitest";
import { renderPageShell } from "../src/index.js";
import type { PageShellInput } from "../src/index.js";

/** Minimal valid input — builder for terser tests. */
function input(overrides: Partial<PageShellInput> = {}): PageShellInput {
  return {
    site: { title: "Site" },
    page: { title: "Page", body: "<p>body</p>" },
    sidebar: [],
    ...overrides,
  };
}

// ─────────────────────────────────────────────────────────────────────
// head
// ─────────────────────────────────────────────────────────────────────

describe("renderPageShell — head", () => {
  it("includes charset + viewport + title", () => {
    const { head } = renderPageShell(input());
    expect(head).toContain('<meta charset="utf-8">');
    expect(head).toContain('<meta name="viewport"');
    expect(head).toContain("<title>Page | Site</title>");
  });
  it("includes description when present", () => {
    const { head } = renderPageShell(input({
      page: { title: "P", body: "", description: "A great page" },
    }));
    expect(head).toContain('<meta name="description" content="A great page">');
  });
  it("omits description when absent or empty", () => {
    const { head } = renderPageShell(input());
    expect(head).not.toContain("name=\"description\"");
    const { head: head2 } = renderPageShell(input({
      page: { title: "P", body: "", description: "" },
    }));
    expect(head2).not.toContain("name=\"description\"");
  });
  it("includes headExtra raw (caller's responsibility to escape)", () => {
    const { head } = renderPageShell(input({
      options: { headExtra: '<link rel="canonical" href="/x">' },
    }));
    expect(head).toContain('<link rel="canonical" href="/x">');
  });
  it("escapes XSS in title", () => {
    const { head } = renderPageShell(input({
      page: { title: "<script>", body: "" },
    }));
    expect(head).toContain("&lt;script&gt;");
    expect(head).not.toMatch(/<title>[^|]*<script>/);
  });
  it("escapes XSS in description", () => {
    const { head } = renderPageShell(input({
      page: { title: "P", body: "", description: '" onload="alert(1)' },
    }));
    expect(head).toContain("&quot;");
    expect(head).not.toContain('onload="');
  });
});

// ─────────────────────────────────────────────────────────────────────
// body — header
// ─────────────────────────────────────────────────────────────────────

describe("renderPageShell — body header", () => {
  it("brand link defaults to '/'", () => {
    const { body } = renderPageShell(input());
    expect(body).toContain('<a class="brand" href="/">Site</a>');
  });
  it("brand link honours homeUrl", () => {
    const { body } = renderPageShell(input({
      site: { title: "Site", homeUrl: "/docs/" },
    }));
    expect(body).toContain('<a class="brand" href="/docs/">Site</a>');
  });
  it("GitHub link appears when githubUrl is set", () => {
    const { body } = renderPageShell(input({
      site: { title: "Site", githubUrl: "https://github.com/me/repo" },
    }));
    expect(body).toContain('class="github" href="https://github.com/me/repo"');
  });
  it("search input appears when searchPlaceholder is set", () => {
    const { body } = renderPageShell(input({
      options: { searchPlaceholder: "Search docs…" },
    }));
    expect(body).toContain('class="search"');
    expect(body).toContain('placeholder="Search docs…"');
  });
  it("escapes site title", () => {
    const { body } = renderPageShell(input({
      site: { title: "<script>" },
    }));
    expect(body).toContain("&lt;script&gt;");
  });
  it("rejects javascript: in githubUrl", () => {
    const { body } = renderPageShell(input({
      site: { title: "Site", githubUrl: "javascript:alert(1)" },
    }));
    expect(body).toContain('class="github" href="#"');
  });
});

// ─────────────────────────────────────────────────────────────────────
// body — main
// ─────────────────────────────────────────────────────────────────────

describe("renderPageShell — body main", () => {
  it("includes the trusted page body verbatim", () => {
    const { body } = renderPageShell(input({
      page: { title: "P", body: "<p>Trusted <em>markup</em></p>" },
    }));
    expect(body).toContain("<p>Trusted <em>markup</em></p>");
  });
  it("renders h1 with escaped page title", () => {
    const { body } = renderPageShell(input({
      page: { title: "P <em>not real</em>", body: "" },
    }));
    expect(body).toContain("<h1>P &lt;em&gt;not real&lt;/em&gt;</h1>");
  });
  it("omits breadcrumbs when absent", () => {
    const { body } = renderPageShell(input());
    expect(body).not.toContain("<ol class=\"breadcrumbs\"");
  });
  it("renders breadcrumbs when present", () => {
    const { body } = renderPageShell(input({
      page: {
        title: "P",
        body: "",
        breadcrumbs: [
          { label: "Home", href: "/" },
          { label: "P", href: "/p" },
        ],
      },
    }));
    expect(body).toContain('<ol class="breadcrumbs"');
    expect(body).toContain('<a href="/">Home</a>');
  });
  it("omits TOC when absent", () => {
    const { body } = renderPageShell(input());
    expect(body).not.toContain('<aside class="toc"');
  });
  it("renders TOC when present", () => {
    const { body } = renderPageShell(input({
      page: {
        title: "P",
        body: "",
        toc: [{ text: "S", id: "s", level: 2, children: [] }],
      },
    }));
    expect(body).toContain('<aside class="toc"');
    expect(body).toContain('href="#s"');
  });
});

// ─────────────────────────────────────────────────────────────────────
// body — sidebar
// ─────────────────────────────────────────────────────────────────────

describe("renderPageShell — body sidebar", () => {
  it("renders sidebar entries", () => {
    const { body } = renderPageShell(input({
      sidebar: [{ kind: "page", label: "Intro", path: "/intro" }],
    }));
    expect(body).toContain('<a href="/intro">Intro</a>');
  });
  it("currentPath highlights active sidebar entry", () => {
    const { body } = renderPageShell(input({
      sidebar: [{ kind: "page", label: "Intro", path: "/intro" }],
      options: { currentPath: "/intro" },
    }));
    expect(body).toContain('aria-current="page"');
  });
});

// ─────────────────────────────────────────────────────────────────────
// body — footer
// ─────────────────────────────────────────────────────────────────────

describe("renderPageShell — body footer", () => {
  it("empty footer still emitted", () => {
    const { body } = renderPageShell(input());
    expect(body).toContain('<footer class="site-footer"></footer>');
  });
  it("version label when set", () => {
    const { body } = renderPageShell(input({
      site: { title: "Site", version: "1.2.3" },
    }));
    expect(body).toContain('<span class="version">v1.2.3</span>');
  });
  it("edit link when set", () => {
    const { body } = renderPageShell(input({
      page: { title: "P", body: "", editUrl: "https://github.com/me/r/edit/main/page.md" },
    }));
    expect(body).toContain('class="edit" href="https://github.com/me/r/edit/main/page.md"');
  });
  it("copyright when set", () => {
    const { body } = renderPageShell(input({
      site: { title: "Site", copyright: "© 2026 Me" },
    }));
    expect(body).toContain('<span class="copyright">© 2026 Me</span>');
  });
  it("all three when all set", () => {
    const { body } = renderPageShell(input({
      site: { title: "Site", version: "1.0", copyright: "(c) Me" },
      page: { title: "P", body: "", editUrl: "/edit" },
    }));
    expect(body).toContain('class="version"');
    expect(body).toContain('class="edit"');
    expect(body).toContain('class="copyright"');
  });
  it("XSS in copyright is escaped", () => {
    const { body } = renderPageShell(input({
      site: { title: "Site", copyright: "<script>alert(1)</script>" },
    }));
    expect(body).toContain("&lt;script&gt;");
  });
  it("XSS in version is escaped", () => {
    const { body } = renderPageShell(input({
      site: { title: "Site", version: "<x>" },
    }));
    expect(body).toContain("&lt;x&gt;");
  });
});

// ─────────────────────────────────────────────────────────────────────
// Structure invariants
// ─────────────────────────────────────────────────────────────────────

describe("renderPageShell — structure", () => {
  it("emits both head and body chunks", () => {
    const out = renderPageShell(input());
    expect(typeof out.head).toBe("string");
    expect(typeof out.body).toBe("string");
    expect(out.head.length).toBeGreaterThan(0);
    expect(out.body.length).toBeGreaterThan(0);
  });
  it("body has header, layout, footer in order", () => {
    const { body } = renderPageShell(input());
    const headerIdx = body.indexOf("site-header");
    const layoutIdx = body.indexOf("class=\"layout\"");
    const footerIdx = body.indexOf("site-footer");
    expect(headerIdx).toBeGreaterThan(-1);
    expect(layoutIdx).toBeGreaterThan(headerIdx);
    expect(footerIdx).toBeGreaterThan(layoutIdx);
  });
  it("layout contains sidebar AND main", () => {
    const { body } = renderPageShell(input());
    const layoutStart = body.indexOf("class=\"layout\"");
    const layoutEnd = body.indexOf("class=\"site-footer\"");
    const layoutSection = body.slice(layoutStart, layoutEnd);
    expect(layoutSection).toContain("class=\"sidebar\"");
    expect(layoutSection).toContain("<main>");
  });
});

// ─────────────────────────────────────────────────────────────────────
// Determinism + immutability
// ─────────────────────────────────────────────────────────────────────

describe("renderPageShell — determinism", () => {
  it("same input → identical output", () => {
    const i = input({
      site: { title: "S", version: "1.0", githubUrl: "https://x.test" },
      page: { title: "P", body: "<p>b</p>", description: "d" },
      sidebar: [{ kind: "page", label: "X", path: "/x" }],
    });
    const a = renderPageShell(i);
    const b = renderPageShell(i);
    expect(a.head).toBe(b.head);
    expect(a.body).toBe(b.body);
  });
});

describe("renderPageShell — immutability", () => {
  it("does not mutate input", () => {
    const i = input({
      site: { title: "S", version: "1.0" },
      page: { title: "P", body: "<p>b</p>" },
      sidebar: [{ kind: "group", label: "G", path: "/g", children: [{ kind: "page", label: "X", path: "/x" }] }],
    });
    const snapshot = JSON.stringify(i);
    renderPageShell(i);
    expect(JSON.stringify(i)).toBe(snapshot);
  });
});
