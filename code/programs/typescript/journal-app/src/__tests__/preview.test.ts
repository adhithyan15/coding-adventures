/**
 * preview.test.ts — Tests for the GFM rendering pipeline wrapper.
 *
 * Verifies that renderPreview() correctly converts markdown strings to
 * HTML using the @coding-adventures/gfm pipeline with RELAXED sanitization.
 */

import { describe, it, expect } from "vitest";
import { renderPreview } from "../preview.js";

describe("renderPreview", () => {
  it("returns empty string for empty input", () => {
    expect(renderPreview("")).toBe("");
  });

  it("renders a heading as <h1>", () => {
    const html = renderPreview("# Hello");
    expect(html).toContain("<h1>");
    expect(html).toContain("Hello");
    expect(html).toContain("</h1>");
  });

  it("renders h2 through h6", () => {
    expect(renderPreview("## Two")).toContain("<h2>");
    expect(renderPreview("### Three")).toContain("<h3>");
    expect(renderPreview("#### Four")).toContain("<h4>");
    expect(renderPreview("##### Five")).toContain("<h5>");
    expect(renderPreview("###### Six")).toContain("<h6>");
  });

  it("renders bold text as <strong>", () => {
    const html = renderPreview("**bold**");
    expect(html).toContain("<strong>");
    expect(html).toContain("bold");
  });

  it("renders italic text as <em>", () => {
    const html = renderPreview("*italic*");
    expect(html).toContain("<em>");
    expect(html).toContain("italic");
  });

  it("renders an unordered list", () => {
    const html = renderPreview("- item one\n- item two");
    expect(html).toContain("<ul>");
    expect(html).toContain("<li>");
    expect(html).toContain("item one");
    expect(html).toContain("item two");
  });

  it("renders a code block as <pre><code>", () => {
    const html = renderPreview("```\nconst x = 1;\n```");
    expect(html).toContain("<pre>");
    expect(html).toContain("<code>");
    expect(html).toContain("const x = 1;");
  });

  it("renders inline code as <code>", () => {
    const html = renderPreview("`hello`");
    expect(html).toContain("<code>");
    expect(html).toContain("hello");
  });

  it("renders a link as <a>", () => {
    const html = renderPreview("[click](https://example.com)");
    expect(html).toContain("<a");
    expect(html).toContain('href="https://example.com"');
    expect(html).toContain("click");
  });

  it("renders a blockquote", () => {
    const html = renderPreview("> quoted text");
    expect(html).toContain("<blockquote>");
    expect(html).toContain("quoted text");
  });

  it("renders a paragraph", () => {
    const html = renderPreview("Just a paragraph.");
    expect(html).toContain("<p>");
    expect(html).toContain("Just a paragraph.");
  });
});
