/**
 * selectors.ts — selector union (FM04 §4).
 *
 * Selectors describe *which document-AST nodes* a rule applies to.
 * The Style IR's selector vocabulary is intentionally smaller than
 * CSS — twelve forms vs CSS's 30+ — because documents don't need
 * the legacy combinators web pages have collected.
 *
 * Three families:
 * 1. **Identity selectors** — `node-type`, `node-type-level`,
 *    `custom-kind`, `tag`, `id`, `role`.  Match a single node by
 *    some intrinsic property.
 * 2. **Position selectors** — `nth`.  Match by index within a
 *    parent's children sequence.
 * 3. **Structural relation selectors** — `child-of`,
 *    `descendant-of`, `adjacent`.  Match based on tree relations.
 * 4. **Composition** — `and`, `or`, `not`.  Combine selectors.
 *
 * Specificity (FM04 §4.9) is *source-order* only — later rules
 * win.  No CSS-style "ID beats class" specificity calculation.  The
 * rationale: CSS specificity is famously surprising; source order
 * is predictable.
 *
 * @module selectors
 */

// ─── Identity selectors ───────────────────────────────────────────────────

/** Match every node of a given DocumentAst block- or inline-node type. */
export interface NodeTypeSelector {
  readonly kind: "node-type";
  /** A DocumentAst type name: "paragraph", "blockquote", "code_block", ... */
  readonly type: string;
}

/** Match a heading at a specific level.  Lifted to first-class because
 *  heading-level styling is universal. */
export interface NodeTypeLevelSelector {
  readonly kind: "node-type-level";
  readonly type: "heading";
  readonly level: 1 | 2 | 3 | 4 | 5 | 6;
}

/** Match a plugin-registered content kind (e.g. "callout", "youtube-embed"). */
export interface CustomKindSelector {
  readonly kind: "custom-kind";
  readonly customKind: string;
}

/** Match by content tag (frontmatter or ancestor-frontmatter declared). */
export interface TagSelector {
  readonly kind: "tag";
  readonly tag: string;
}

/** Match by node id (one-off override target). */
export interface IdSelector {
  readonly kind: "id";
  readonly id: string;
}

/** Match by ARIA-style semantic role (e.g. "navigation", "byline"). */
export interface RoleSelector {
  readonly kind: "role";
  readonly role: string;
}

// ─── Position selector ────────────────────────────────────────────────────

/** Match by *index within parent's children that satisfy `of`*.
 *  Index is 0-based; or a CSS-style `an + b` formula. */
export interface NthSelector {
  readonly kind: "nth";
  readonly of: Selector;
  readonly n: number | NthFormula;
}

/** CSS-style `an+b`.  `fromEnd: true` counts from the end. */
export interface NthFormula {
  readonly a: number;
  readonly b: number;
  readonly fromEnd?: boolean;
}

// ─── Structural relation selectors ────────────────────────────────────────

/** A node that is a *direct* child of an outer-selector match. */
export interface ChildOfSelector {
  readonly kind: "child-of";
  readonly parent: Selector;
  readonly child: Selector;
}

/** A node that has *any ancestor* matching the outer selector. */
export interface DescendantOfSelector {
  readonly kind: "descendant-of";
  readonly ancestor: Selector;
  readonly descendant: Selector;
}

/** A node immediately following (sibling-wise) a previous match. */
export interface AdjacentSelector {
  readonly kind: "adjacent";
  readonly previous: Selector;
  readonly following: Selector;
}

// ─── Composition ──────────────────────────────────────────────────────────

/** Match nodes satisfying *every* inner selector. */
export interface AndSelector {
  readonly kind: "and";
  readonly all: readonly Selector[];
}

/** Match nodes satisfying *any* inner selector. */
export interface OrSelector {
  readonly kind: "or";
  readonly any: readonly Selector[];
}

/** Match nodes *not* satisfying the inner selector. */
export interface NotSelector {
  readonly kind: "not";
  readonly inner: Selector;
}

// ─── The union ────────────────────────────────────────────────────────────

/** The closed selector union.  Adding a variant is a backward-compatible
 *  minor-version bump (per FM04 §0.3); removing one is a major. */
export type Selector =
  | NodeTypeSelector
  | NodeTypeLevelSelector
  | CustomKindSelector
  | TagSelector
  | IdSelector
  | RoleSelector
  | NthSelector
  | ChildOfSelector
  | DescendantOfSelector
  | AdjacentSelector
  | AndSelector
  | OrSelector
  | NotSelector;

/** Frozen list of selector kind discriminants.  Used by the validator. */
export const SELECTOR_KINDS = Object.freeze([
  "node-type",
  "node-type-level",
  "custom-kind",
  "tag",
  "id",
  "role",
  "nth",
  "child-of",
  "descendant-of",
  "adjacent",
  "and",
  "or",
  "not",
] as const);

export type SelectorKind = (typeof SELECTOR_KINDS)[number];

// ─── Convenience constructors ─────────────────────────────────────────────
//
// Authoring a selector tree by hand gets noisy (every node needs `kind`).
// These helpers preserve the static union but are nicer to type.

export const sel = {
  type:     (type: string): NodeTypeSelector => ({ kind: "node-type", type }),
  heading:  (level: 1 | 2 | 3 | 4 | 5 | 6): NodeTypeLevelSelector => ({ kind: "node-type-level", type: "heading", level }),
  custom:   (customKind: string): CustomKindSelector => ({ kind: "custom-kind", customKind }),
  tag:      (tag: string): TagSelector => ({ kind: "tag", tag }),
  id:       (id: string): IdSelector => ({ kind: "id", id }),
  role:     (role: string): RoleSelector => ({ kind: "role", role }),
  nth:      (of: Selector, n: number | NthFormula): NthSelector => ({ kind: "nth", of, n }),
  childOf:      (parent: Selector, child: Selector): ChildOfSelector => ({ kind: "child-of", parent, child }),
  descendantOf: (ancestor: Selector, descendant: Selector): DescendantOfSelector => ({ kind: "descendant-of", ancestor, descendant }),
  adjacent: (previous: Selector, following: Selector): AdjacentSelector => ({ kind: "adjacent", previous, following }),
  and:      (...all: Selector[]): AndSelector => ({ kind: "and", all }),
  or:       (...any: Selector[]): OrSelector => ({ kind: "or", any }),
  not:      (inner: Selector): NotSelector => ({ kind: "not", inner }),
} as const;
