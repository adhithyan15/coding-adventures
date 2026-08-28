/**
 * theme.test.ts — escapeHtml + renderHtmlDocument unit tests.
 */

import { describe, it, expect } from "vitest";
import { escapeHtml, renderHtmlDocument } from "../src/theme.js";

describe("escapeHtml", () => {
  it("escapes &, <, >, \", '", () => {
    expect(escapeHtml(`<a href="x" title='y'>&</a>`))
      .toBe("&lt;a href=&quot;x&quot; title=&#39;y&#39;&gt;&amp;&lt;/a&gt;");
  });

  it("leaves plain text alone", () => {
    expect(escapeHtml("Hello world")).toBe("Hello world");
  });

  it("escapes & first to avoid double-escaping &lt;", () => {
    expect(escapeHtml("&lt;")).toBe("&amp;lt;");
  });
});

describe("renderHtmlDocument", () => {
  it("wraps the body in a valid HTML5 doctype + head + main", () => {
    const html = renderHtmlDocument({
      title: "Hello",
      siteTitle: "",
      bodyHtml: "<p>Body.</p>\n",
    });
    expect(html).toMatch(/^<!DOCTYPE html>/);
    expect(html).toMatch(/<html lang="en">/);
    expect(html).toMatch(/<title>Hello<\/title>/);
    expect(html).toMatch(/<main>\n<p>Body\.<\/p>\n<\/main>/);
    expect(html.endsWith("\n")).toBe(true);
  });

  it("escapes the title", () => {
    const html = renderHtmlDocument({
      title: "Hello <script>",
      siteTitle: "",
      bodyHtml: "",
    });
    expect(html).toContain("<title>Hello &lt;script&gt;</title>");
    expect(html).not.toContain("<title>Hello <script>");
  });

  it("inlines only caller-supplied CSS", () => {
    const html = renderHtmlDocument({
      title: "T",
      siteTitle: "",
      styleCss: "p {\n  color: navy;\n}",
      bodyHtml: "",
    });
    expect(html).toContain("<style>");
    expect(html).toContain("color: navy");
    expect(html).toContain("</style>");
  });

  it("does not invent a renderer-owned theme", () => {
    const html = renderHtmlDocument({ title: "T", siteTitle: "", bodyHtml: "" });
    expect(html).not.toContain("<style>");
  });

  it("omits the <header> when siteTitle is empty", () => {
    const html = renderHtmlDocument({
      title: "T",
      siteTitle: "",
      bodyHtml: "",
    });
    expect(html).not.toContain("<header>");
  });

  it("emits a <header> with the linked site title when siteTitle is set", () => {
    const html = renderHtmlDocument({
      title: "T",
      siteTitle: "My Blog",
      bodyHtml: "",
    });
    expect(html).toContain(`<header><a href="/">My Blog</a></header>`);
  });

  it("escapes the site title in the header", () => {
    const html = renderHtmlDocument({
      title: "T",
      siteTitle: `Joe & "Friends"`,
      bodyHtml: "",
    });
    expect(html).toContain(`<header><a href="/">Joe &amp; &quot;Friends&quot;</a></header>`);
  });

  it("supports a project-page-safe header URL and trusted head tags", () => {
    const html = renderHtmlDocument({
      title: "T",
      siteTitle: "My Blog",
      siteHref: "https://example.com/project/blog/index.html?a=1&b=2",
      headHtml: '<link rel="canonical" href="https://example.com/project/blog/post.html">',
      styleCss: "p { color: navy; }",
      bodyHtml: "",
    });
    expect(html).toContain('href="https://example.com/project/blog/index.html?a=1&amp;b=2"');
    expect(html).toContain('<link rel="canonical" href="https://example.com/project/blog/post.html">\n<style>');
  });

  it("advertises light/dark browser chrome only when requested", () => {
    const dark = renderHtmlDocument({
      title: "T", siteTitle: "", bodyHtml: "", supportsDarkMode: true,
    });
    const light = renderHtmlDocument({ title: "T", siteTitle: "", bodyHtml: "" });
    expect(dark).toContain('<meta name="color-scheme" content="light dark">');
    expect(light).not.toContain('name="color-scheme"');
  });

  it("includes responsive viewport meta + UTF-8 charset", () => {
    const html = renderHtmlDocument({ title: "T", siteTitle: "", bodyHtml: "" });
    expect(html).toContain(`<meta charset="utf-8">`);
    expect(html).toContain(`<meta name="viewport" content="width=device-width,initial-scale=1">`);
  });
});
