import { describe, expect, it } from "vitest";
import { parse } from "@coding-adventures/gfm-parser";
import type { DocumentNode } from "@coding-adventures/document-ast";
import {
  emptyStyleDocument,
  sel,
  styleRuleId,
  type Selector,
  type StyleDocument,
} from "@coding-adventures/forme-style-ir";
import { collectUsedStyle } from "../src/used-style.js";

function style(entries: readonly [string, Selector][]): StyleDocument {
  return {
    ...emptyStyleDocument(),
    rules: entries.map(([id, selector]) => ({
      id: styleRuleId(id),
      selector,
      properties: [{ kind: "color", value: { kind: "named", name: "navy" } }],
    })),
  };
}

describe("collectUsedStyle", () => {
  it("matches identity, structural, composition, position, shell, and tag selectors", () => {
    const document = parse([
      "# Title",
      "",
      "Intro with *emphasis*, **strong**, `code`, [link](https://example.com), and ![image](x.png).",
      "",
      "- one",
      "- two",
      "",
      "> Quote",
      "",
      "| A | B |",
      "| - | - |",
      "| 1 | 2 |",
      "",
    ].join("\n"));
    const rules = style([
      ["paragraph", sel.type("p")],
      ["heading", sel.heading(1)],
      ["main-role", sel.role("main")],
      ["frontmatter-tag", sel.tag("featured")],
      ["second-item", sel.nth(sel.type("li"), 1)],
      ["odd-item", sel.nth(sel.type("li"), { a: 2, b: 1 })],
      ["last-item", sel.nth(sel.type("li"), { a: 0, b: 1, fromEnd: true })],
      ["list-child", sel.childOf(sel.type("ul"), sel.type("li"))],
      ["quote-paragraph", sel.descendantOf(sel.type("blockquote"), sel.type("p"))],
      ["intro-after-title", sel.adjacent(sel.heading(1), sel.type("p"))],
      ["tagged-paragraph", sel.and(sel.type("p"), sel.tag("featured"))],
      ["table-or-title", sel.or(sel.type("table"), sel.heading(6))],
      ["not-table", sel.and(sel.type("p"), sel.not(sel.type("table")))],
      ["header-link", sel.descendantOf(sel.type("header"), sel.type("a"))],
      ["emphasis", sel.type("em")],
      ["strong", sel.type("strong")],
      ["code", sel.type("code")],
      ["link", sel.type("a")],
      ["image", sel.type("img")],
      ["table-cell", sel.type("td")],
      ["missing-custom", sel.custom("callout")],
      ["missing-id", sel.id("hero")],
      ["missing-heading", sel.heading(6)],
    ]);

    expect(collectUsedStyle(document, rules, {
      siteHeader: true,
      frontmatter: { tags: ["featured"] },
    })).toEqual([
      "paragraph", "heading", "main-role", "frontmatter-tag", "second-item",
      "odd-item", "last-item", "list-child", "quote-paragraph",
      "intro-after-title", "tagged-paragraph", "not-table",
      "header-link", "emphasis", "strong", "code", "link", "image",
    ]);
  });

  it("matches the HTML table structure emitted for table AST nodes", () => {
    const document: DocumentNode = {
      type: "document",
      children: [{
        type: "table",
        align: [null],
        children: [
          { type: "table_row", isHeader: true, children: [{ type: "table_cell", children: [{ type: "text", value: "H" }] }] },
          { type: "table_row", isHeader: false, children: [{ type: "table_cell", children: [{ type: "text", value: "D" }] }] },
        ],
      }],
    };
    expect(collectUsedStyle(document, style([
      ["table", sel.type("table")],
      ["head", sel.descendantOf(sel.type("thead"), sel.type("th"))],
      ["body", sel.descendantOf(sel.type("tbody"), sel.type("td"))],
    ]), { siteHeader: false, frontmatter: {} })).toEqual(["table", "head", "body"]);
  });

  it("does not count the omitted paragraph wrapper in a tight list", () => {
    const document = parse("- one\n- two\n");
    expect(collectUsedStyle(document, style([["paragraph", sel.type("p")]]), {
      siteHeader: false,
      frontmatter: {},
    })).toEqual([]);
  });

  it("retains every rule when trusted raw HTML makes exact matching impossible", () => {
    const document = parse("<aside id=\"hero\">Raw</aside>\n");
    const rules = style([
      ["paragraph", sel.type("p")],
      ["hero", sel.id("hero")],
      ["aside", sel.type("aside")],
    ]);
    expect(collectUsedStyle(document, rules, {
      siteHeader: false,
      frontmatter: { tags: "one, two" },
    })).toEqual(["paragraph", "hero", "aside"]);
  });
});
