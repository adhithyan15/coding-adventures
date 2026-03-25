/**
 * AST Sanitizer Tests
 *
 * Comprehensive test coverage for the document-ast-sanitizer package.
 *
 * Test categories:
 *   1. Named presets (STRICT, RELAXED, PASSTHROUGH)
 *   2. Raw block/inline handling
 *   3. URL scheme sanitization (including bypass vectors)
 *   4. Heading level clamping
 *   5. Image handling (drop, transformToText)
 *   6. Link handling (drop, promote children)
 *   7. Empty children cleanup
 *   8. Immutability guarantee
 *   9. PASSTHROUGH identity
 *   10. Code block and code span handling
 *   11. Nested structures (lists, blockquotes)
 *   12. URL utilities (unit-tested separately)
 *
 * @module sanitizer.test
 */

import { describe, it, expect } from "vitest";
import { sanitize, STRICT, RELAXED, PASSTHROUGH } from "../src/index.js";
import { stripControlChars, extractScheme, isSchemeAllowed } from "../src/url-utils.js";
import type { DocumentNode, TextNode, ImageNode, LinkNode, HeadingNode } from "@coding-adventures/document-ast";
import {
  doc, para, text, heading, link, image, autolink, rawBlock, rawInline,
  LINK_JAVASCRIPT, LINK_JAVASCRIPT_UPPER, LINK_DATA, LINK_BLOB, LINK_VBSCRIPT,
  LINK_NULL_BYTE_BYPASS, LINK_CR_BYPASS, LINK_ZWS_BYPASS,
  IMAGE_JAVASCRIPT, AUTOLINK_JAVASCRIPT,
  LINK_HTTPS, LINK_RELATIVE, LINK_MAILTO, IMAGE_HTTPS, AUTOLINK_HTTPS, AUTOLINK_EMAIL,
  RAW_BLOCK_HTML, RAW_BLOCK_LATEX, RAW_INLINE_HTML, RAW_INLINE_LATEX,
  HEADING_H1, HEADING_H2, HEADING_H5,
  PARA_ONLY_RAW_INLINE, LINK_ONLY_RAW_INLINE,
} from "./xss-vectors.js";

// ─── Helper: extract first paragraph children ─────────────────────────────────

function firstParaChildren(d: DocumentNode) {
  const para = d.children[0];
  if (!para || para.type !== "paragraph") throw new Error("expected paragraph");
  return para.children;
}

function firstChild(d: DocumentNode) {
  return d.children[0];
}

// ─── URL Utilities (unit tests) ───────────────────────────────────────────────

describe("stripControlChars", () => {
  it("strips null bytes", () => {
    expect(stripControlChars("java\x00script:")).toBe("javascript:");
  });

  it("strips carriage return", () => {
    expect(stripControlChars("java\rscript:")).toBe("javascript:");
  });

  it("strips line feed", () => {
    expect(stripControlChars("java\nscript:")).toBe("javascript:");
  });

  it("strips tab", () => {
    expect(stripControlChars("java\tscript:")).toBe("javascript:");
  });

  it("strips zero-width space (U+200B)", () => {
    expect(stripControlChars("\u200bjavascript:")).toBe("javascript:");
  });

  it("strips zero-width non-joiner (U+200C)", () => {
    expect(stripControlChars("java\u200Cscript:")).toBe("javascript:");
  });

  it("strips zero-width joiner (U+200D)", () => {
    expect(stripControlChars("java\u200Dscript:")).toBe("javascript:");
  });

  it("strips word joiner (U+2060)", () => {
    expect(stripControlChars("java\u2060script:")).toBe("javascript:");
  });

  it("strips BOM (U+FEFF)", () => {
    expect(stripControlChars("\uFEFFjavascript:")).toBe("javascript:");
  });

  it("leaves normal URLs unchanged", () => {
    expect(stripControlChars("https://example.com/path?q=1")).toBe("https://example.com/path?q=1");
  });

  it("leaves empty string unchanged", () => {
    expect(stripControlChars("")).toBe("");
  });
});

describe("extractScheme", () => {
  it("extracts https scheme", () => {
    expect(extractScheme("https://example.com")).toBe("https");
  });

  it("extracts javascript scheme", () => {
    expect(extractScheme("javascript:alert(1)")).toBe("javascript");
  });

  it("extracts mailto scheme", () => {
    expect(extractScheme("mailto:user@host")).toBe("mailto");
  });

  it("returns lowercase for uppercase scheme", () => {
    expect(extractScheme("JAVASCRIPT:alert(1)")).toBe("javascript");
  });

  it("returns null for relative URLs with no colon", () => {
    expect(extractScheme("/relative/path")).toBeNull();
  });

  it("returns null for relative URLs with no scheme at all", () => {
    expect(extractScheme("path/to/page")).toBeNull();
  });

  it("returns null for colon after slash (not a scheme)", () => {
    expect(extractScheme("/path:with:colons")).toBeNull();
  });

  it("returns null for colon after question mark", () => {
    expect(extractScheme("?q=a:b")).toBeNull();
  });

  it("returns null for empty string", () => {
    expect(extractScheme("")).toBeNull();
  });

  it("extracts ftp scheme", () => {
    expect(extractScheme("ftp://files.example.com")).toBe("ftp");
  });

  it("extracts data scheme", () => {
    expect(extractScheme("data:text/html,<b>x</b>")).toBe("data");
  });
});

describe("isSchemeAllowed", () => {
  it("allows https when in allowlist", () => {
    expect(isSchemeAllowed("https://ok.com", ["http", "https"])).toBe(true);
  });

  it("blocks javascript when not in allowlist", () => {
    expect(isSchemeAllowed("javascript:alert(1)", ["http", "https"])).toBe(false);
  });

  it("allows relative URLs always (no scheme)", () => {
    expect(isSchemeAllowed("/relative/path", ["http", "https"])).toBe(true);
    expect(isSchemeAllowed("relative/path", ["http", "https"])).toBe(true);
  });

  it("allows anything when allowedUrlSchemes is null", () => {
    expect(isSchemeAllowed("javascript:x", null)).toBe(true);
    expect(isSchemeAllowed("data:evil", null)).toBe(true);
  });

  it("allows anything when allowedUrlSchemes is undefined", () => {
    expect(isSchemeAllowed("javascript:x", undefined)).toBe(true);
  });

  it("strips control chars before checking", () => {
    // java\x00script: should be detected as javascript: after stripping
    expect(isSchemeAllowed("java\x00script:alert(1)", ["http", "https"])).toBe(false);
  });

  it("case-insensitive scheme matching", () => {
    expect(isSchemeAllowed("HTTPS://ok.com", ["http", "https"])).toBe(true);
    expect(isSchemeAllowed("JAVASCRIPT:x", ["http", "https"])).toBe(false);
  });
});

// ─── PASSTHROUGH Preset ───────────────────────────────────────────────────────

describe("PASSTHROUGH preset", () => {
  it("keeps raw HTML blocks unchanged", () => {
    const result = sanitize(RAW_BLOCK_HTML, PASSTHROUGH);
    expect(result.children[0]).toMatchObject({ type: "raw_block", format: "html" });
  });

  it("keeps raw inline HTML unchanged", () => {
    const result = sanitize(RAW_INLINE_HTML, PASSTHROUGH);
    const children = firstParaChildren(result);
    expect(children[0]).toMatchObject({ type: "raw_inline", format: "html" });
  });

  it("keeps javascript: links unchanged", () => {
    const result = sanitize(LINK_JAVASCRIPT, PASSTHROUGH);
    const link = firstParaChildren(result)[0] as LinkNode;
    expect(link.destination).toBe("javascript:alert(1)");
  });

  it("keeps h1 headings unchanged", () => {
    const result = sanitize(HEADING_H1, PASSTHROUGH);
    expect(firstChild(result)).toMatchObject({ type: "heading", level: 1 });
  });

  it("keeps images unchanged", () => {
    const result = sanitize(IMAGE_HTTPS, PASSTHROUGH);
    expect(firstParaChildren(result)[0]).toMatchObject({ type: "image" });
  });

  it("returns empty document for empty document", () => {
    const empty: DocumentNode = { type: "document", children: [] };
    const result = sanitize(empty, PASSTHROUGH);
    expect(result).toEqual({ type: "document", children: [] });
  });
});

// ─── Raw Block Handling ───────────────────────────────────────────────────────

describe("Raw block handling", () => {
  it("drop-all drops HTML raw block", () => {
    const result = sanitize(RAW_BLOCK_HTML, { allowRawBlockFormats: "drop-all" });
    expect(result.children).toHaveLength(0);
  });

  it("drop-all drops LaTeX raw block", () => {
    const result = sanitize(RAW_BLOCK_LATEX, { allowRawBlockFormats: "drop-all" });
    expect(result.children).toHaveLength(0);
  });

  it("passthrough keeps HTML raw block", () => {
    const result = sanitize(RAW_BLOCK_HTML, { allowRawBlockFormats: "passthrough" });
    expect(result.children[0]).toMatchObject({ type: "raw_block", format: "html" });
  });

  it("allowlist keeps HTML when html is in list", () => {
    const result = sanitize(RAW_BLOCK_HTML, { allowRawBlockFormats: ["html"] });
    expect(result.children[0]).toMatchObject({ type: "raw_block", format: "html" });
  });

  it("allowlist drops LaTeX when html is in list but not latex", () => {
    const result = sanitize(RAW_BLOCK_LATEX, { allowRawBlockFormats: ["html"] });
    expect(result.children).toHaveLength(0);
  });

  it("STRICT drops HTML raw block", () => {
    const result = sanitize(RAW_BLOCK_HTML, STRICT);
    expect(result.children).toHaveLength(0);
  });
});

describe("Raw inline handling", () => {
  it("drop-all drops HTML raw inline", () => {
    const result = sanitize(RAW_INLINE_HTML, { allowRawInlineFormats: "drop-all" });
    // The paragraph had only a raw inline; after dropping it, the paragraph
    // itself should be dropped (empty children cleanup)
    expect(result.children).toHaveLength(0);
  });

  it("passthrough keeps HTML raw inline", () => {
    const result = sanitize(RAW_INLINE_HTML, { allowRawInlineFormats: "passthrough" });
    const children = firstParaChildren(result);
    expect(children[0]).toMatchObject({ type: "raw_inline", format: "html" });
  });

  it("allowlist keeps html raw inline when html in list", () => {
    const result = sanitize(RAW_INLINE_HTML, { allowRawInlineFormats: ["html"] });
    const children = firstParaChildren(result);
    expect(children[0]).toMatchObject({ type: "raw_inline", format: "html" });
  });

  it("allowlist drops latex raw inline when only html in list", () => {
    const result = sanitize(RAW_INLINE_LATEX, { allowRawInlineFormats: ["html"] });
    expect(result.children).toHaveLength(0);
  });

  it("STRICT drops HTML raw inline", () => {
    const result = sanitize(RAW_INLINE_HTML, STRICT);
    expect(result.children).toHaveLength(0);
  });
});

// ─── URL Scheme Sanitization ──────────────────────────────────────────────────

describe("URL scheme sanitization — links", () => {
  it("blocks javascript: → sets destination to ''", () => {
    const result = sanitize(LINK_JAVASCRIPT, STRICT);
    const ln = firstParaChildren(result)[0] as LinkNode;
    expect(ln.type).toBe("link");
    expect(ln.destination).toBe("");
  });

  it("blocks JAVASCRIPT: (uppercase)", () => {
    const result = sanitize(LINK_JAVASCRIPT_UPPER, STRICT);
    const ln = firstParaChildren(result)[0] as LinkNode;
    expect(ln.destination).toBe("");
  });

  it("blocks data: scheme", () => {
    const result = sanitize(LINK_DATA, STRICT);
    const ln = firstParaChildren(result)[0] as LinkNode;
    expect(ln.destination).toBe("");
  });

  it("blocks blob: scheme", () => {
    const result = sanitize(LINK_BLOB, { allowedUrlSchemes: ["http", "https"] });
    const ln = firstParaChildren(result)[0] as LinkNode;
    expect(ln.destination).toBe("");
  });

  it("blocks vbscript: scheme", () => {
    const result = sanitize(LINK_VBSCRIPT, STRICT);
    const ln = firstParaChildren(result)[0] as LinkNode;
    expect(ln.destination).toBe("");
  });

  it("blocks null byte bypass (java\\x00script:)", () => {
    const result = sanitize(LINK_NULL_BYTE_BYPASS, STRICT);
    const ln = firstParaChildren(result)[0] as LinkNode;
    expect(ln.destination).toBe("");
  });

  it("blocks CR bypass (java\\rscript:)", () => {
    const result = sanitize(LINK_CR_BYPASS, STRICT);
    const ln = firstParaChildren(result)[0] as LinkNode;
    expect(ln.destination).toBe("");
  });

  it("blocks zero-width space bypass (\\u200bjavascript:)", () => {
    const result = sanitize(LINK_ZWS_BYPASS, STRICT);
    const ln = firstParaChildren(result)[0] as LinkNode;
    expect(ln.destination).toBe("");
  });

  it("allows https: link through", () => {
    const result = sanitize(LINK_HTTPS, STRICT);
    const ln = firstParaChildren(result)[0] as LinkNode;
    expect(ln.destination).toBe("https://example.com");
  });

  it("allows relative URL through", () => {
    const result = sanitize(LINK_RELATIVE, STRICT);
    const ln = firstParaChildren(result)[0] as LinkNode;
    expect(ln.destination).toBe("/relative/path");
  });

  it("allows mailto: link through", () => {
    const result = sanitize(LINK_MAILTO, STRICT);
    const ln = firstParaChildren(result)[0] as LinkNode;
    expect(ln.destination).toBe("mailto:user@example.com");
  });
});

describe("URL scheme sanitization — images", () => {
  it("blocks javascript: image source → sets destination to ''", () => {
    const result = sanitize(IMAGE_JAVASCRIPT, STRICT);
    // STRICT has transformImageToText: true, so this becomes a TextNode
    const node = firstParaChildren(result)[0] as TextNode;
    expect(node.type).toBe("text");
    // alt text is "alt text"
    expect(node.value).toBe("alt text");
  });

  it("blocks javascript: image source when not transforming to text", () => {
    const result = sanitize(IMAGE_JAVASCRIPT, {
      ...STRICT,
      transformImageToText: false,
    });
    const img = firstParaChildren(result)[0] as ImageNode;
    expect(img.type).toBe("image");
    expect(img.destination).toBe("");
  });

  it("allows https: image through", () => {
    const result = sanitize(IMAGE_HTTPS, { allowedUrlSchemes: ["http", "https"] });
    const img = firstParaChildren(result)[0] as ImageNode;
    expect(img.destination).toBe("https://example.com/img.png");
  });
});

describe("URL scheme sanitization — autolinks", () => {
  it("drops javascript: autolink", () => {
    const result = sanitize(AUTOLINK_JAVASCRIPT, STRICT);
    expect(result.children).toHaveLength(0);
  });

  it("keeps https: autolink", () => {
    const result = sanitize(AUTOLINK_HTTPS, STRICT);
    const node = firstParaChildren(result)[0];
    expect(node).toMatchObject({ type: "autolink", destination: "https://example.com" });
  });

  it("keeps email autolink (relative by scheme detection)", () => {
    const result = sanitize(AUTOLINK_EMAIL, STRICT);
    const node = firstParaChildren(result)[0];
    expect(node).toMatchObject({ type: "autolink", isEmail: true });
  });
});

// ─── Heading Level Clamping ───────────────────────────────────────────────────

describe("Heading level clamping", () => {
  it("drops all headings when maxHeadingLevel is 'drop'", () => {
    const result = sanitize(HEADING_H1, { maxHeadingLevel: "drop" });
    expect(result.children).toHaveLength(0);
  });

  it("drops h5 when maxHeadingLevel is 'drop'", () => {
    const result = sanitize(HEADING_H5, { maxHeadingLevel: "drop" });
    expect(result.children).toHaveLength(0);
  });

  it("clamps h1 to h2 when minHeadingLevel is 2", () => {
    const result = sanitize(HEADING_H1, { minHeadingLevel: 2 });
    const h = firstChild(result) as HeadingNode;
    expect(h.level).toBe(2);
  });

  it("clamps h5 to h3 when maxHeadingLevel is 3", () => {
    const result = sanitize(HEADING_H5, { maxHeadingLevel: 3 });
    const h = firstChild(result) as HeadingNode;
    expect(h.level).toBe(3);
  });

  it("does not clamp h2 when min=2, max=6", () => {
    const result = sanitize(HEADING_H2, { minHeadingLevel: 2, maxHeadingLevel: 6 });
    const h = firstChild(result) as HeadingNode;
    expect(h.level).toBe(2);
  });

  it("STRICT clamps h1 to h2", () => {
    const result = sanitize(HEADING_H1, STRICT);
    const h = firstChild(result) as HeadingNode;
    expect(h.level).toBe(2);
  });

  it("preserves heading children after clamping", () => {
    const result = sanitize(HEADING_H1, { minHeadingLevel: 2 });
    const h = firstChild(result) as HeadingNode;
    expect(h.children[0]).toMatchObject({ type: "text", value: "Page Title Override" });
  });

  it("clamps when both min and max conflict: level below min", () => {
    // h1 with minHeadingLevel: 3 → clamped to 3
    const d = doc(heading(1, text("title")));
    const result = sanitize(d, { minHeadingLevel: 3, maxHeadingLevel: 5 });
    const h = firstChild(result) as HeadingNode;
    expect(h.level).toBe(3);
  });

  it("clamps when both min and max conflict: level above max", () => {
    // h6 with maxHeadingLevel: 4 → clamped to 4
    const d = doc(heading(6, text("deep")));
    const result = sanitize(d, { minHeadingLevel: 2, maxHeadingLevel: 4 });
    const h = firstChild(result) as HeadingNode;
    expect(h.level).toBe(4);
  });
});

// ─── Image Handling ───────────────────────────────────────────────────────────

describe("Image handling", () => {
  it("drops image when dropImages: true", () => {
    const result = sanitize(IMAGE_HTTPS, { dropImages: true });
    expect(result.children).toHaveLength(0);
  });

  it("converts image to TextNode when transformImageToText: true", () => {
    const result = sanitize(IMAGE_HTTPS, { transformImageToText: true });
    const node = firstParaChildren(result)[0];
    expect(node).toMatchObject({ type: "text", value: "a photo" });
  });

  it("dropImages takes precedence over transformImageToText", () => {
    const result = sanitize(IMAGE_HTTPS, { dropImages: true, transformImageToText: true });
    expect(result.children).toHaveLength(0);
  });

  it("keeps image as-is with safe URL when neither flag set", () => {
    const result = sanitize(IMAGE_HTTPS, { allowedUrlSchemes: ["http", "https"] });
    const img = firstParaChildren(result)[0] as ImageNode;
    expect(img.type).toBe("image");
    expect(img.destination).toBe("https://example.com/img.png");
    expect(img.alt).toBe("a photo");
  });

  it("converts image with empty alt to TextNode with empty value", () => {
    const d = doc(para(image("https://ok.com/img.png", "")));
    const result = sanitize(d, { transformImageToText: true });
    const node = firstParaChildren(result)[0];
    expect(node).toMatchObject({ type: "text", value: "" });
  });
});

// ─── Link Dropping and Promotion ──────────────────────────────────────────────

describe("Link dropping and promotion", () => {
  it("promotes link children to parent when dropLinks: true", () => {
    const d = doc(para(text("See "), link("https://ok.com", text("click here")), text(" for more")));
    const result = sanitize(d, { dropLinks: true });
    const children = firstParaChildren(result);
    expect(children).toHaveLength(3);
    expect(children[0]).toMatchObject({ type: "text", value: "See " });
    expect(children[1]).toMatchObject({ type: "text", value: "click here" });
    expect(children[2]).toMatchObject({ type: "text", value: " for more" });
  });

  it("promoted children still get sanitized", () => {
    // A link containing a raw inline — with dropLinks+dropRawInlines, both go
    const d = doc(para(link("https://ok.com", rawInline("html", "<b>x</b>"))));
    const result = sanitize(d, {
      dropLinks: true,
      allowRawInlineFormats: "drop-all",
    });
    // The raw inline was promoted then dropped → paragraph empty → paragraph dropped
    expect(result.children).toHaveLength(0);
  });

  it("keeps links when dropLinks: false", () => {
    const result = sanitize(LINK_HTTPS, { dropLinks: false, allowedUrlSchemes: ["https"] });
    const node = firstParaChildren(result)[0];
    expect(node.type).toBe("link");
  });

  it("drops link entirely when all children are promoted and dropped", () => {
    // Link with only a raw inline child
    const result = sanitize(LINK_ONLY_RAW_INLINE, {
      dropLinks: false,
      allowRawInlineFormats: "drop-all",
    });
    // The link's only child was dropped → link is empty → link is dropped
    // → paragraph is empty → paragraph is dropped
    expect(result.children).toHaveLength(0);
  });
});

// ─── Empty Children Cleanup ───────────────────────────────────────────────────

describe("Empty children cleanup", () => {
  it("drops paragraph when its only child is a dropped raw inline", () => {
    const result = sanitize(PARA_ONLY_RAW_INLINE, { allowRawInlineFormats: "drop-all" });
    expect(result.children).toHaveLength(0);
  });

  it("never drops DocumentNode even when all children are dropped", () => {
    const result = sanitize(RAW_BLOCK_HTML, { allowRawBlockFormats: "drop-all" });
    expect(result.type).toBe("document");
    expect(result.children).toHaveLength(0);
  });

  it("drops blockquote when all its children are dropped", () => {
    const d = doc({ type: "blockquote", children: [para(rawInline("html", "<b>x</b>"))] });
    const result = sanitize(d, { allowRawInlineFormats: "drop-all" });
    expect(result.children).toHaveLength(0);
  });

  it("drops list when all items are dropped", () => {
    const d = doc({
      type: "list",
      ordered: false,
      start: null,
      tight: true,
      children: [{ type: "list_item", children: [para(rawInline("html", "<b>x</b>"))] }],
    });
    const result = sanitize(d, { allowRawInlineFormats: "drop-all" });
    expect(result.children).toHaveLength(0);
  });

  it("drops emphasis when all children are dropped", () => {
    const d = doc(para({ type: "emphasis", children: [rawInline("html", "<b>x</b>")] }));
    const result = sanitize(d, { allowRawInlineFormats: "drop-all" });
    expect(result.children).toHaveLength(0);
  });

  it("drops strong when all children are dropped", () => {
    const d = doc(para({ type: "strong", children: [rawInline("html", "<b>x</b>")] }));
    const result = sanitize(d, { allowRawInlineFormats: "drop-all" });
    expect(result.children).toHaveLength(0);
  });

  it("drops heading when all inline children are dropped", () => {
    const d = doc(heading(2, rawInline("html", "<b>title</b>")));
    const result = sanitize(d, { allowRawInlineFormats: "drop-all" });
    expect(result.children).toHaveLength(0);
  });
});

// ─── Code Block and Code Span ─────────────────────────────────────────────────

describe("Code block handling", () => {
  it("drops code block when dropCodeBlocks: true", () => {
    const d = doc({ type: "code_block", language: "js", value: "alert(1);\n" });
    const result = sanitize(d, { dropCodeBlocks: true });
    expect(result.children).toHaveLength(0);
  });

  it("keeps code block when dropCodeBlocks: false", () => {
    const d = doc({ type: "code_block", language: "js", value: "const x = 1;\n" });
    const result = sanitize(d, { dropCodeBlocks: false });
    expect(result.children[0]).toMatchObject({ type: "code_block" });
  });
});

describe("Code span handling", () => {
  it("converts code span to text when transformCodeSpanToText: true", () => {
    const d = doc(para({ type: "code_span", value: "const x = 1" }));
    const result = sanitize(d, { transformCodeSpanToText: true });
    const node = firstParaChildren(result)[0];
    expect(node).toMatchObject({ type: "text", value: "const x = 1" });
  });

  it("keeps code span when transformCodeSpanToText: false", () => {
    const d = doc(para({ type: "code_span", value: "const x = 1" }));
    const result = sanitize(d, { transformCodeSpanToText: false });
    const node = firstParaChildren(result)[0];
    expect(node).toMatchObject({ type: "code_span", value: "const x = 1" });
  });
});

// ─── Blockquote Handling ──────────────────────────────────────────────────────

describe("Blockquote handling", () => {
  it("drops blockquote when dropBlockquotes: true", () => {
    const d = doc({ type: "blockquote", children: [para(text("a quote"))] });
    const result = sanitize(d, { dropBlockquotes: true });
    expect(result.children).toHaveLength(0);
  });

  it("keeps blockquote with sanitized children when dropBlockquotes: false", () => {
    const d = doc({ type: "blockquote", children: [para(text("a quote"))] });
    const result = sanitize(d, { dropBlockquotes: false });
    expect(result.children[0]).toMatchObject({ type: "blockquote" });
  });
});

// ─── ThematicBreak and Break Nodes ───────────────────────────────────────────

describe("Leaf nodes always kept", () => {
  it("thematic break is always kept", () => {
    const d = doc({ type: "thematic_break" });
    const result = sanitize(d, STRICT);
    expect(result.children[0]).toMatchObject({ type: "thematic_break" });
  });

  it("hard break is always kept", () => {
    const d = doc(para({ type: "hard_break" }));
    const result = sanitize(d, STRICT);
    const node = firstParaChildren(result)[0];
    expect(node).toMatchObject({ type: "hard_break" });
  });

  it("soft break is always kept", () => {
    const d = doc(para(text("line 1"), { type: "soft_break" }, text("line 2")));
    const result = sanitize(d, STRICT);
    const children = firstParaChildren(result);
    expect(children[1]).toMatchObject({ type: "soft_break" });
  });
});

// ─── List Handling ────────────────────────────────────────────────────────────

describe("List handling", () => {
  it("sanitizes list item children", () => {
    const d = doc({
      type: "list",
      ordered: false,
      start: null,
      tight: true,
      children: [
        { type: "list_item", children: [para(text("item 1"))] },
        { type: "list_item", children: [para(rawInline("html", "<b>x</b>"))] },
      ],
    });
    const result = sanitize(d, { allowRawInlineFormats: "drop-all" });
    const list = result.children[0];
    if (list.type !== "list") throw new Error("expected list");
    // Second item had only a raw inline → item dropped → 1 item remains
    expect(list.children).toHaveLength(1);
    expect(list.children[0].children[0]).toMatchObject({ type: "paragraph" });
  });

  it("preserves ordered list properties", () => {
    const d = doc({
      type: "list",
      ordered: true,
      start: 3,
      tight: false,
      children: [{ type: "list_item", children: [para(text("item"))] }],
    });
    const result = sanitize(d, PASSTHROUGH);
    const list = result.children[0];
    if (list.type !== "list") throw new Error("expected list");
    expect(list.ordered).toBe(true);
    expect(list.start).toBe(3);
    expect(list.tight).toBe(false);
  });
});

// ─── Immutability ─────────────────────────────────────────────────────────────

describe("Immutability", () => {
  it("does not mutate the input document", () => {
    const original = LINK_JAVASCRIPT;
    // Deep clone to compare later
    const snapshot = JSON.stringify(original);
    sanitize(original, STRICT);
    // The original document must be unchanged
    expect(JSON.stringify(original)).toBe(snapshot);
  });

  it("returns a new document object (not the same reference)", () => {
    const original = LINK_HTTPS;
    const result = sanitize(original, PASSTHROUGH);
    expect(result).not.toBe(original);
  });

  it("same document can be sanitized with different policies independently", () => {
    const original = doc(
      rawBlock("html", "<div>raw</div>\n"),
      para(link("javascript:x", text("click"))),
    );
    const strict = sanitize(original, STRICT);
    const pass = sanitize(original, PASSTHROUGH);

    // STRICT drops the raw block. The paragraph with the link is kept but
    // the link destination becomes "" (not the whole paragraph).
    expect(strict.children).toHaveLength(1);
    expect(strict.children[0].type).toBe("paragraph");

    // PASSTHROUGH keeps both nodes unchanged
    expect(pass.children).toHaveLength(2);
    expect(pass.children[0].type).toBe("raw_block");

    // Original unchanged
    expect(original.children).toHaveLength(2);
  });
});

// ─── STRICT Preset Full Smoke Test ───────────────────────────────────────────

describe("STRICT preset smoke test", () => {
  it("strips raw HTML block", () => {
    const result = sanitize(RAW_BLOCK_HTML, STRICT);
    expect(result.children).toHaveLength(0);
  });

  it("strips raw inline HTML", () => {
    const result = sanitize(RAW_INLINE_HTML, STRICT);
    expect(result.children).toHaveLength(0);
  });

  it("blocks javascript: link", () => {
    const result = sanitize(LINK_JAVASCRIPT, STRICT);
    const ln = firstParaChildren(result)[0] as LinkNode;
    expect(ln.destination).toBe("");
  });

  it("converts image to alt text", () => {
    const result = sanitize(IMAGE_HTTPS, STRICT);
    const node = firstParaChildren(result)[0];
    expect(node.type).toBe("text");
  });

  it("clamps h1 to h2", () => {
    const result = sanitize(HEADING_H1, STRICT);
    const h = firstChild(result) as HeadingNode;
    expect(h.level).toBe(2);
  });

  it("keeps safe text nodes", () => {
    const d = doc(para(text("Hello, world!")));
    const result = sanitize(d, STRICT);
    expect(firstParaChildren(result)[0]).toMatchObject({ type: "text", value: "Hello, world!" });
  });
});

// ─── RELAXED Preset ───────────────────────────────────────────────────────────

describe("RELAXED preset", () => {
  it("allows HTML raw blocks", () => {
    const result = sanitize(RAW_BLOCK_HTML, RELAXED);
    expect(result.children[0]).toMatchObject({ type: "raw_block", format: "html" });
  });

  it("drops LaTeX raw blocks", () => {
    const result = sanitize(RAW_BLOCK_LATEX, RELAXED);
    expect(result.children).toHaveLength(0);
  });

  it("blocks javascript: links", () => {
    const result = sanitize(LINK_JAVASCRIPT, RELAXED);
    const ln = firstParaChildren(result)[0] as LinkNode;
    expect(ln.destination).toBe("");
  });

  it("keeps images unchanged", () => {
    const result = sanitize(IMAGE_HTTPS, RELAXED);
    const img = firstParaChildren(result)[0];
    expect(img.type).toBe("image");
  });

  it("allows ftp: links", () => {
    const d = doc(para(link("ftp://files.example.com", text("files"))));
    const result = sanitize(d, RELAXED);
    const ln = firstParaChildren(result)[0] as LinkNode;
    expect(ln.destination).toBe("ftp://files.example.com");
  });

  it("allows h1 headings", () => {
    const result = sanitize(HEADING_H1, RELAXED);
    const h = firstChild(result) as HeadingNode;
    expect(h.level).toBe(1);
  });
});

// ─── Nested Structure Sanitization ───────────────────────────────────────────

describe("Nested structures", () => {
  it("sanitizes nested blockquote content", () => {
    const d = doc({
      type: "blockquote",
      children: [{
        type: "blockquote",
        children: [para(rawInline("html", "<b>x</b>"), text("safe"))],
      }],
    });
    const result = sanitize(d, { allowRawInlineFormats: "drop-all" });
    // Inner blockquote paragraph had raw inline dropped — "safe" text remains
    const outerBq = result.children[0];
    if (outerBq.type !== "blockquote") throw new Error("expected blockquote");
    const innerBq = outerBq.children[0];
    if (innerBq.type !== "blockquote") throw new Error("expected inner blockquote");
    const p = innerBq.children[0];
    if (p.type !== "paragraph") throw new Error("expected paragraph");
    expect(p.children[0]).toMatchObject({ type: "text", value: "safe" });
  });

  it("sanitizes links inside emphasis", () => {
    const d = doc(para({
      type: "emphasis",
      children: [link("javascript:x", text("dangerous"))],
    }));
    const result = sanitize(d, STRICT);
    const em = firstParaChildren(result)[0];
    if (em.type !== "emphasis") throw new Error("expected emphasis");
    const ln = em.children[0] as LinkNode;
    expect(ln.destination).toBe("");
  });

  it("sanitizes multiple raw blocks in one document", () => {
    const d = doc(
      rawBlock("html", "<script>alert(1)</script>\n"),
      para(text("normal text")),
      rawBlock("latex", "\\textbf{x}\n"),
    );
    const result = sanitize(d, STRICT);
    // Both raw blocks dropped, paragraph kept
    expect(result.children).toHaveLength(1);
    expect(result.children[0]).toMatchObject({ type: "paragraph" });
  });
});

// ─── Policy Composition ───────────────────────────────────────────────────────

describe("Policy composition via spread", () => {
  it("can override a single field from STRICT", () => {
    // Allow h1 but keep everything else strict
    const policy = { ...STRICT, minHeadingLevel: 1 as const };
    const result = sanitize(HEADING_H1, policy);
    const h = firstChild(result) as HeadingNode;
    expect(h.level).toBe(1);
  });

  it("can override allowedUrlSchemes from RELAXED", () => {
    const policy = { ...RELAXED, allowedUrlSchemes: ["https"] as const };
    // ftp: link should now be blocked (not in ["https"])
    const d = doc(para(link("ftp://files.example.com", text("files"))));
    const result = sanitize(d, policy);
    const ln = firstParaChildren(result)[0] as LinkNode;
    expect(ln.destination).toBe("");
  });
});

// ─── Default Policy (omitted fields) ─────────────────────────────────────────

describe("Default policy values (omitted fields)", () => {
  it("omitting allowRawBlockFormats defaults to passthrough", () => {
    const result = sanitize(RAW_BLOCK_HTML, {});
    expect(result.children[0]).toMatchObject({ type: "raw_block" });
  });

  it("omitting allowedUrlSchemes defaults to safe list (blocks javascript:)", () => {
    // When allowedUrlSchemes is omitted, the default is ["http","https","mailto","ftp"]
    // (see resolvePolicy — allowedUrlSchemes has a special default, not null)
    const result = sanitize(LINK_JAVASCRIPT, {});
    const ln = firstParaChildren(result)[0] as LinkNode;
    expect(ln.destination).toBe("");
  });

  it("omitting dropLinks defaults to false (links kept)", () => {
    const result = sanitize(LINK_HTTPS, {});
    expect(firstParaChildren(result)[0].type).toBe("link");
  });

  it("omitting maxHeadingLevel defaults to 6 (no clamping)", () => {
    const result = sanitize(HEADING_H5, {});
    const h = firstChild(result) as HeadingNode;
    expect(h.level).toBe(5);
  });
});
