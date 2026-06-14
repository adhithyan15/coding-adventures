/**
 * `ConfigError` — typed validation errors raised by `validateConfig`.
 *
 * Per FM03 §2.4, every error carries:
 *
 *   - The failing field path (`"stages[2].id"`, `"settings.maxConcurrency"`)
 *   - The rule that was violated (machine-readable code)
 *   - A human-readable remediation message
 *
 * `ConfigError` is *not* a `StageError` — it surfaces before any stage
 * runs, during the orchestrator's resolve/typecheck phase, and the
 * orchestrator's per-stage error boundary doesn't apply here.  The CLI
 * (and dev-server, and editor preview) format these with their own UI.
 *
 * The validator collects ALL violations across the config rather than
 * stopping at the first.  This is a usability lever — users want to
 * see every problem in one pass, not chase them one at a time.  The
 * thrown `ConfigError` carries an `errors[]` array of all violations;
 * its top-level `code`/`message` summarise the count.
 */

export interface ConfigErrorEntry {
  /** JSON-pointer-ish path to the offending field. */
  readonly path: string;
  /** Machine-readable rule code (e.g. "DUPLICATE_INSTANCE_ID"). */
  readonly code: string;
  /** Human-readable explanation + remediation. */
  readonly message: string;
}

/**
 * Aggregate validation failure.  Subclass `Error` so existing
 * `instanceof Error` boundaries (Vitest, IDE error reporters) handle
 * it correctly.
 */
export class ConfigError extends Error {
  readonly errors: readonly ConfigErrorEntry[];

  constructor(errors: readonly ConfigErrorEntry[]) {
    if (errors.length === 0) {
      throw new Error("ConfigError must be constructed with at least one entry");
    }
    super(buildSummary(errors));
    this.name = "ConfigError";
    this.errors = Object.freeze(errors.map(e => Object.freeze({ ...e })));
  }
}

function buildSummary(errors: readonly ConfigErrorEntry[]): string {
  if (errors.length === 1) {
    const e = errors[0]!;
    return `Pipeline config invalid: ${e.path}: ${e.message} [${e.code}]`;
  }
  const lines = errors.map(e => `  - ${e.path}: ${e.message} [${e.code}]`);
  return `Pipeline config invalid (${errors.length} errors):\n${lines.join("\n")}`;
}

/**
 * Canonical rule codes the validator emits.  Matches the spec rules
 * in FM03 §2.4.  Stages and the orchestrator MAY introduce additional
 * codes via the `<package>/<code>` convention; the kernel reserves
 * the unprefixed names below.
 */
export const CONFIG_ERROR_CODES = Object.freeze({
  /** Two stage instances share the same `id` (or default-derived id). */
  DUPLICATE_INSTANCE_ID:     "DUPLICATE_INSTANCE_ID",
  /** A stage value is missing required fields (name, version, …). */
  INVALID_STAGE_VALUE:       "INVALID_STAGE_VALUE",
  /** Stage targets an `apiVersion` the kernel doesn't support. */
  API_VERSION_MISMATCH:      "API_VERSION_MISMATCH",
  /**
   * A `StageRef` slipped through without a plugin host loaded.
   * v0's default-direct-import host refuses these.
   */
  STAGE_REF_UNRESOLVED:      "STAGE_REF_UNRESOLVED",
  /** An instance requests a capability the stage's manifest doesn't declare. */
  CAPABILITY_NOT_DECLARED:   "CAPABILITY_NOT_DECLARED",
  /** A stage demands a config but none was provided. */
  CONFIG_REQUIRED:           "CONFIG_REQUIRED",
  /**
   * The provided config violates the stage's declared `configSchema`.
   * Field-level details (which property, what constraint) are in
   * the entry's `message`.  See `json-schema.ts` for the subset
   * of draft-07 keywords supported.
   */
  CONFIG_SCHEMA_VIOLATION:   "CONFIG_SCHEMA_VIOLATION",
  /**
   * Multiple terminal stages (no consumer) exist without a matching
   * `OutputSpec` for each (FM03 §3.3 step 5).
   */
  MULTIPLE_OUTPUTS_UNNAMED:  "MULTIPLE_OUTPUTS_UNNAMED",
  /** An `EdgeSpec` or `OutputSpec` references an instance ID that doesn't exist. */
  UNKNOWN_INSTANCE_ID:       "UNKNOWN_INSTANCE_ID",
  /** Top-level field is missing or has the wrong type. */
  MALFORMED:                 "MALFORMED",
} as const);
export type ConfigErrorCode = (typeof CONFIG_ERROR_CODES)[keyof typeof CONFIG_ERROR_CODES];
