/**
 * compose.ts — `composeWithTheme(base, theme)` per FM04 §7.2.
 *
 * A `Theme` is a sparse overlay on a `StyleDocument`: a partial
 * `TokenSet` (per-named-token overrides) plus an optional appended
 * `rules` list.  Composition is mechanical:
 *
 *   1. Start with `base`.
 *   2. **Deep-merge** `theme.tokens` over `base.tokens` — per-named
 *      override; missing entries stay at their base value.  The merge
 *      stops at *named* tokens; we don't recursively deep-merge the
 *      *value* (you can't merge half of an rgb color with half of an
 *      hsl one — overriding a token swaps the whole value).
 *   3. **Append** `theme.rules` to `base.rules` in order.  Per FM04
 *      §4.9, later-in-source wins on equal-specificity ties — so
 *      theme rules naturally override base rules with overlapping
 *      selectors without any special "cascade" machinery here.
 *   4. `kind`, `contexts`, `theme`, and `extensions` are preserved
 *      from `base` (themes don't redeclare contexts or pivot the
 *      whole document's identity).
 *
 * ## Why a custom deep-merge instead of `{ ...a, ...b }`?
 *
 * `TokenSet` has *two* levels of named-record nesting (top-level
 * buckets + per-bucket entries, plus a third level inside
 * `typography`).  A single spread only merges the top level — it
 * would overwrite an entire `colors` bucket if the theme contributed
 * any colors at all, wiping out base colors not mentioned in the
 * theme.  Bucket-aware merge is the correct semantics per §7.2.
 *
 * ## Security note — prototype-pollution defence
 *
 * The merge walks user-supplied keys.  We refuse `__proto__`,
 * `constructor`, and `prototype` unconditionally, and only copy
 * **own** properties.  In practice the `forme-style-ir` validator
 * already restricts token-name keys via the token-grammar
 * (dotted-identifier), but a stage that hands us a hand-rolled
 * `Theme` (bypassing the validator) can't poison the base
 * `Object.prototype` through us.
 *
 * @module compose
 */

import type {
  ReadonlyRecord,
} from "@coding-adventures/forme-types";
import type {
  StyleDocument, StyleRule, Theme,
  TokenSet, TypographyTokens,
} from "@coding-adventures/forme-style-ir";

/** Keys we refuse to follow during deep-merge (proto-pollution shields). */
const FORBIDDEN_KEYS = new Set(["__proto__", "constructor", "prototype"]);

/**
 * Compose a theme onto a base `StyleDocument`, returning a new merged
 * `StyleDocument`.  Inputs are not mutated.
 *
 * Token override is **per-named-entry** within each bucket; rules
 * are **appended** so the theme's rules trail the base's (later wins
 * per FM04 §4.9 specificity).
 */
export function composeWithTheme(
  base: StyleDocument,
  theme: Theme,
): StyleDocument {
  const tokens = mergeTokens(base.tokens, theme.tokens);
  const rules: readonly StyleRule[] = theme.rules
    ? [...base.rules, ...theme.rules]
    : base.rules;

  // Preserve extensions verbatim — themes don't merge into them in v0.
  // (Plugin-contributed extension data is opaque; a plugin that wants
  // theme-aware extension data exposes its own composition helper.)
  return {
    kind: "StyleDocument",
    tokens,
    rules,
    contexts: base.contexts,
    theme: base.theme,
    ...(base.extensions !== undefined ? { extensions: base.extensions } : {}),
  };
}

// ─── TokenSet merge ──────────────────────────────────────────────────────

function mergeTokens(
  base: TokenSet,
  overlay: Partial<TokenSet> | undefined,
): TokenSet {
  if (overlay === undefined) return base;

  // Each bucket is a flat ReadonlyRecord<string, V> — except
  // `typography`, which is itself a record of records.  Handle that
  // one specially; the rest share a generic per-entry override.
  return {
    colors:     mergeRecord(base.colors,    overlay.colors),
    typography: mergeTypography(base.typography, overlay.typography),
    space:      mergeRecord(base.space,     overlay.space),
    radii:      mergeRecord(base.radii,     overlay.radii),
    shadows:    mergeRecord(base.shadows,   overlay.shadows),
    ...(base.extensions !== undefined || overlay.extensions !== undefined
      ? { extensions: mergeRecord(base.extensions ?? {}, overlay.extensions) }
      : {}),
  };
}

function mergeTypography(
  base: TypographyTokens,
  overlay: Partial<TypographyTokens> | undefined,
): TypographyTokens {
  if (overlay === undefined) return base;
  return {
    families: mergeRecord(base.families, overlay.families),
    scale:    mergeRecord(base.scale,    overlay.scale),
    weights:  mergeRecord(base.weights,  overlay.weights),
    leading:  mergeRecord(base.leading,  overlay.leading),
    tracking: mergeRecord(base.tracking, overlay.tracking),
  };
}

/**
 * Per-named-entry override: copy own keys from `base`, then
 * overwrite with own keys from `overlay`.  Forbidden keys
 * (`__proto__`, `constructor`, `prototype`) are silently dropped —
 * the validator's grammar should have caught them already, but
 * belt-and-braces.
 */
function mergeRecord<V>(
  base: ReadonlyRecord<string, V>,
  overlay: ReadonlyRecord<string, V> | undefined,
): ReadonlyRecord<string, V> {
  if (overlay === undefined) return base;
  const out: Record<string, V> = Object.create(null);
  copyOwn(base, out);
  copyOwn(overlay, out);
  return out;
}

function copyOwn<V>(
  src: ReadonlyRecord<string, V>,
  dst: Record<string, V>,
): void {
  for (const key of Object.keys(src)) {
    if (FORBIDDEN_KEYS.has(key)) continue;
    if (!Object.prototype.hasOwnProperty.call(src, key)) continue;
    dst[key] = src[key]!;
  }
}
