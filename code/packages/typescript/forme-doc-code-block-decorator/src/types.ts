/**
 * types.ts — public signatures for the code-block decorator.
 *
 * @module types
 */

import type { CodeBlockNode } from "@coding-adventures/document-ast";

/**
 * A `CodeBlockNode` augmented with presentation metadata.
 *
 * Structurally a `CodeBlockNode` (same `type`, `language`, `value`)
 * plus four added fields that downstream renderers consume:
 *
 *   - `copyable` — always `true` in v0; HTML renderers attach a
 *     `data-copyable` attribute the client-side copy-button shim
 *     listens for.  Future versions may flip this off via opt-out
 *     comments in the source.
 *
 *   - `languageLabel` — the human-readable language name to display
 *     in the block's chrome (e.g. the `TypeScript` text on a
 *     "Copy" / "TypeScript" button row).  Derived from the raw
 *     `language` field via a small alias table — `"ts"` →
 *     `"TypeScript"`, `"py"` → `"Python"`, etc.  If `language` is
 *     `null` or unrecognised, this falls back to the raw string
 *     (or `null` when language is `null`).
 *
 *   - `filename` — extracted from the first line of `value` if it
 *     matches a `// file: foo.ts` style hint in any of six comment
 *     styles (C/C++/JS, hash, HTML, C-block, SQL, LaTeX).  When
 *     extracted, the hint line is stripped from `value`.  `null`
 *     when no hint is present.
 *
 *   - `lineNumbers` — `true` iff the caller opted in via
 *     `decorateCodeBlocks(doc, { lineNumbers: true })`.  Off by
 *     default; renderers either emit a `<ol>`-style gutter or skip
 *     it entirely.
 *
 * The shape is JSON-friendly — no AST references, no symbols, no
 * Map/Set values.  `JSON.stringify`-based caches can serialise it
 * directly.
 */
export interface DecoratedCodeBlockNode extends CodeBlockNode {
  readonly copyable: true;
  readonly languageLabel: string | null;
  readonly filename: string | null;
  readonly lineNumbers: boolean;
}

/**
 * Options for `decorateCodeBlocks`.
 */
export interface DecorateOptions {
  /**
   * If `true`, every code block's `lineNumbers` field is set to
   * `true`, signalling the renderer to emit a gutter.  Default
   * `false`.
   *
   * v0 is binary — all-on or all-off per call.  Per-block opt-in
   * via in-source magic comments (`// linenos`) is intentionally
   * deferred to v1; opinions on the syntax differ across projects
   * and we'd rather not invent one prematurely.
   */
  readonly lineNumbers?: boolean;
}
