/**
 * compile.ts — the unified `compile(doc, target, options)` entry point.
 *
 * Single-call shape for the common case:
 *
 *   1. **Validate** the document via `validateStyleDocument` — if it
 *      throws a `StyleError`, capture the errors[] array into the
 *      result and skip the rest (never throw to the caller).
 *   2. **Compose theme** if `options.theme` is supplied — either by
 *      name (looked up via the optional `themeRegistry`) or by direct
 *      `Theme` value.  Theme-by-name with an unknown name surfaces
 *      as a warning, not an error — the rest of the rule still
 *      translates (no theme just means "use base tokens").
 *   3. **Dispatch** to the requested translator and return its
 *      `output` / `emittedRules` / `warnings`.
 *
 * Two convenience type guards (`isCompileError` / `isCompileSuccess`)
 * let consumers branch without type assertions.
 *
 * The orchestrator is **pure**: same `(doc, target, options)`
 * triple → byte-identical output (FM03 reproducibility).  It never
 * throws on shape or unresolved refs; the translator-level warn-and-
 * skip discipline propagates through.
 *
 * Two genuine failure modes that still throw:
 *
 *   - **Unknown target.**  The translator dispatch table is closed;
 *     a target the orchestrator doesn't know is a programming bug,
 *     not a runtime data condition.  Throws `TypeError`.
 *   - **Theme registry says theme name "X" but `themeRegistry` is
 *     not supplied.**  Caller asked for name lookup without giving us
 *     anywhere to look — surface as a thrown `TypeError`.
 *
 * Everything else (validator failure, unresolved tokens, ext: kinds
 * without translators) is captured into the result.
 *
 * @module compile
 */

import {
  validateStyleDocument,
  StyleError, canonicalStyleDocument,
  type StyleErrorEntry, type StyleRuleId, type StyleWarning, type Theme,
} from "@coding-adventures/forme-style-ir";
import {
  composeWithTheme, type ThemeRegistry,
} from "@coding-adventures/forme-style-theme";
import { translateToCss } from "@coding-adventures/forme-style-to-css";
import { translateToLatex } from "@coding-adventures/forme-style-to-latex";
import { translateToTerminal } from "@coding-adventures/forme-style-to-terminal";

// ─── Public types ────────────────────────────────────────────────────────

/** The set of backend targets the orchestrator can dispatch to. */
export type CompileTarget = "css" | "latex" | "terminal";

/** Options accepted by `compile`. */
export interface CompileOptions {
  /** Same shape as every translator's `activeContexts`. */
  readonly activeContexts: readonly string[];
  /** Per-page CSS slicing (FM06 input). */
  readonly usedRuleIds?: readonly StyleRuleId[];
  /** Per-page selector scoping.  Caller-trusted; see translator JSDocs. */
  readonly scope?: string;
  /**
   * Optional theme to compose onto the base document before
   * translation.  Either a `Theme` value directly or a string name
   * to look up via `themeRegistry`.
   */
  readonly theme?: string | Theme;
  /**
   * Required when `theme` is a string — the registry to resolve
   * theme names against.  Throws if `theme` is a string but
   * `themeRegistry` is absent (programmer error).
   */
  readonly themeRegistry?: ThemeRegistry;
}

/**
 * Result type — discriminated union.  `errors` populated when the
 * validator rejected the input; in that case `output` is the empty
 * string, `emittedRules` is empty, `warnings` is empty.
 */
export interface CompileResult {
  readonly target: CompileTarget;
  readonly output: string;
  readonly emittedRules: readonly StyleRuleId[];
  readonly warnings: readonly StyleWarning[];
  /**
   * Validator-failure entries.  Empty array on success; non-empty
   * on validator failure (in which case `output` is empty).
   */
  readonly errors: readonly StyleErrorEntry[];
}

// ─── Public entry point ──────────────────────────────────────────────────

/**
 * Validate, optionally compose a theme, then dispatch to the chosen
 * backend translator.  Returns a unified result; never throws on
 * shape.
 */
export function compile(
  doc: unknown,
  target: CompileTarget,
  options: CompileOptions,
): CompileResult {
  // ─── 1. Validate ─────────────────────────────────────────────────────
  let validated: ReturnType<typeof validateStyleDocument>;
  try {
    validated = validateStyleDocument(doc);
  } catch (e) {
    if (e instanceof StyleError) {
      return {
        target,
        output: "",
        emittedRules: Object.freeze([]),
        warnings: Object.freeze([]),
        errors: Object.freeze([...e.errors]),
      };
    }
    // Anything else is a programmer bug (something downstream of the
    // validator threw unexpectedly).  Re-throw so the caller sees it.
    throw e;
  }

  // The validator returns `{ document, warnings }`.  Carry forward
  // any validator warnings into the final result.
  const validatorWarnings = [...validated.warnings];

  // ─── 2. Compose theme (if requested) ─────────────────────────────────
  let doc1 = validated.document;
  if (options.theme !== undefined) {
    if (typeof options.theme === "string") {
      if (options.themeRegistry === undefined) {
        throw new TypeError(
          `compile: theme name ${JSON.stringify(options.theme)} requested but no themeRegistry supplied`,
        );
      }
      const t = options.themeRegistry.lookup(options.theme);
      if (t === undefined) {
        // Unknown theme name is a *warning*, not an error — the
        // document still translates with base tokens.  Caller can
        // tell the difference via the warning's code.
        validatorWarnings.push({
          code: "THEME_NOT_FOUND",
          message: `theme ${JSON.stringify(options.theme)} not found in registry; proceeding with base tokens`,
        });
      } else {
        doc1 = composeWithTheme(doc1, t);
      }
    } else {
      doc1 = composeWithTheme(doc1, options.theme);
    }
  }

  // ─── 3. Dispatch to translator ───────────────────────────────────────
  // Pass through the same activeContexts / usedRuleIds / scope to
  // whichever translator gets called.  The TranslateOptions shapes
  // are identical across the three backends (FM04 §9.1 — each
  // translator redeclares locally but the structure is the same).
  const translatorOptions = {
    activeContexts: options.activeContexts,
    usedRuleIds: options.usedRuleIds,
    scope: options.scope,
  };

  let translatorResult;
  switch (target) {
    case "css":
      translatorResult = translateToCss(doc1, translatorOptions);
      break;
    case "latex":
      translatorResult = translateToLatex(doc1, translatorOptions);
      break;
    case "terminal":
      translatorResult = translateToTerminal(doc1, translatorOptions);
      break;
    default: {
      // The CompileTarget union is closed; reaching here means a
      // caller bypassed the type system.  Throw.
      const t: string = String(target);
      throw new TypeError(`compile: unknown target ${JSON.stringify(t)}`);
    }
  }

  return {
    target,
    output: translatorResult.output,
    emittedRules: translatorResult.emittedRules,
    warnings: Object.freeze([...validatorWarnings, ...translatorResult.warnings]),
    errors: Object.freeze([]),
  };
}

// ─── Convenience type guards ─────────────────────────────────────────────

/**
 * True iff `result.errors` is non-empty — i.e. the validator
 * rejected the input and no translation happened.
 */
export function isCompileError(result: CompileResult): boolean {
  return result.errors.length > 0;
}

/** Inverse of `isCompileError`. */
export function isCompileSuccess(result: CompileResult): boolean {
  return result.errors.length === 0;
}

// ─── Reproducibility helper ──────────────────────────────────────────────

/**
 * Compute a canonical-JSON fingerprint of the *validated* document.
 * Useful for downstream cache keys.  Wraps `canonicalStyleDocument`
 * around a fresh `compile` validation pass so callers don't have to
 * import the IR package directly.
 *
 * Returns the canonical-JSON string on success, null on validator
 * failure.
 *
 * Mirrors `compile`'s policy: only `StyleError` and `RangeError`
 * (the two documented failure modes of validator + serializer) are
 * captured; any other exception is a programmer bug and re-raised.
 * `canonicalStyleDocument` throws `RangeError` on non-finite numbers
 * or cycle-guard hits — both signal a malformed-but-not-validator-
 * caught document; returning null is the right caller-facing
 * behaviour for a fingerprint helper.
 */
export function fingerprintDocument(doc: unknown): string | null {
  try {
    const { document } = validateStyleDocument(doc);
    return canonicalStyleDocument(document);
  } catch (e) {
    if (e instanceof StyleError || e instanceof RangeError) return null;
    throw e;
  }
}
