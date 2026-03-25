/**
 * XSS Attack Vectors — Shared Test Fixtures
 *
 * This file collects DocumentNode fixtures for the XSS attack scenarios
 * defined in the TE02 spec §Testing Strategy. Each fixture documents:
 *
 *   - The attack technique
 *   - What a broken sanitizer would allow through
 *   - What the correct sanitizer should produce
 *
 * @module xss-vectors
 */

import type {
  DocumentNode,
  ParagraphNode,
  LinkNode,
  ImageNode,
  AutolinkNode,
  RawBlockNode,
  RawInlineNode,
  TextNode,
  HeadingNode,
} from "@coding-adventures/document-ast";

// ─── Helper Builders ──────────────────────────────────────────────────────────

export function doc(...children: DocumentNode["children"]): DocumentNode {
  return { type: "document", children };
}

export function para(...children: ParagraphNode["children"]): ParagraphNode {
  return { type: "paragraph", children };
}

export function text(value: string): TextNode {
  return { type: "text", value };
}

export function heading(level: 1 | 2 | 3 | 4 | 5 | 6, ...children: HeadingNode["children"]): HeadingNode {
  return { type: "heading", level, children };
}

export function link(destination: string, ...children: LinkNode["children"]): LinkNode {
  return { type: "link", destination, title: null, children };
}

export function image(destination: string, alt: string): ImageNode {
  return { type: "image", destination, title: null, alt };
}

export function autolink(destination: string, isEmail = false): AutolinkNode {
  return { type: "autolink", destination, isEmail };
}

export function rawBlock(format: string, value: string): RawBlockNode {
  return { type: "raw_block", format, value };
}

export function rawInline(format: string, value: string): RawInlineNode {
  return { type: "raw_inline", format, value };
}

// ─── JavaScript URL Injection Vectors ─────────────────────────────────────────
//
// These vectors test that the URL scheme check correctly identifies and
// neutralises dangerous schemes. Browsers parse URL schemes leniently —
// they strip control characters before parsing — so a naive string comparison
// would allow many of these through.

/** Plain javascript: scheme — the most common XSS vector in links */
export const LINK_JAVASCRIPT = doc(para(link("javascript:alert(1)", text("click me"))));

/** Uppercase scheme — JAVASCRIPT: is the same as javascript: in browsers */
export const LINK_JAVASCRIPT_UPPER = doc(para(link("JAVASCRIPT:alert(1)", text("click me"))));

/** data: scheme — can embed HTML with scripts */
export const LINK_DATA = doc(para(link("data:text/html,<script>alert(1)</script>", text("click me"))));

/** blob: scheme — can execute scripts in same-origin blob URLs */
export const LINK_BLOB = doc(para(link("blob:https://origin/some-uuid", text("click me"))));

/** vbscript: scheme — legacy IE script execution */
export const LINK_VBSCRIPT = doc(para(link("vbscript:MsgBox(1)", text("click me"))));

/**
 * Null byte bypass — "java\x00script:" in some parsers strips the null byte
 * and executes the javascript: scheme. Stripping C0 controls before scheme
 * detection prevents this.
 */
export const LINK_NULL_BYTE_BYPASS = doc(para(link("java\x00script:alert(1)", text("click me"))));

/**
 * Carriage return bypass — "java\rscript:" can be parsed as "javascript:"
 * by WHATWG URL parsers that strip CR before scheme detection.
 */
export const LINK_CR_BYPASS = doc(para(link("java\rscript:alert(1)", text("click me"))));

/**
 * Zero-width space bypass — "\u200bjavascript:" uses an invisible Unicode
 * character to fool naive string comparisons.
 */
export const LINK_ZWS_BYPASS = doc(para(link("\u200bjavascript:alert(1)", text("click me"))));

/** javascript: in image source */
export const IMAGE_JAVASCRIPT = doc(para(image("javascript:alert(1)", "alt text")));

/** javascript: in autolink */
export const AUTOLINK_JAVASCRIPT = doc(para(autolink("javascript:alert(1)")));

// ─── Safe URLs ────────────────────────────────────────────────────────────────

export const LINK_HTTPS = doc(para(link("https://example.com", text("safe link"))));
export const LINK_RELATIVE = doc(para(link("/relative/path", text("relative link"))));
export const LINK_MAILTO = doc(para(link("mailto:user@example.com", text("email"))));
export const IMAGE_HTTPS = doc(para(image("https://example.com/img.png", "a photo")));
export const AUTOLINK_HTTPS = doc(para(autolink("https://example.com")));
export const AUTOLINK_EMAIL = doc(para(autolink("user@example.com", true)));

// ─── Raw Block / Inline Injection Vectors ─────────────────────────────────────

/** HTML raw block — passes through verbatim to HTML renderer */
export const RAW_BLOCK_HTML = doc(rawBlock("html", "<script>alert(1)</script>\n"));
export const RAW_BLOCK_LATEX = doc(rawBlock("latex", "\\textbf{danger}\n"));
export const RAW_INLINE_HTML = doc(para(rawInline("html", "<script>alert(1)</script>")));
export const RAW_INLINE_LATEX = doc(para(rawInline("latex", "\\emph{x}")));

// ─── Heading Level Vectors ────────────────────────────────────────────────────

/** h1 heading — should be clamped to h2 by STRICT policy */
export const HEADING_H1 = doc(heading(1, text("Page Title Override")));
export const HEADING_H2 = doc(heading(2, text("Section")));
export const HEADING_H5 = doc(heading(5, text("Deep Section")));

// ─── Empty Children Cases ─────────────────────────────────────────────────────

/** Paragraph containing only a raw inline — both should be dropped by STRICT */
export const PARA_ONLY_RAW_INLINE = doc(para(rawInline("html", "<b>bold</b>")));

/** Link containing only a raw inline child */
export const LINK_ONLY_RAW_INLINE = doc(para(link("https://ok.com", rawInline("html", "<b>x</b>"))));
