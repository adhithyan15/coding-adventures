/**
 * types.ts — annotation types produced by the autolink-headings transform.
 *
 * The Document AST is immutable and closed (no `id` field on
 * `HeadingNode`), so this transform does not modify the document.
 * Instead it produces a parallel ordered list of slug annotations
 * that renderers consume in document order to emit
 *
 *   <h2 id="my-slug"><a href="#my-slug" class="forme-anchor">Heading text</a></h2>
 *
 * Why a list, not a `Map<HeadingNode, string>`?
 *
 *   1. **Determinism.**  Two equal-by-value documents that differ
 *      only in `HeadingNode` object identity (because each was
 *      parsed independently) produce the same `HeadingSlug[]`.
 *      A `Map` keyed by identity would not.
 *   2. **JSON-serialisable.**  Renderers run in separate processes
 *      from parsers in some Forme deployments; the annotation
 *      stream survives serialisation losslessly.
 *   3. **Position-coupled to a document walk.**  Renderers walk
 *      the AST and consume slugs in heading-encounter order.  An
 *      array indexed by encounter is the natural fit.
 *
 * @module types
 */

/**
 * One annotation entry per `HeadingNode` in document order.
 *
 *   - `level` — copy of `HeadingNode.level` (1-6).  Lets TOC
 *     consumers reconstruct hierarchy without a second AST walk.
 *   - `text` — flattened plain-text content of the heading's
 *     inline children.  Used by TOC consumers as the link label.
 *   - `slug` — the deterministic, collision-resolved id assigned
 *     to this heading.  Safe to interpolate into an HTML
 *     attribute value (only `[a-z0-9-]`).
 *   - `anchorHref` — `#${slug}`.  Provided so renderers don't
 *     re-concatenate (and accidentally introduce a URL-encoding
 *     bug).
 */
export interface HeadingSlug {
  readonly level: 1 | 2 | 3 | 4 | 5 | 6;
  readonly text: string;
  readonly slug: string;
  readonly anchorHref: string;
}
