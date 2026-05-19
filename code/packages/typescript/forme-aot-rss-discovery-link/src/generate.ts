/**
 * generate.ts — main `generateFeedDiscoveryLinks` entry.
 *
 * Two-pass: validate every link's href/type first into a fresh
 * array, then emit the HTML tags joined by newlines.  Same
 * fail-fast posture as the sibling emitters.
 *
 * Output attribute order (deterministic): `rel` → `type` →
 * `title` → `href`.  Matches the convention used in WordPress,
 * Hugo, and most static-site generators.
 *
 * @module generate
 */

import { escapeHtmlAttr } from "./escape.js";
import type { FeedDiscoveryLink } from "./types.js";
import { validateFeedHref, validateFeedType } from "./validate.js";

interface ResolvedLink {
  readonly type: string;
  readonly href: string;
  readonly title: string | undefined;
}

/**
 * Generate one or more `<link rel="alternate">` discovery tags.
 *
 * Accepts a single `FeedDiscoveryLink` or an array.  Returns
 * the concatenated HTML string (one tag per line; no trailing
 * newline).
 *
 * Throws `TypeError` synchronously on validation failure
 * BEFORE any output is built.
 *
 * ```ts
 * generateFeedDiscoveryLinks({
 *   href: "/feed.xml",
 *   title: "My Blog",
 * });
 * // => <link rel="alternate" type="application/rss+xml" title="My Blog" href="/feed.xml">
 *
 * generateFeedDiscoveryLinks([
 *   { href: "/feed.xml",      type: "application/rss+xml",  title: "RSS" },
 *   { href: "/atom.xml",      type: "application/atom+xml", title: "Atom" },
 *   { href: "/feed.json",     type: "application/json",     title: "JSON Feed" },
 * ]);
 * // => three lines, one tag each
 * ```
 *
 * Reproducibility: same input → byte-identical output.  Input
 * is never mutated.
 */
export function generateFeedDiscoveryLinks(
  input: FeedDiscoveryLink | readonly FeedDiscoveryLink[],
): string {
  const links = Array.isArray(input) ? input : [input];

  // Validation pass.
  const resolved: ResolvedLink[] = new Array(links.length);
  for (let i = 0; i < links.length; i++) {
    const link = links[i];
    if (link === null || typeof link !== "object") {
      throw new TypeError(
        `forme-aot-rss-discovery-link: input[${i}] must be a non-null object; got ${typeof link}`,
      );
    }
    const type = link.type === undefined ? "application/rss+xml" : validateFeedType(link.type);
    const href = validateFeedHref(link.href);
    if (link.title !== undefined && typeof link.title !== "string") {
      throw new TypeError(
        `forme-aot-rss-discovery-link: input[${i}].title must be a string; got ${typeof link.title}`,
      );
    }
    resolved[i] = { type, href, title: link.title };
  }

  // Emit pass.
  const tags: string[] = new Array(resolved.length);
  for (let i = 0; i < resolved.length; i++) {
    tags[i] = renderTag(resolved[i]!);
  }
  return tags.join("\n");
}

/**
 * Render one `<link>` tag.  Attribute order: `rel` → `type` →
 * `title` → `href`.  Every value passes through
 * `escapeHtmlAttr` (belt-and-braces — `type` and `href` are
 * already validated, but escape ensures the tag stays
 * well-formed if the validators ever loosen).
 */
function renderTag(link: ResolvedLink): string {
  const parts: string[] = [
    `<link rel="alternate"`,
    `type="${escapeHtmlAttr(link.type)}"`,
  ];
  if (link.title !== undefined) {
    parts.push(`title="${escapeHtmlAttr(link.title)}"`);
  }
  parts.push(`href="${escapeHtmlAttr(link.href)}">`);
  return parts.join(" ");
}
