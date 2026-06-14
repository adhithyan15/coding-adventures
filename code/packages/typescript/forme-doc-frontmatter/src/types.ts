/**
 * types.ts — public signatures for the frontmatter extractor.
 *
 * @module types
 */

/**
 * Result of `extractFrontmatter`.  `body` is the markdown source
 * with the frontmatter block (and its delimiters) stripped.
 * `frontmatter` is the parsed metadata object, or `null` if
 * the input had no frontmatter.  `format` indicates which
 * frontmatter syntax was detected.
 */
export interface FrontmatterResult {
  readonly body: string;
  readonly frontmatter: Record<string, unknown> | null;
  readonly format: "yaml" | "toml" | "none";
}
