/**
 * GFM Parser — Internal Types
 *
 * This module defines types that are internal to the parser and not part of
 * the Document AST IR (@coding-adventures/document-ast). All AST node types
 * are imported from that package.
 *
 * The only types here are the link reference map structures used during Phase 1
 * (block parsing) and Phase 2 (inline parsing) to resolve `[text][label]` links.
 *
 * @module types
 */

// ─── Parse Options ─────────────────────────────────────────────────────────────

/**
 * Options passed to the `parse()` function.
 *
 * Reserved for future parser flags.
 */
export interface ParseOptions {
  readonly _future?: never;
}

// ─── Link Reference Map ────────────────────────────────────────────────────────

/**
 * A resolved link reference definition, keyed by normalized label.
 *
 * Populated during Phase 1 (block parsing) when `[label]: destination "title"`
 * definitions are encountered. Consumed during Phase 2 (inline parsing) to
 * resolve `[text][label]` and `[text][]` links.
 *
 * The key is the normalized label: trimmed, lowercased, and internal whitespace
 * collapsed. So `[Example]`, `[EXAMPLE]`, and `[  example  ]` all map to `"example"`.
 */
export interface LinkReference {
  readonly destination: string;
  readonly title: string | null;
}

/** Map of normalized link labels to their resolved reference data. */
export type LinkRefMap = Map<string, LinkReference>;
