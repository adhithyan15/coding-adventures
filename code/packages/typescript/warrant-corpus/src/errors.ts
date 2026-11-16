/**
 * # Error types and the closed set of failure ids
 *
 * Two different things can go wrong in a lane run, and they must never be
 * confused:
 *
 * - **The run cannot be trusted to start.** The corpus is malformed, the
 *   binding names an export that does not exist, the command line is wrong.
 *   These throw a {@link LaneError} (one subclass per cause) and the command
 *   exits with status `2`. No verdict is produced, because there is nothing to
 *   give a verdict *about*.
 * - **The library under test is wrong.** A case disagrees with the model. That
 *   is a *verdict* (`not ok <id> <failure-id>`, exit status `1`). It is data,
 *   not an exception, because a negative control *wants* it.
 *
 * The failure ids are a closed set (VER04i section 6.2), so a manifest checker
 * can reject an id it has never heard of instead of guessing.
 */

export const COUNT_MISMATCH = "count-mismatch";
export const STREAM_MISMATCH = "stream-mismatch";
export const CALL_COUNT_MISMATCH = "call-count-mismatch";
export const RESULT_MISMATCH = "result-mismatch";
export const TYPE_MISMATCH = "type-mismatch";
export const MISSING_ERROR = "missing-error";
export const UNEXPECTED_ERROR = "unexpected-error";
export const UNCAUGHT_FAULT = "uncaught-fault";

export const FAILURE_IDS: ReadonlySet<string> = new Set([
  COUNT_MISMATCH,
  STREAM_MISMATCH,
  CALL_COUNT_MISMATCH,
  RESULT_MISMATCH,
  TYPE_MISMATCH,
  MISSING_ERROR,
  UNEXPECTED_ERROR,
  UNCAUGHT_FAULT,
]);

/**
 * The run cannot proceed; the command exits with status 2.
 *
 * `code` is a short stable slug (`"malformed-json"`, `"digest"` ...) so tests
 * and callers can match on the cause without parsing prose. The message never
 * echoes untrusted input wholesale: it names the *kind* of problem.
 */
export class LaneError extends Error {
  readonly code: string;

  constructor(code: string, message: string) {
    super(`${code}: ${message}`);
    this.code = code;
  }
}

/** The corpus, its TFV file or its model IR is malformed or inconsistent. */
export class CorpusError extends LaneError {}

/** The binding file is malformed, incomplete or does not match the code. */
export class BindingError extends LaneError {}

/** The command line or environment asks for something refused. */
export class UsageError extends LaneError {}

/**
 * Throw a {@link CorpusError} unless `condition` holds.
 *
 * Corpus validation is a long list of "this must be true"; one helper keeps
 * each of them to a single readable line.
 */
export function need(condition: boolean, code: string, message: string): asserts condition {
  if (!condition) {
    throw new CorpusError(code, message);
  }
}
