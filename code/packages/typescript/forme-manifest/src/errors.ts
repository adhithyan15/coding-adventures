/**
 * ManifestError — the typed error every public function in this
 * package throws on bad input.
 *
 * Carries:
 *   - `code`     — machine-readable; one of `MANIFEST_ERROR_CODES`
 *   - `message`  — human-readable
 *   - `path`     — JSON-path-like field locator ("plugin.name",
 *                  "capabilities.required[2].realm", "" for root)
 *   - `errors[]` — when the validator collects multiple violations
 *                  in one pass, every individual finding lands here
 *                  with its own code/message/path, and the top-level
 *                  fields summarise.
 *
 * The validator (validate.ts) constructs an aggregate ManifestError
 * carrying every violation it finds; the parser (parse-toml.ts) throws
 * on the first syntax error since by then there's no useful structure
 * to keep walking.  Match the FM03 §2.4 `ConfigError` precedent: one
 * pass surfaces every problem.
 */

/**
 * Frozen list of every error code this package emits.
 *
 * Parser codes (TOML_*) signal a syntactic problem — the input isn't
 * valid TOML or uses a feature we don't accept (multi-line strings,
 * floats, datetimes, inline tables).
 *
 * Validator codes (everything else) signal a semantic problem — the
 * input parses but violates FM02 §3.3.
 */
export const MANIFEST_ERROR_CODES = Object.freeze([
  // ─── Parser ───────────────────────────────────────────────────────
  "TOML_MALFORMED",                  // catch-all syntax error
  "TOML_UNTERMINATED_STRING",
  "TOML_INVALID_ESCAPE",
  "TOML_UNSUPPORTED_FEATURE",        // multi-line, float, datetime, inline table
  "TOML_DUPLICATE_KEY",
  "TOML_INVALID_INTEGER",
  "TOML_INVALID_ARRAY",
  // ─── Validator (FM02 §3.3) ────────────────────────────────────────
  "MANIFEST_VERSION_UNSUPPORTED",
  "PLUGIN_NAME_INVALID",
  "PLUGIN_VERSION_INVALID",
  "PLUGIN_API_VERSION_INVALID",
  "RUNTIME_KIND_INVALID",
  "RUNTIME_ENTRY_MISSING",
  "RUNTIME_PLATFORMS_MISSING",
  "CAPABILITY_MALFORMED",
  "CAPABILITY_FIRST_PARTY_ONLY",     // FM02 §3.3 rule 8
  "STAGE_ID_INVALID",
  "STAGE_ID_DUPLICATE",
  "STAGE_KIND_NAME_INVALID",
  "KIND_NAME_INVALID",                // ext: prefix required
  "RESOURCE_VALUE_INVALID",
  "SIGNATURE_ALGORITHM_INVALID",
  "SIGNATURE_FIELD_MISSING",
  "SIGNATURE_INVALID",
  "REQUIRED_FIELD_MISSING",
  "FIELD_TYPE_MISMATCH",
  // ─── Templating ───────────────────────────────────────────────────
  "TEMPLATE_UNKNOWN_VARIABLE",
  "TEMPLATE_MALFORMED",
] as const);

/** String-literal union of every recognised code. */
export type ManifestErrorCode = (typeof MANIFEST_ERROR_CODES)[number];

/** One individual finding (one violation, one field). */
export interface ManifestErrorEntry {
  readonly code: ManifestErrorCode;
  /**
   * Dotted field path to the offending field, JSON-path style.
   * Examples: `"plugin.name"`, `"capabilities.required[2].realm"`,
   * `""` (root).
   */
  readonly path: string;
  readonly message: string;
}

export interface ManifestErrorInit {
  readonly code: ManifestErrorCode;
  readonly message: string;
  readonly path?: string;
  readonly errors?: readonly ManifestErrorEntry[];
}

/**
 * Typed error raised by `parseManifest`, `validateManifest`, and the
 * templating + signature helpers.  Aggregates multi-finding outputs
 * via the `errors[]` field; single-finding cases just have the
 * top-level `code`/`message`.
 */
export class ManifestError extends Error {
  readonly code: ManifestErrorCode;
  readonly path: string;
  readonly errors: readonly ManifestErrorEntry[];

  constructor(init: ManifestErrorInit) {
    const path = init.path ?? "";
    const entries = init.errors ?? [];
    super(buildMessage(init.message, path, entries));
    this.name = "ManifestError";
    this.code = init.code;
    this.path = path;
    this.errors = Object.freeze(entries.map((e) => Object.freeze({ ...e })));
  }
}

/**
 * Build the top-level Error.message.  When there's one entry the
 * message is straightforward; when there are many we summarise the
 * count and list the first few so the test failure / stderr line
 * makes the size of the problem obvious without flooding output.
 */
function buildMessage(
  summary: string,
  path: string,
  errors: readonly ManifestErrorEntry[],
): string {
  if (errors.length === 0) {
    return path ? `${summary} (at ${path})` : summary;
  }
  const head = errors.slice(0, 5)
    .map((e) => `  - [${e.code}] ${e.path || "(root)"}: ${e.message}`)
    .join("\n");
  const more = errors.length > 5 ? `\n  ... (${errors.length - 5} more)` : "";
  return `${summary} (${errors.length} violation${errors.length === 1 ? "" : "s"}):\n${head}${more}`;
}
