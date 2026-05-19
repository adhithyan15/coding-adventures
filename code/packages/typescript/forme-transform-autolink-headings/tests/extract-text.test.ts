/**
 * extract-text.test.ts — inline-node flattening to plain text.
 */

import { describe, it, expect } from "vitest";
import type { InlineNode } from "@coding-adventures/document-ast";
import { extractText } from "../src/index.js";

function txt(value: string): InlineNode { return { type: "text", value }; }

describe("extractText — basic nodes", () => {
  it("single text node", () => {
    expect(extractText([txt("hello")])).toBe("hello");
  });

  it("multiple text nodes concatenate", () => {
    expect(extractText([txt("hello "), txt("world")])).toBe("hello world");
  });

  it("empty list → empty string", () => {
    expect(extractText([])).toBe("");
  });
});

describe("extractText — formatting wrappers (recurse)", () => {
  it("emphasis", () => {
    expect(extractText([
      txt("hello "),
      { type: "emphasis", children: [txt("world")] },
    ])).toBe("hello world");
  });

  it("strong", () => {
    expect(extractText([
      { type: "strong", children: [txt("bold")] },
    ])).toBe("bold");
  });

  it("strikethrough", () => {
    expect(extractText([
      { type: "strikethrough", children: [txt("old")] },
    ])).toBe("old");
  });

  it("nested emphasis + strong", () => {
    expect(extractText([
      { type: "strong", children: [
        { type: "emphasis", children: [txt("bold-italic")] },
      ]},
    ])).toBe("bold-italic");
  });
});

describe("extractText — link / code / image", () => {
  it("link uses children text (the label)", () => {
    expect(extractText([
      { type: "link", destination: "https://example.com", title: null, children: [txt("click here")] },
    ])).toBe("click here");
  });

  it("code_span value verbatim", () => {
    expect(extractText([
      txt("call "),
      { type: "code_span", value: "doStuff()" },
    ])).toBe("call doStuff()");
  });

  it("image contributes alt text", () => {
    expect(extractText([
      { type: "image", destination: "/cat.png", title: null, alt: "a cat" },
    ])).toBe("a cat");
  });

  it("autolink uses destination (URL or email)", () => {
    expect(extractText([
      { type: "autolink", destination: "https://example.com", isEmail: false },
    ])).toBe("https://example.com");
  });

  it("autolink email destination", () => {
    expect(extractText([
      { type: "autolink", destination: "user@example.com", isEmail: true },
    ])).toBe("user@example.com");
  });
});

describe("extractText — breaks become spaces", () => {
  it("hard_break", () => {
    expect(extractText([txt("a"), { type: "hard_break" }, txt("b")])).toBe("a b");
  });

  it("soft_break", () => {
    expect(extractText([txt("a"), { type: "soft_break" }, txt("b")])).toBe("a b");
  });
});

describe("extractText — raw_inline is skipped", () => {
  it("raw_inline value not included (back-end-specific)", () => {
    expect(extractText([
      txt("hello "),
      { type: "raw_inline", format: "html", value: "<sup>x</sup>" },
      txt(" world"),
    ])).toBe("hello  world");
  });
});

describe("extractText — mixed real-world headings", () => {
  it("'Step **2**: install `npm` deps'", () => {
    expect(extractText([
      txt("Step "),
      { type: "strong", children: [txt("2")] },
      txt(": install "),
      { type: "code_span", value: "npm" },
      txt(" deps"),
    ])).toBe("Step 2: install npm deps");
  });

  it("link inside heading: '[Forme](https://...) overview'", () => {
    expect(extractText([
      { type: "link", destination: "https://forme.example", title: null, children: [txt("Forme")] },
      txt(" overview"),
    ])).toBe("Forme overview");
  });
});
