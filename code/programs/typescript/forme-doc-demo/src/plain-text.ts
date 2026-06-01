/**
 * plain-text.ts — extract plain-text content from a DocumentNode
 * AST, so the search-index-builder can index page bodies without
 * indexing markdown syntax noise.
 *
 * Why a tiny dedicated walker instead of regex-stripping the
 * markdown source?
 *
 *   - The AST is already parsed and trusted; walking it costs
 *     nothing.
 *   - Regex-stripping fenced code blocks, link syntax, image
 *     refs, etc. is the kind of "looks easy, becomes a
 *     rats-nest" job we explicitly avoid in this repo.
 *   - Walking matches what the index actually wants — paragraph
 *     text, heading text, list-item text, alt-text on images,
 *     etc. — and naturally skips structural nodes that aren't
 *     content.
 *
 * Capability `[]`.  Pure data walk.
 */

/**
 * Maximum recursion depth.  Markdown documents in practice nest
 * at most a handful of levels (a blockquote containing a list
 * containing nested lists is already extreme); 1024 is two
 * orders of magnitude past that.  The cap protects against a
 * pathological or malformed AST with cyclic `children`
 * references which would otherwise stack-overflow Node's
 * default call stack (~10k frames).
 */
const MAX_DEPTH = 1024;

/**
 * Recursively pull every `text` field out of an AST node and
 * its descendants, joined by spaces.  Plain conservative walk;
 * we treat unknown node shapes as opaque containers and
 * recurse into `children` if present.
 *
 * Bounded by `MAX_DEPTH` — defends against malformed ASTs with
 * cyclic `children` references.  Today's input source
 * (`commonmark-parser`) cannot produce cycles, so the cap is
 * defence-in-depth only.
 */
export function plainText(node: unknown): string {
  const buf: string[] = [];
  walk(node, buf, 0);
  return buf.join(" ").replace(/\s+/g, " ").trim();
}

function walk(node: unknown, buf: string[], depth: number): void {
  if (depth > MAX_DEPTH) return;
  if (node === null || node === undefined) return;
  if (typeof node !== "object") return;
  const n = node as { type?: unknown; text?: unknown; value?: unknown; children?: unknown };
  // The DocumentNode IR uses TWO different field names depending
  // on node type:
  //   - `TextNode`, `CodeBlockNode`, `RawInlineNode`, etc. use
  //     `value: string` for their textual payload.
  //   - Older / sibling ASTs sometimes use `text: string`.
  // Accept both so we work across the spectrum of nodes that
  // pass through this demo's content pipeline.
  if (typeof n.text === "string") {
    buf.push(n.text);
  }
  if (typeof n.value === "string") {
    buf.push(n.value);
  }
  // Recurse into children if the node has any.  Most DocumentNode
  // variants use `children: Node[]`; some inline nodes (TextNode,
  // CodeSpanNode) have no children — `walk` is a no-op on those.
  if (Array.isArray(n.children)) {
    for (let i = 0; i < n.children.length; i++) {
      walk(n.children[i], buf, depth + 1);
    }
  }
}
