/**
 * style-error.ts — error/warning shapes for the Style IR (FM04 §9).
 *
 * Two failure modes:
 *
 * - **`StyleError`** is thrown by the *validator* when the IR shape
 *   itself is invalid (missing fields, duplicate ids, malformed
 *   selectors, unknown property kinds outside the `ext:` namespace).
 *   It carries a structured `errors[]` so callers see *every*
 *   violation in one pass — same pattern as
 *   `forme-pipeline-config`'s `ConfigError`.
 *
 * - **`StyleWarning`** is *returned* by translators (not thrown)
 *   when the IR is valid but the backend can't realise something
 *   (unknown property kind in `ext:*`, unresolved `TokenRef`, etc.).
 *   Warnings preserve the build's forward-compat guarantee per FM04
 *   §9.6 — a future property kind shouldn't fail today's builds.
 *
 * @module style-error
 */

// ─── Error codes ─────────────────────────────────────────────────────────

/**
 * Frozen vocabulary of validator-emitted error codes.  Callers can
 * switch exhaustively on these to localise messages or to filter.
 */
export const STYLE_ERROR_CODES = Object.freeze([
  "MALFORMED",
  "DUPLICATE_RULE_ID",
  "EMPTY_RULE_ID",
  "UNKNOWN_PROPERTY_KIND",
  "UNKNOWN_SELECTOR_KIND",
  "INVALID_HEADING_LEVEL",
  "INVALID_TOKEN_REF_PATH",
  "INVALID_LENGTH_UNIT",
  "INVALID_COLOR",
  "INVALID_COLOR_CHANNEL",
  "INVALID_PROPERTY_VALUE",
  "EMPTY_COMPOSITION",
  "UNKNOWN_CONTEXT",
  "INVALID_EXTENSION_KEY",
] as const);

export type StyleErrorCode = (typeof STYLE_ERROR_CODES)[number];

// ─── StyleError ──────────────────────────────────────────────────────────

/**
 * A single validator complaint.  `path` is a slash-delimited locator
 * into the document tree (e.g. `rules/3/properties/0/value/r`).
 * Entries are frozen so callers can't mutate them.
 */
export interface StyleErrorEntry {
  readonly code: StyleErrorCode;
  readonly path: string;
  readonly message: string;
}

/**
 * Thrown when `validateStyleDocument` finds one or more violations.
 * The `errors` array carries every violation; the `message` is a
 * multi-line summary.  Same shape as `forme-pipeline-config`'s
 * `ConfigError`.
 */
export class StyleError extends Error {
  override readonly name = "StyleError";
  readonly errors: readonly StyleErrorEntry[];

  constructor(entries: readonly StyleErrorEntry[]) {
    const frozen = entries.map((e) => Object.freeze({ ...e }));
    super(formatMessage(frozen));
    this.errors = Object.freeze(frozen);
  }
}

function formatMessage(entries: readonly StyleErrorEntry[]): string {
  if (entries.length === 0) return "StyleError: (no entries)";
  if (entries.length === 1) {
    const e = entries[0]!;
    return `StyleError: ${e.code} at ${e.path}: ${e.message}`;
  }
  const lines = entries.map((e) => `  - [${e.code}] ${e.path}: ${e.message}`);
  return `StyleError: ${entries.length} violations:\n${lines.join("\n")}`;
}

// ─── StyleWarning ────────────────────────────────────────────────────────

/**
 * A translator-emitted warning.  Carried in `TranslateResult.warnings`.
 * Translators MAY emit warnings rather than throwing per FM04 §9.6 —
 * keeps builds forward-compatible.
 */
export interface StyleWarning {
  readonly code: string;
  readonly message: string;
  /** Rule id the warning pertains to (if applicable). */
  readonly ruleId?: string;
  /** Property kind the warning pertains to (if applicable). */
  readonly propertyKind?: string;
}
