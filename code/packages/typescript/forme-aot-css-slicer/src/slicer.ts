/**
 * slicer.ts — `slicePerPage(doc, pages, options)` (FM06 §3).
 *
 * For each page the renderer visited, the AOT compiler knows
 * exactly which `StyleRuleId`s that page's content referenced (the
 * `usedStyle` accumulator from FM01 §2.3.6).  This module takes the
 * full `StyleDocument` and a per-page `usedRuleIds` map and emits
 * a content-addressed CSS artefact per page, scoped so two pages'
 * CSS can be concatenated into one bundle without selector collisions.
 *
 * ## Per-page scoping (default)
 *
 * Every emitted rule's selector is prefixed by a per-page scope.
 * Default scope: `"#p-<8 hex chars of sha256(pageId)>"`.  The
 * choice of `#p-` (id selector + ASCII hyphen + 32-bit hex run)
 * keeps the prefix a valid CSS identifier under any reasonable
 * pageId without further sanitisation — the hash collapses all
 * pageId byte ranges into the alphabet `[0-9a-f]`.
 *
 * Pages that share `usedRuleIds` get identical CSS bodies but
 * different scopes — the `sha256` field of `CssArtifact` is over
 * the *unscoped* canonical bytes, so a downstream cache can
 * deduplicate by content.
 *
 * Callers can override the scope function via `options.scopePrefix`
 * to inline a different naming scheme (e.g. a route-derived slug).
 *
 * ## Reproducibility
 *
 * Same inputs → byte-identical output (FM03 contract preserved
 * end-to-end).  Page iteration order = caller's array order; we
 * do not sort.
 *
 * @module slicer
 */

import { createHash } from "node:crypto";
import {
  translateToCss,
} from "@coding-adventures/forme-style-to-css";
import type {
  StyleDocument, StyleRuleId, StyleWarning,
} from "@coding-adventures/forme-style-ir";

// ─── Public types ────────────────────────────────────────────────────────

/** One page's slicing input. */
export interface PageSlice {
  /** Stable page identifier — feeds the default scope hash. */
  readonly id: string;
  /** The `usedStyle` accumulator from the renderer (FM01 §2.3.6). */
  readonly usedRuleIds: readonly StyleRuleId[];
}

/** Options accepted by `slicePerPage`. */
export interface SliceOptions {
  /** Forwarded to the CSS translator. */
  readonly activeContexts: readonly string[];
  /**
   * Per-page scope prefix function.  Default: 8-hex-char sha256 of
   * the page id wrapped in `#p-…`.  Caller can supply their own
   * (e.g. for route-derived slugs) — the only contract is that the
   * function MUST be deterministic and MUST produce a valid CSS
   * selector fragment per FM04 §9.2's `scope` semantics.
   *
   * **Caller-trusted output**: the CSS translator's `scope` field
   * is concatenated verbatim into the generated CSS.  Custom
   * functions must already produce safe CSS.
   */
  readonly scopePrefix?: (pageId: string) => string;
}

/** What `slicePerPage` returns for each page. */
export interface CssArtifact {
  readonly pageId: string;
  /** The scoped CSS text. */
  readonly css: string;
  /** Which rule ids actually landed (post translator warn-skip). */
  readonly emittedRules: readonly StyleRuleId[];
  /** Translator warnings for this page. */
  readonly warnings: readonly StyleWarning[];
  /** `Buffer.byteLength(css, "utf8")` — for size budgets / reports. */
  readonly byteSize: number;
  /**
   * Hex-encoded sha256 of the *unscoped* canonical CSS bytes.
   * Pages that produce identical unscoped CSS share a `sha256` —
   * downstream caches can deduplicate by content.  Distinct from
   * the scoped `css` field which is per-page unique.
   */
  readonly sha256: string;
}

/** The full slicing result.  Map preserves caller's page order. */
export interface SliceResult {
  readonly artefacts: ReadonlyMap<string, CssArtifact>;
}

// ─── Public entry point ──────────────────────────────────────────────────

/**
 * Slice a `StyleDocument` into per-page CSS artefacts.
 *
 * For each page:
 *
 *   1. Compute the scope (default: `defaultScopePrefix(page.id)`).
 *   2. Run `translateToCss` once **unscoped** to get the content-
 *      addressed fingerprint (sha256) — pages with identical
 *      `usedRuleIds` produce identical bodies and can be
 *      deduplicated downstream.
 *   3. Run `translateToCss` a second time **with the scope** to
 *      produce the deliverable CSS text the page actually loads.
 *
 * The double translation is deliberate.  The cheaper alternative
 * (hashing the scoped output and stripping the prefix at lookup
 * time) couples the cache key to the scope choice; pages that
 * differ only in their scope-prefix function would then be
 * cache-misses despite being byte-identical at the rule level.
 * Per-page translate is O(rules-per-page) — typically tiny.
 */
export function slicePerPage(
  doc: StyleDocument,
  pages: readonly PageSlice[],
  options: SliceOptions,
): SliceResult {
  const scopeFn = options.scopePrefix ?? defaultScopePrefix;
  const artefacts = new Map<string, CssArtifact>();

  for (const page of pages) {
    // Step 1: unscoped translation for the content-addressed hash.
    const unscoped = translateToCss(doc, {
      activeContexts: options.activeContexts,
      usedRuleIds: page.usedRuleIds,
    });
    const sha256 = sha256Hex(unscoped.output);

    // Step 2: scoped translation for the deliverable.  Warnings
    // are identical between the two calls (warnings don't depend
    // on the scope); take them from `scoped` for consistency
    // with the css we emit.
    const scope = scopeFn(page.id);
    const scoped = translateToCss(doc, {
      activeContexts: options.activeContexts,
      usedRuleIds: page.usedRuleIds,
      scope,
    });

    const css = scoped.output;
    artefacts.set(page.id, {
      pageId: page.id,
      css,
      emittedRules: scoped.emittedRules,
      warnings: scoped.warnings,
      byteSize: Buffer.byteLength(css, "utf8"),
      sha256,
    });
  }

  return { artefacts };
}

// ─── Default scope helper ────────────────────────────────────────────────

/**
 * Default `pageId → scope` function: first 8 hex chars of
 * sha256(pageId), prefixed by `#p-`.
 *
 * Why 8 chars (32 bits)?  Birthday-collision odds at 32 bits hit
 * 1% around ~9k pages and 50% around ~65k — well above any
 * reasonable static-site page count.  Sites that need more
 * collision-resistance can override `options.scopePrefix`.
 *
 * Why `#` not `.`?  Id selectors have higher CSS specificity than
 * class selectors, so a per-page scope using `#` overrides any
 * unscoped descendants without `!important`.  The `p-` prefix
 * avoids leading-digit identifier issues (CSS identifiers may not
 * start with a digit).
 */
export function defaultScopePrefix(pageId: string): string {
  return `#p-${sha256Hex(pageId).slice(0, 8)}`;
}

// ─── Internals ───────────────────────────────────────────────────────────

function sha256Hex(s: string): string {
  return createHash("sha256").update(s, "utf8").digest("hex");
}
