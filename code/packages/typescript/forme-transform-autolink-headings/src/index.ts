/**
 * @coding-adventures/forme-transform-autolink-headings
 *
 * FM00 v0 §5.3 transform — produce deterministic slug ids and
 * self-link anchors for every `HeadingNode` in a `DocumentNode`.
 *
 * Pure transform: `DocumentNode` → `HeadingSlug[]` (one entry per
 * heading, in document order).  Renderers consume the annotation
 * stream while walking the AST to emit
 *
 *   <h2 id="my-slug"><a href="#my-slug" class="forme-anchor">Heading</a></h2>
 *
 * ```ts
 * import { autolinkHeadings } from "@coding-adventures/forme-transform-autolink-headings";
 *
 * const slugs = autolinkHeadings(doc);
 * for (const { level, text, slug, anchorHref } of slugs) {
 *   console.log(`h${level} → ${anchorHref}  (${text})`);
 * }
 * ```
 *
 * Sub-helpers (`slugify`, `resolveCollisions`, `extractText`) are
 * re-exported for callers building custom transforms — e.g. a
 * TOC extractor that wants the same slug as this transform
 * generated, or a tooling step that validates external deep links
 * against the document's slug set.
 *
 * Fifth FM00 v0 stage package — joins `forme-feeds`,
 * `forme-opengraph`, `forme-index-renderer`, `forme-transforms`.
 *
 * @module index
 */

export { autolinkHeadings } from "./autolink.js";
export { slugify } from "./slugify.js";
export { resolveCollisions } from "./collisions.js";
export { extractText } from "./extract-text.js";
export type { HeadingSlug } from "./types.js";
