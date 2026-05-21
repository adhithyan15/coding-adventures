/**
 * @coding-adventures/forme-doc-frontmatter
 *
 * Strip YAML or TOML frontmatter from a markdown source string
 * and return both the (frontmatter-free) body and the parsed
 * metadata object.  The body then goes to `commonmark-parser`
 * or `gfm-parser` for HTML rendering; the metadata is consumed
 * by site-structure packages (sidebar position, title overrides,
 * draft flag, etc.).
 *
 * Pure transform.  Capabilities: `[]`.  Tiny in-house YAML and
 * TOML parsers; no `eval`, no `new Function`, no prototype
 * pollution.
 *
 * ```ts
 * import { extractFrontmatter } from "@coding-adventures/forme-doc-frontmatter";
 *
 * const md = `---\ntitle: Hello\ndate: 2026-05-20\ntags: [a, b]\n---\n# Body`;
 * const { body, frontmatter, format } = extractFrontmatter(md);
 * // body        = "# Body"
 * // frontmatter = { title: "Hello", date: "2026-05-20", tags: ["a", "b"] }
 * // format      = "yaml"
 * ```
 *
 * First concrete DOC00 v0 package.
 *
 * @module index
 */

export { extractFrontmatter } from "./extract.js";
export { parseYaml } from "./yaml.js";
export { parseToml } from "./toml.js";
export type { FrontmatterResult } from "./types.js";
