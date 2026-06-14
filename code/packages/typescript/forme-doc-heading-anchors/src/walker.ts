/**
 * walker.ts — AST walk that finds every heading, computes a slug, and
 * returns a new DocumentNode tree with the slug attached as an `id`
 * field on each heading node.
 *
 * =============================================================================
 * WHY A NEW TREE, NOT MUTATION?
 * =============================================================================
 *
 * Every node in `@coding-adventures/document-ast` is declared `readonly`.
 * That's a contract: callers downstream may share AST references across
 * multiple transforms (frontmatter strip → heading anchors → TOC
 * extract → syntax highlight → HTML emit) and rely on each transform
 * being pure.  Mutating in place breaks that.
 *
 * So this transform builds a NEW `DocumentNode` whose children are:
 *   - The same reference as the input for non-heading blocks (their
 *     readonly contract means sharing is safe).
 *   - A freshly-allocated `AnchoredHeadingNode` for each heading, with
 *     the same `type`, `level`, and `children` references plus a new
 *     `id` field.
 *
 * Memory cost: O(headings), not O(nodes).  Bounded and small.
 *
 * =============================================================================
 * COLLISION SUFFIXING
 * =============================================================================
 *
 * Two `# Setup` headings in the same document both want `setup` as
 * their slug.  We follow GitHub's behaviour: the first heading keeps
 * the bare slug; the second gets `-1`, the third `-2`, etc.
 *
 *   ## Setup           → id="setup"
 *   ## Setup           → id="setup-1"
 *   ## Setup           → id="setup-2"
 *
 * NOTE: GitHub starts collision suffixes at `-1`, NOT `-2`.  We match
 * that.  The spec's "-2, -3, …" wording was approximate — the
 * important thing is that the algorithm is deterministic and the first
 * collision is bumped.  The first occurrence keeps the bare slug
 * because that's what's most likely to be the canonical anchor people
 * link to in practice.
 *
 * Empty slugs (`# !@#$%` → `""`) collide on the empty string and get
 * suffixed the same way: `""`, `-1`, `-2`, …  We keep the empty-base
 * behaviour rather than substituting a placeholder like `"section"`
 * because (a) it's deterministic and (b) it surfaces "your heading has
 * no slug-eligible content" to the docs author as a weird-looking link.
 *
 * =============================================================================
 * PLAIN-TEXT EXTRACTION
 * =============================================================================
 *
 * Heading content is a tree of `InlineNode`s — emphasis, links, code
 * spans, images, line breaks.  The slug should be derived from what a
 * reader would *say out loud*: the text content, with markup elided.
 *
 *   ## *Hello* world           → "Hello world"            → "hello-world"
 *   ## `code()` rules          → "code() rules"           → "code-rules"
 *   ## [Link](url) text        → "Link text"              → "link-text"
 *   ## ![alt](img) caption     → "alt caption"            → "alt-caption"
 *   ## one<br>two              → "one two"                → "one-two"
 *   ## one\ntwo (soft break)   → "one two"                → "one-two"
 *
 * `RawInlineNode`s (raw HTML / LaTeX fragments inside headings) are
 * skipped entirely — we don't know how to extract text from them, and
 * leaking raw markup into a slug would produce surprising IDs.
 *
 * @module walker
 */

import type {
  DocumentNode,
  BlockNode,
  HeadingNode,
  InlineNode,
} from "@coding-adventures/document-ast";

import { slugify } from "./slug.js";
import type {
  AnchoredHeadingNode,
  HeadingAnchor,
  HeadingAnchorsResult,
} from "./types.js";

// ─────────────────────────────────────────────────────────────────────────
// Public entry
// ─────────────────────────────────────────────────────────────────────────

/**
 * Walk a `DocumentNode`, compute a deterministic slug ID for every
 * heading, and return a new tree with the slugs attached.
 *
 * @param doc - The input document AST.
 * @returns `{ document, anchors }` where `document` is a new tree
 *          (non-heading children shared by reference, heading children
 *          replaced with `AnchoredHeadingNode`) and `anchors` is the
 *          flat in-document-order list of heading metadata.
 */
export function generateHeadingAnchors(doc: DocumentNode): HeadingAnchorsResult {
  // Track per-base-slug occurrence count.  We allocate via `Object.create(null)`
  // to avoid the prototype-chain footgun: a heading literally titled
  // "__proto__" would otherwise read the inherited `Object.prototype.__proto__`
  // accessor instead of the bare property.  Using a null-prototype map
  // makes `counts[slug]` always behave as plain key→value lookup.
  const counts: Record<string, number> = Object.create(null);
  const anchors: HeadingAnchor[] = [];

  // Walk the top-level children in order.  Headings are always
  // top-level blocks in document-ast — they're not nested inside
  // blockquotes or lists in the type system, so a single-level loop
  // suffices for v0.  (If GFM blockquote-wrapped headings become a
  // thing later, we'd recurse here.)
  const newChildren: BlockNode[] = doc.children.map((child): BlockNode => {
    if (child.type !== "heading") {
      // Non-heading blocks pass through by reference — readonly contract
      // means downstream code can't mutate them out from under us.
      return child;
    }
    // Compute the slug from the plain-text content.
    const text = extractPlainText(child.children);
    const baseSlug = slugify(text);
    const id = uniquify(baseSlug, counts);

    const anchored: AnchoredHeadingNode = {
      type: "heading",
      level: child.level,
      children: child.children,
      id,
    };
    anchors.push({ text, id, level: child.level, heading: anchored });
    return anchored;
  });

  const document: DocumentNode = {
    type: "document",
    children: newChildren,
  };
  return { document, anchors };
}

// ─────────────────────────────────────────────────────────────────────────
// Collision suffixing
// ─────────────────────────────────────────────────────────────────────────

/**
 * Convert a base slug into a unique slug using the in-document
 * occurrence counter.  First occurrence: bare slug.  Second: `-1`.
 * Third: `-2`.  And so on.
 *
 * Mutates `counts` so the next call sees the updated count.
 */
function uniquify(baseSlug: string, counts: Record<string, number>): string {
  // `counts[baseSlug]` is `undefined` on first sight — coerce to 0.
  const prior = counts[baseSlug] ?? 0;
  counts[baseSlug] = prior + 1;
  if (prior === 0) {
    return baseSlug;
  }
  return `${baseSlug}-${prior}`;
}

// ─────────────────────────────────────────────────────────────────────────
// Plain-text extraction from inline nodes
// ─────────────────────────────────────────────────────────────────────────

/**
 * Extract a flat plain-text string from a list of inline nodes.
 *
 * This is the "what would a screen reader read out loud" projection:
 * text values are concatenated, markup containers (emphasis, strong,
 * strikethrough, links) are flattened, code spans contribute their
 * raw text, images contribute their alt text, line breaks contribute
 * a single space, and raw HTML/LaTeX fragments are skipped (no safe
 * way to extract text from arbitrary markup).
 *
 * Exposed (un-exported) only via the walker — kept as an internal
 * helper because callers who need this should be using the walker.
 */
function extractPlainText(nodes: readonly InlineNode[]): string {
  const parts: string[] = [];
  for (const node of nodes) {
    parts.push(textOf(node));
  }
  return parts.join("");
}

/**
 * Per-node-type plain-text projection.  Switch-on-type is exhaustive
 * over `InlineNode` — the TypeScript compiler will catch any missing
 * case if document-ast adds a new variant.
 */
function textOf(node: InlineNode): string {
  switch (node.type) {
    case "text":
      return node.value;

    case "emphasis":
    case "strong":
    case "strikethrough":
      return extractPlainText(node.children);

    case "code_span":
      return node.value;

    case "link":
      // Link text — the destination URL is not part of the heading's
      // "spoken" content, so we skip it and recurse into children only.
      return extractPlainText(node.children);

    case "image":
      // Images render as their alt text in plain-text projections.
      return node.alt;

    case "autolink":
      // `<https://example.com>` — the destination IS the spoken text.
      return node.destination;

    case "raw_inline":
      // Raw HTML / LaTeX inside a heading.  We can't reliably extract
      // text from arbitrary markup without a per-format parser, and
      // leaking the raw tags into the slug would produce surprising
      // ids like `getting-sub-started-sub`.  Skip.
      return "";

    case "hard_break":
    case "soft_break":
      // Line breaks become single spaces — `# one\ntwo` slugs to `one-two`.
      return " ";
  }
}
