/**
 * contexts.ts — context constants and helpers (FM04 §6).
 *
 * Contexts are named gating conditions on style rules.  At translate
 * time the consumer activates some subset; rules whose `context` is
 * in the active set (or has no context) apply.
 *
 * The kernel blesses a fixed set of standard contexts; anything else
 * follows the `ext:<plugin>:<name>` convention.  The fixed set
 * covers the common multi-backend cases (print vs screen, dark vs
 * light, narrow vs wide viewport, reduced motion / high contrast
 * accessibility) without needing a registry.
 *
 * Per FM04 §6.2 a single rule has at most one context — to express
 * "print AND high-contrast" the producer declares two rules with the
 * same selector + properties, one per context.  This is intentional;
 * compound contexts open up AND/OR ambiguity questions we sidestep.
 *
 * @module contexts
 */

// ─── Standard contexts ────────────────────────────────────────────────────

/** Print backends (LaTeX, PDF, paper). */
export const CONTEXT_PRINT          = "print";
/** Screen backends (web, terminal). */
export const CONTEXT_SCREEN         = "screen";
/** Dark colour scheme. */
export const CONTEXT_DARK           = "dark";
/** Narrow viewport (web). */
export const CONTEXT_NARROW         = "narrow";
/** Wide viewport (web). */
export const CONTEXT_WIDE           = "wide";
/** Reduced-motion accessibility preference. */
export const CONTEXT_REDUCED_MOTION = "reduced-motion";
/** High-contrast accessibility preference. */
export const CONTEXT_HIGH_CONTRAST  = "high-contrast";

/**
 * Frozen tuple of the kernel-blessed contexts.  The validator uses
 * this to flag (warn, not reject) rules referencing contexts that
 * aren't in this set AND don't carry an `ext:` prefix — most likely
 * typos.
 */
export const STANDARD_CONTEXTS = Object.freeze([
  CONTEXT_PRINT,
  CONTEXT_SCREEN,
  CONTEXT_DARK,
  CONTEXT_NARROW,
  CONTEXT_WIDE,
  CONTEXT_REDUCED_MOTION,
  CONTEXT_HIGH_CONTRAST,
] as const);

export type StandardContext = (typeof STANDARD_CONTEXTS)[number];

/** Detect an `ext:` namespaced context without instantiating regex. */
export function isExtensionContext(name: string): name is `ext:${string}` {
  return name.startsWith("ext:") && name.length > 4;
}

/**
 * Test whether a context name is *kernel-recognised*.  Returns true
 * for any standard context or any `ext:*` name.  False for an
 * unrecognised bare string — the validator warns on these to catch
 * typos.
 */
export function isRecognisedContext(name: string): boolean {
  return (
    (STANDARD_CONTEXTS as readonly string[]).includes(name)
    || isExtensionContext(name)
  );
}
