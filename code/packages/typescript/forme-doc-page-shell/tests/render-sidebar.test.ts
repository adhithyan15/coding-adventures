/**
 * render-sidebar.test.ts — sidebar tree → HTML tests.
 */

import { describe, it, expect } from "vitest";
import { renderSidebar } from "../src/index.js";
import type { SidebarEntry } from "../src/index.js";

function page(label: string, path: string): SidebarEntry {
  return { kind: "page", label, path };
}
function group(label: string, path: string | null, children: SidebarEntry[]): SidebarEntry {
  return { kind: "group", label, path, children };
}

describe("renderSidebar — degenerate inputs", () => {
  it("empty → nav with empty list", () => {
    const html = renderSidebar([]);
    expect(html).toBe('<nav class="sidebar" aria-label="Site navigation"></nav>');
  });
});

describe("renderSidebar — pages", () => {
  it("single page entry", () => {
    const html = renderSidebar([page("Intro", "/intro")]);
    expect(html).toContain('<li><a href="/intro">Intro</a></li>');
  });
  it("multiple pages", () => {
    const html = renderSidebar([page("A", "/a"), page("B", "/b")]);
    expect(html).toContain('<li><a href="/a">A</a></li>');
    expect(html).toContain('<li><a href="/b">B</a></li>');
  });
  it("page with null path → non-clickable span (defensive)", () => {
    const entry: SidebarEntry = { kind: "page", label: "Orphan", path: null };
    expect(renderSidebar([entry])).toContain("<li><span>Orphan</span></li>");
  });
});

describe("renderSidebar — groups", () => {
  it("group with index page → clickable label", () => {
    const html = renderSidebar([group("Guide", "/guide", [page("Setup", "/guide/setup")])]);
    expect(html).toContain('<li class="group"><a href="/guide">Guide</a>');
    expect(html).toContain('<li><a href="/guide/setup">Setup</a></li>');
  });
  it("group without index → span (no link)", () => {
    const html = renderSidebar([group("Misc", null, [page("X", "/x")])]);
    expect(html).toContain('<li class="group"><span>Misc</span>');
  });
  it("nested groups recurse", () => {
    const html = renderSidebar([
      group("Outer", null, [group("Inner", null, [page("Deep", "/deep")])]),
    ]);
    expect(html).toContain('<li class="group"><span>Outer</span>');
    expect(html).toContain('<li class="group"><span>Inner</span>');
    expect(html).toContain('<li><a href="/deep">Deep</a></li>');
  });
});

describe("renderSidebar — active-page highlighting", () => {
  it("currentPath match adds aria-current='page'", () => {
    const html = renderSidebar([page("Intro", "/intro"), page("Setup", "/setup")], "/setup");
    expect(html).toContain('<a href="/setup" aria-current="page">Setup</a>');
    expect(html).toContain('<a href="/intro">Intro</a>'); // no aria
  });
  it("currentPath match on a group's index page", () => {
    const html = renderSidebar([group("Guide", "/guide", [])], "/guide");
    expect(html).toContain('<a href="/guide" aria-current="page">Guide</a>');
  });
  it("no currentPath → no aria-current anywhere", () => {
    const html = renderSidebar([page("A", "/a")]);
    expect(html).not.toContain("aria-current");
  });
});

describe("renderSidebar — defensive defaults", () => {
  it("group with undefined children (not just empty array)", () => {
    // The SidebarEntry type allows `children` to be undefined on
    // groups; the renderer defaults to [].  This exercises the
    // `entry.children ?? []` branch.
    const entry: SidebarEntry = { kind: "group", label: "Empty", path: null };
    const html = renderSidebar([entry]);
    expect(html).toContain('<li class="group"><span>Empty</span>');
  });
});

describe("renderSidebar — XSS defence", () => {
  it("escapes label", () => {
    const html = renderSidebar([page("<script>", "/x")]);
    expect(html).toContain("&lt;script&gt;");
    expect(html).not.toContain("<script>");
  });
  it("escapes path query strings", () => {
    const html = renderSidebar([page("Search", "/search?q=a&b")]);
    expect(html).toContain("/search?q=a&amp;b");
  });
  it("rejects javascript: URL", () => {
    const html = renderSidebar([page("Bad", "javascript:alert(1)")]);
    expect(html).toContain('<a href="#">Bad</a>');
    expect(html).not.toContain("javascript:");
  });
  it("group label with quotes is escaped", () => {
    const html = renderSidebar([group('say "hi"', null, [])]);
    expect(html).toContain("&quot;hi&quot;");
  });
});
