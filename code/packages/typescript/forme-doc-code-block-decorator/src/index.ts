/**
 * @coding-adventures/forme-doc-code-block-decorator
 *
 * Walk a `DocumentNode` AST and decorate every fenced code block
 * with the presentation metadata an HTML chrome / sidebar / static
 * site renderer needs:
 *
 *   - A copy-button hook (`copyable: true` flag, becomes a
 *     `data-copyable` attribute in the renderer).
 *   - A human-readable language label (`ts` → `"TypeScript"`,
 *     `py` → `"Python"`, etc.) — see `language-labels.ts` for the
 *     full alias table.
 *   - A filename badge extracted from a `// file: foo.ts` first-line
 *     hint in any of six comment styles (line-comment, hash, SQL
 *     double-dash, LaTeX percent, HTML, C block).  When extracted,
 *     the hint line is stripped from the code body.
 *   - An optional line-number gutter flag (`lineNumbers: true`)
 *     when the caller opts in via `decorateCodeBlocks(doc, { lineNumbers: true })`.
 *
 * Recurses into blockquotes, lists, and task-list items — code
 * blocks nested inside any of those still get decorated.
 *
 * Pure transform.  Capabilities: `[]`.  No `eval`, no `new Function`,
 * no `JSON.parse` reviver, no fs / network / env / shell.  Depends
 * only on `@coding-adventures/document-ast`.
 *
 * ```ts
 * import { decorateCodeBlocks } from "@coding-adventures/forme-doc-code-block-decorator";
 * import { parseCommonMark } from "@coding-adventures/commonmark-parser";
 *
 * const doc = parseCommonMark(
 *   "```ts\n" +
 *   "// file: src/auth.ts\n" +
 *   "export function login(user) { return user; }\n" +
 *   "```"
 * );
 *
 * const decorated = decorateCodeBlocks(doc, { lineNumbers: true });
 * const block = decorated.children[0];
 * // block.type           = "code_block"
 * // block.language       = "ts"
 * // block.value          = "export function login(user) { return user; }\n"
 * // block.copyable       = true
 * // block.languageLabel  = "TypeScript"
 * // block.filename       = "src/auth.ts"
 * // block.lineNumbers    = true
 * ```
 *
 * Fourth concrete DOC00 v0 package (after `forme-doc-frontmatter`,
 * `forme-doc-heading-anchors`, `forme-doc-toc-extractor`).
 *
 * @module index
 */

export { decorateCodeBlocks } from "./decorator.js";
export { extractFilenameHint } from "./filename.js";
export { languageLabel } from "./language-labels.js";
export type { DecoratedCodeBlockNode, DecorateOptions } from "./types.js";
