/**
 * Canonical error codes — the kernel-blessed vocabulary that stages
 * SHOULD use whenever their failure semantically matches one of these.
 *
 * See FM01 §6.1 for the full design.  The table is intentionally small:
 * every additional code is one more thing pipeline drivers (CLI, dev
 * server, editor) must know how to render and document.  Stages that
 * truly need a custom error code declare it under a package-prefixed
 * name (e.g. `parse-mdx/MY_SPECIAL_CASE`) — that namespacing prevents
 * collisions across the ecosystem.
 *
 * Realm prefixes — `TRANSFORM_*`, `COLLECT_*`, `RENDER_*`, `EMIT_*` —
 * are open: stages in those realms invent their own specific codes
 * (`TRANSFORM_SYNTAX_HIGHLIGHT_NO_LANG`, `COLLECT_DATE_PARSE_FAILED`)
 * and the kernel's role is just to reserve the prefix.
 */

/**
 * The closed vocabulary of kernel-blessed error codes.
 *
 * Frozen so test code can't accidentally mutate it between runs (the
 * same lesson we learned in `forme-types/src/kinds.ts`).
 */
export const ERROR_CODES = Object.freeze({
  // ─── Parse realm ────────────────────────────────────────────────────
  /** Generic parse failure when no more specific code applies. */
  PARSE_ERROR:               "PARSE_ERROR",
  /** Frontmatter could not be parsed (e.g. invalid YAML, unclosed delimiter). */
  PARSE_FRONTMATTER_INVALID: "PARSE_FRONTMATTER_INVALID",
  /** Source produced no document at all (empty file, header-only). */
  PARSE_NO_DOCUMENT:         "PARSE_NO_DOCUMENT",

  // ─── Capability realm ──────────────────────────────────────────────
  /**
   * Stage attempted an operation it has no capability for.  This is
   * always non-recoverable — best-effort mode does not soften it.
   */
  CAPABILITY_DENIED:         "CAPABILITY_DENIED",

  // ─── Lifecycle realm ───────────────────────────────────────────────
  /** Stage was cancelled before completing. */
  CANCELLED:                 "CANCELLED",
  /**
   * Stage threw an exception that wasn't a `StageError`.  The
   * orchestrator wraps it with `cause` set to the original throw.
   */
  UNCAUGHT:                  "UNCAUGHT",
  /** Stage exceeded its deadline. */
  TIMEOUT:                   "TIMEOUT",

  // ─── I/O realm ─────────────────────────────────────────────────────
  /** Storage path does not exist. */
  IO_NOT_FOUND:              "IO_NOT_FOUND",
  /** Storage path exists but the operation was refused. */
  IO_PERMISSION_DENIED:      "IO_PERMISSION_DENIED",

  // ─── Network realm ─────────────────────────────────────────────────
  /** Network request failed before reaching the host. */
  NETWORK_UNREACHABLE:       "NETWORK_UNREACHABLE",
} as const);

/**
 * The string-literal union of every kernel-blessed code.  Stages may
 * use any string code — this type exists for the codes the kernel
 * itself produces.
 */
export type KernelErrorCode = (typeof ERROR_CODES)[keyof typeof ERROR_CODES];
