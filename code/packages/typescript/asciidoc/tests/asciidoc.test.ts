/**
 * @coding-adventures/asciidoc — End-to-end toHtml tests
 *
 * These tests verify the full AsciiDoc → HTML pipeline: parse() +  render().
 * We test the public `toHtml()` function which combines both steps.
 */

import { describe, it, expect } from "vitest";
import { toHtml, parse, render } from "../src/index.js";

describe("toHtml — headings", () => {
  it("converts level-1 heading to <h1>", () => {
    const html = toHtml("= My Title\n");
    expect(html).toContain("<h1>");
    expect(html).toContain("My Title");
    expect(html).toContain("</h1>");
  });

  it("converts level-2 heading to <h2>", () => {
    const html = toHtml("== Section\n");
    expect(html).toContain("<h2>");
    expect(html).toContain("Section");
  });

  it("converts level-3 heading to <h3>", () => {
    const html = toHtml("=== Sub-section\n");
    expect(html).toContain("<h3>");
  });
});

describe("toHtml — paragraphs", () => {
  it("wraps paragraph in <p> tags", () => {
    const html = toHtml("Hello world.\n");
    expect(html).toContain("<p>");
    expect(html).toContain("Hello world.");
    expect(html).toContain("</p>");
  });

  it("produces two <p> elements for two paragraphs", () => {
    const html = toHtml("First.\n\nSecond.\n");
    const matches = html.match(/<p>/g);
    expect(matches).toHaveLength(2);
  });
});

describe("toHtml — strong and emphasis", () => {
  it("renders *bold* as <strong>", () => {
    const html = toHtml("Hello *bold* world.\n");
    expect(html).toContain("<strong>bold</strong>");
  });

  it("renders _italic_ as <em>", () => {
    const html = toHtml("Hello _italic_ world.\n");
    expect(html).toContain("<em>italic</em>");
  });

  it("renders **unconstrained bold** as <strong>", () => {
    const html = toHtml("**bold** text.\n");
    expect(html).toContain("<strong>");
  });
});

describe("toHtml — code", () => {
  it("renders `code span` as <code>", () => {
    const html = toHtml("Use `npm install` here.\n");
    expect(html).toContain("<code>npm install</code>");
  });

  it("renders delimited code block as <pre><code>", () => {
    const html = toHtml("----\nconst x = 1;\n----\n");
    expect(html).toContain("<pre>");
    expect(html).toContain("<code>");
    expect(html).toContain("const x = 1;");
  });

  it("includes language class for [source,js] blocks", () => {
    const html = toHtml("[source,js]\n----\nconsole.log('hi');\n----\n");
    expect(html).toContain("language-js");
  });
});

describe("toHtml — lists", () => {
  it("renders unordered list as <ul>", () => {
    const html = toHtml("* Alpha\n* Beta\n");
    expect(html).toContain("<ul>");
    expect(html).toContain("<li>");
    expect(html).toContain("Alpha");
    expect(html).toContain("Beta");
  });

  it("renders ordered list as <ol>", () => {
    const html = toHtml(". First\n. Second\n");
    expect(html).toContain("<ol>");
  });
});

describe("toHtml — blockquote", () => {
  it("renders ____ block as <blockquote>", () => {
    const html = toHtml("____\nQuoted text.\n____\n");
    expect(html).toContain("<blockquote>");
    expect(html).toContain("Quoted text.");
    expect(html).toContain("</blockquote>");
  });
});

describe("toHtml — thematic break", () => {
  it("renders ''' as <hr />", () => {
    const html = toHtml("'''\n");
    expect(html).toMatch(/<hr/);
  });
});

describe("toHtml — raw passthrough", () => {
  it("passes through raw HTML from ++++ block", () => {
    const html = toHtml("++++\n<div class=\"custom\">raw</div>\n++++\n");
    expect(html).toContain('<div class="custom">raw</div>');
  });
});

describe("parse + render API", () => {
  it("parse() and render() are re-exported", () => {
    const doc = parse("= Hello\n");
    const html = render(doc);
    expect(html).toContain("<h1>");
  });

  it("produces consistent output for empty input", () => {
    const html = toHtml("");
    expect(typeof html).toBe("string");
  });
});
