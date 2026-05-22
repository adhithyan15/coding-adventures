/**
 * @coding-adventures/forme-doc-syntax-highlighter
 *
 * v0 interface-first stub for the documentation-site syntax
 * highlighter.  Walks a `DocumentNode` and attaches a
 * `highlighted: HighlightSpan[]` field to every code block.
 *
 * **v0 doesn't actually highlight anything** — every block gets a
 * single `plain` span covering its full text (or `[]` for empty
 * blocks).  The TYPE-LEVEL CONTRACT (`HighlightSpan`,
 * `HighlightedCodeBlockNode`, `TokenType`) is FINAL and downstream
 * consumers (HTML renderer, page-shell) can be built against it
 * RIGHT NOW.  v1 will swap in the real TextMate-grammar engine
 * (per the DOC00 spec) without changing any signatures.
 *
 * The decision to ship a stub instead of a real engine for v0 is
 * documented in `CHANGELOG.md` — short version: a real engine is a
 * v1-sized effort (thousands of lines, per-language grammar
 * bundles, theme system, scope-stack tokeniser), and there's
 * meaningful value in unblocking downstream renderers with the
 * stable contract today.
 *
 * Pure transform.  Capabilities: `[]`.  No `eval`, no `new Function`,
 * no `JSON.parse` reviver, no fs / network / env / shell.  v1's
 * grammar engine will also be pure-transform — TextMate grammars
 * are static data, not code.
 *
 * ```ts
 * import { highlightCodeBlocks } from "@coding-adventures/forme-doc-syntax-highlighter";
 * import { parseCommonMark } from "@coding-adventures/commonmark-parser";
 *
 * const doc = parseCommonMark("```ts\nconst x = 1;\n```");
 * const result = highlightCodeBlocks(doc);
 * const block = result.children[0];
 * // block.type       = "code_block"
 * // block.language   = "ts"
 * // block.value      = "const x = 1;\n"
 * // block.highlighted = [{ type: "plain", value: "const x = 1;\n" }]
 * //                     (v1 will emit a richer span sequence here)
 * ```
 *
 * Fifth concrete DOC00 v0 package (after `forme-doc-frontmatter`,
 * `forme-doc-heading-anchors`, `forme-doc-toc-extractor`,
 * `forme-doc-code-block-decorator`).
 *
 * @module index
 */

export { highlightCodeBlocks, highlight } from "./highlighter.js";
export { SUPPORTED_LANGUAGES, isSupportedLanguage } from "./supported-languages.js";
export type {
  HighlightSpan,
  HighlightedCodeBlockNode,
  HighlightOptions,
  TokenType,
} from "./types.js";
