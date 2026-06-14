/**
 * title.ts — derive a page title from a ContentNode.
 *
 * Three-stage fallback (highest priority first):
 *
 *   1. `frontmatter.title` — author intent wins.  This is what the
 *      collector copies into `CollectionEntry.overlay.title` too, so
 *      the renderer and the collector stay in sync.
 *   2. First `<h1>` in the document — the conventional Markdown
 *      pattern of writing the title as a level-1 heading.
 *   3. The slug — last resort, always non-empty (slugify falls back
 *      to "untitled"), so a title is always produced.
 *
 * Edge cases the implementation guards:
 *   - frontmatter values are JsonValue, not necessarily strings — we
 *     only accept strings (and only non-empty ones).
 *   - The h1 may contain inline children that need flattening to text.
 *     For v0 we walk the inline tree gathering `text.value` and
 *     `code.value`; everything else is ignored.  That covers the
 *     overwhelming common case (`# Hello *world*`).
 *
 * @module title
 */

import type { DocumentNode } from "@coding-adventures/document-ast";
import type { ContentNode, JsonValue } from "@coding-adventures/forme-types";

/**
 * Resolve a title for a node.  Always returns a non-empty string.
 */
export function deriveTitle(node: ContentNode, slug: string): string {
  const fromFrontmatter = stringField(node.frontmatter, "title");
  if (fromFrontmatter !== null) return fromFrontmatter;
  const fromH1 = firstH1Text(node.document);
  if (fromH1 !== null && fromH1.length > 0) return fromH1;
  return slug;
}

function stringField(
  fm: { readonly [k: string]: JsonValue | undefined },
  key: string,
): string | null {
  const v = fm[key];
  return (typeof v === "string" && v.length > 0) ? v : null;
}

/**
 * Walk the document looking for the first `heading` of level 1.  Once
 * found, flatten its inline children to plain text.  Returns null if
 * no h1 exists (e.g. a document whose first heading is h2).
 */
function firstH1Text(doc: DocumentNode): string | null {
  for (const block of doc.children) {
    // Narrow: the heading discriminator is the only one we care about.
    if ((block as { type: string }).type === "heading") {
      const heading = block as { level: number; children: readonly unknown[] };
      if (heading.level === 1) {
        return flattenInline(heading.children).trim();
      }
    }
  }
  return null;
}

/**
 * Recursive walk of an inline subtree, concatenating any `text.value`
 * / `code.value` it finds.  Everything else (links, emphasis wrappers)
 * recurses into its own children.  Soft / hard breaks become spaces
 * so "first\nline" → "first line" rather than "firstline".
 */
function flattenInline(nodes: readonly unknown[]): string {
  let out = "";
  for (const n of nodes) {
    const node = n as { type: string; value?: string; children?: readonly unknown[] };
    switch (node.type) {
      case "text":
      case "code":
        out += node.value ?? "";
        break;
      case "soft_break":
      case "hard_break":
        out += " ";
        break;
      default:
        if (node.children) out += flattenInline(node.children);
    }
  }
  return out;
}
