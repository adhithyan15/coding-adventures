/**
 * types.ts — public signatures for the syntax-highlighter v0 stub.
 *
 * The TYPES here are intended to be FINAL — v1's real TextMate-grammar
 * engine will populate `highlighted` with a richer span sequence, but
 * downstream consumers (HTML renderer, page-shell) can be written
 * today against this stable shape.
 *
 * @module types
 */

import type { CodeBlockNode } from "@coding-adventures/document-ast";

/**
 * Token-type classification for a highlighted span.  The set is
 * deliberately broad — matches what TextMate-style themes typically
 * stylise — so v1's engine has somewhere to slot every token kind
 * without breaking the contract.
 *
 * v0 only ever emits `"plain"`; the other variants exist purely for
 * type-system stability.
 */
export type TokenType =
  /** Unstyled text — what v0 emits for every span. */
  | "plain"
  /** Language keywords: `if`, `return`, `def`, `fn`, `class`, … */
  | "keyword"
  /** String literals (any quoting). */
  | "string"
  /** Numeric literals (int, float, hex, bin, oct). */
  | "number"
  /** Comments (any style). */
  | "comment"
  /** Operators: `+`, `-`, `=`, `&&`, `=>`, … */
  | "operator"
  /** Punctuation: braces, parens, brackets, commas, semicolons. */
  | "punctuation"
  /** User-defined identifiers (variables, parameters). */
  | "identifier"
  /** Function names (call sites and definitions). */
  | "function"
  /** Type names (classes, interfaces, type aliases). */
  | "type"
  /** Built-in constants: `true`, `false`, `null`, `undefined`, `nil`. */
  | "constant"
  /** HTML / XML tag names. */
  | "tag"
  /** HTML / XML attribute names. */
  | "attribute"
  /** Regex literals. */
  | "regex";

/**
 * One contiguous run of text classified as a single token type.
 *
 * Spans MUST tile the code block exactly: concatenating every
 * `span.value` in order reconstructs the original `CodeBlockNode.value`
 * byte-for-byte.  This is the renderer's invariant — if `value` and
 * `highlighted` ever disagree the page shows wrong code.  v0 enforces
 * it trivially (one span = whole block); v1's engine will enforce it
 * via an exhaustive lexer that never drops a character.
 */
export interface HighlightSpan {
  readonly type: TokenType;
  readonly value: string;
}

/**
 * A `CodeBlockNode` augmented with a `highlighted` span sequence.
 *
 * Extends `CodeBlockNode` (not `DecoratedCodeBlockNode`) on the type
 * side so callers can highlight raw, decorated, or any future
 * super-type of code blocks — the highlighter is composition-friendly.
 * At runtime, any extra fields on the input node ride through
 * unchanged (object spread), so a decorated block stays decorated
 * after highlighting.
 */
export interface HighlightedCodeBlockNode extends CodeBlockNode {
  readonly highlighted: readonly HighlightSpan[];
}

/**
 * Options for `highlightCodeBlocks`.  Empty in v0 — reserved for
 * future theme / language-override / per-language enable-disable
 * options without breaking the call signature.
 */
export interface HighlightOptions {
  /**
   * Reserved.  v0 ignores this; v1 will accept a theme name (e.g.
   * `"github-light"`, `"monokai"`) that influences span colouring
   * downstream.  Specifying it in v0 is a no-op but accepted for
   * forward-compatibility.
   */
  readonly theme?: string;
}
