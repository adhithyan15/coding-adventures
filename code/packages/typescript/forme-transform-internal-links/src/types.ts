/**
 * types.ts — resolver signature and option bag.
 *
 * The resolver is the caller-supplied bridge between an internal
 * slug (the path-style reference an author wrote in Markdown) and
 * the canonical URL the renderer should emit.  Resolvers
 * typically consult the site's manifest:
 *
 * ```ts
 * function resolve(slug: string): string | null {
 *   const entry = manifest.byPath.get(slug);
 *   return entry ? entry.canonicalUrl : null;
 * }
 * ```
 *
 * Returning `null` signals "I don't know about this slug" — the
 * `unresolved` option decides what happens next.
 *
 * @module types
 */

/**
 * Caller-supplied function mapping an internal slug
 * (`/about`, `/blog/post`) to its canonical URL or `null` if
 * unresolved.  The resolver is invoked once per internal
 * `LinkNode` in the document.
 *
 * Contract:
 *   - **Pure.**  Must return the same result for the same slug
 *     every call (or reproducibility breaks).
 *   - **Synchronous.**  No I/O — manifest lookups are in-memory.
 *   - **Validated downstream.**  Resolved URLs must match
 *     `^https?://` once the transform emits them — anything else
 *     is rejected (defence against a malicious or buggy
 *     resolver).
 */
export type SlugResolver = (slug: string) => string | null;

/**
 * What to do with an internal link whose slug the resolver
 * returns `null` for.
 *
 *   - `"keep"` (default) — leave the original `/slug` in
 *     `LinkNode.destination`.  Browsers will follow it as a
 *     site-relative path; works on most static hosts.
 *   - `"strip"` — replace the `LinkNode` with its inline
 *     children (drop the link wrapper).  Useful when the renderer
 *     refuses to emit broken `<a href>`s.
 *   - `"throw"` — throw `Error` immediately.  Useful in
 *     pre-publish validation: an unresolvable link is a content
 *     bug.
 *
 * Note: `"strip"` only applies to inline `LinkNode`s, not to the
 * other URL-bearing inline shapes (image destinations, autolink
 * destinations).  Those are passed through unchanged regardless
 * of this option.
 */
export type UnresolvedPolicy = "keep" | "strip" | "throw";

/**
 * Options for the link rewriter.
 *
 *   - `unresolved` — see `UnresolvedPolicy`.  Defaults to
 *     `"keep"` (least surprising for callers wiring up a
 *     resolver mid-development).
 *
 * Future v1 options that v0 explicitly defers:
 *
 *   - `internalPredicate` — caller-defined "is this an internal
 *     link" check (currently hardcoded to "starts with `/` but
 *     not `//`").
 *   - `transformImageSrc` — extend rewriting to `ImageNode`
 *     destinations (image rewrite is a separate spec transform).
 */
export interface InternalLinksOptions {
  readonly unresolved?: UnresolvedPolicy;
}
