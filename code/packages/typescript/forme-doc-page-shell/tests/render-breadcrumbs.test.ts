/**
 * render-breadcrumbs.test.ts — breadcrumb trail → HTML tests.
 */

import { describe, it, expect } from "vitest";
import { renderBreadcrumbs } from "../src/index.js";

describe("renderBreadcrumbs — degenerate", () => {
  it("empty → empty string", () => {
    expect(renderBreadcrumbs([])).toBe("");
  });
});

describe("renderBreadcrumbs — basic", () => {
  it("single item is current page (no link)", () => {
    const html = renderBreadcrumbs([{ label: "Home", href: "/" }]);
    expect(html).toContain('<li aria-current="page"><span>Home</span></li>');
  });
  it("multiple items: last is current page, prior are links", () => {
    const html = renderBreadcrumbs([
      { label: "Home", href: "/" },
      { label: "Guide", href: "/guide" },
      { label: "Setup", href: "/guide/setup" },
    ]);
    expect(html).toContain('<li><a href="/">Home</a></li>');
    expect(html).toContain('<li><a href="/guide">Guide</a></li>');
    expect(html).toContain('<li aria-current="page"><span>Setup</span></li>');
  });
});

describe("renderBreadcrumbs — XSS defence", () => {
  it("escapes label", () => {
    const html = renderBreadcrumbs([{ label: "<script>", href: "/" }]);
    expect(html).toContain("&lt;script&gt;");
  });
  it("rejects javascript: href", () => {
    const html = renderBreadcrumbs([
      { label: "Bad", href: "javascript:alert(1)" },
      { label: "Now", href: "/" },
    ]);
    expect(html).toContain('<a href="#">Bad</a>');
  });
});
