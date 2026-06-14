/**
 * style-document.ts — the top-level Style IR shape (FM04 §5.1, §7, §8).
 *
 * `StyleDocument` is the "stylesheet" of a Forme document — a tokens
 * bag + a rules list + a contexts declaration + an optional theme
 * reference.  It's what a `StyleTranslator` consumes to emit CSS,
 * LaTeX, terminal ANSI, etc.
 *
 * `StyleRule` is the (selector, properties, optional context) triple
 * — the unit of style application.  Producers (theme stages, parser
 * plugins, editor surfaces) generate rules; producers also assign
 * the opaque `StyleRuleId` for per-page `usedStyle` tracking (the
 * AOT compiler's CSS-slicing input — FM01 §2.3.6 / FM06).
 *
 * `Theme` is itself a partial `StyleDocument` (token overrides +
 * extra rules).  Themes compose with a base StyleDocument via
 * deep-merge of tokens + append of rules — there's no separate
 * "theme format."
 *
 * @module style-document
 */

import type { JsonValue, ReadonlyRecord } from "@coding-adventures/forme-types";
import type { Selector } from "./selectors.js";
import type { StyleProperty } from "./properties.js";
import type { TokenSet } from "./tokens.js";

// ─── StyleRule ────────────────────────────────────────────────────────────

/**
 * Branded opaque id.  Producers mint unique ids; the AOT compiler
 * reads them back to slice per-page CSS.  Translators may use ids
 * as anchors but must not assume any internal structure beyond
 * uniqueness within a single `StyleDocument`.
 *
 * Branded so accidental coercion from a bare string (or another
 * branded id like `LogicalId`) is a compile error.
 */
export type StyleRuleId = string & { readonly __brand: "StyleRuleId" };

/**
 * One rule.  Producers are responsible for `id` uniqueness within a
 * `StyleDocument`; the validator enforces it.  An absent `context`
 * means "apply unconditionally."
 */
export interface StyleRule {
  readonly id: StyleRuleId;
  readonly selector: Selector;
  readonly properties: readonly StyleProperty[];
  /** Optional context (FM04 §6).  Absent ⇒ apply unconditionally. */
  readonly context?: string;
}

// ─── StyleDocument ────────────────────────────────────────────────────────

/**
 * The full Style IR value.  Replaces the FM01 §2.3.5 stub: the three
 * stub fields (`tokens`, `rules`, `theme`) are preserved; their
 * shapes are now precise.
 *
 * The discriminant `kind: "StyleDocument"` is present so values are
 * unambiguously tagged when carried alongside other IRs in a
 * `Document` (FM00 §3).
 */
export interface StyleDocument {
  readonly kind: "StyleDocument";
  readonly tokens: TokenSet;
  readonly rules: readonly StyleRule[];
  /**
   * Contexts THIS document declares.  Rules referencing contexts not
   * in this list still translate, but the translator emits a warning
   * — most likely a typo.  Empty array ⇒ no contexts declared.
   */
  readonly contexts: readonly string[];
  /** Optional named theme.  Resolution happens in a separate stage. */
  readonly theme: string | null;
  /** Open extension slot for plugin-contributed top-level data. */
  readonly extensions?: ReadonlyRecord<string, JsonValue>;
}

// ─── Theme ────────────────────────────────────────────────────────────────

/**
 * A theme overlay.  Partial token overrides + appended rules.
 *
 * Composition (FM04 §7.2):
 * 1. Start with the base `StyleDocument`.
 * 2. Deep-merge `theme.tokens` over the base tokens (per-named-token
 *    override; missing entries stay at base value).
 * 3. Append `theme.rules` to `rules` — they inherit later-in-order
 *    specificity per §4.9, so theme rules naturally override base
 *    rules with overlapping selectors.
 *
 * `Theme` is itself Style IR — there's no separate format and no
 * "theme parser."  Theme-producing stages emit `Theme` values; the
 * orchestrator's theme registry (in-memory in v0) holds them by name.
 */
export interface Theme {
  readonly name: string;
  /** Sparse token overrides.  Each sub-record is also sparse. */
  readonly tokens?: Partial<TokenSet>;
  /** Additional rules.  Appended after base rules. */
  readonly rules?: readonly StyleRule[];
}

// ─── Constructors ─────────────────────────────────────────────────────────

/** Brand a string as a `StyleRuleId`.  Producer-side ergonomics. */
export function styleRuleId(s: string): StyleRuleId {
  return s as StyleRuleId;
}

/**
 * Build an empty `StyleDocument` — present-but-empty buckets across
 * the board.  Useful as a starting point that doesn't trip the
 * validator's required-field checks.
 */
export function emptyStyleDocument(): StyleDocument {
  return {
    kind: "StyleDocument",
    tokens: {
      colors: {},
      typography: {
        families: {},
        scale: {},
        weights: {},
        leading: {},
        tracking: {},
      },
      space: {},
      radii: {},
      shadows: {},
    },
    rules: [],
    contexts: [],
    theme: null,
  };
}
