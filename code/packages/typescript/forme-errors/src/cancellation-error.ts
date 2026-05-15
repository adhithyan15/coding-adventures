/**
 * CancellationError — thrown when a stage observes that its
 * `ctx.cancellation` token has fired.
 *
 * Per FM01 §6.3, `CancellationError` is **not** a `StageError`.  This
 * is a deliberate split: cancellation is an orchestrator-level event,
 * not a stage-level failure.  The orchestrator's error boundary
 * should propagate `CancellationError` straight up to unwind the
 * pipeline rather than wrapping it as `UNCAUGHT` and triggering
 * fallback / retry logic.
 *
 * The `reason` field carries an optional short string the originator
 * supplied.  It surfaces in logs and the final RunResult so a user
 * who cancelled with Ctrl-C sees "Cancelled by user" rather than a
 * bare stack trace.
 *
 * Stages SHOULD honour cancellation at safe points by calling
 * `ctx.cancellation.throwIfCancelled()`, which throws this error.
 * Stages MUST NOT catch and swallow it.
 */

export class CancellationError extends Error {
  /**
   * Optional human-readable reason ("user pressed Ctrl-C", "deadline
   * exceeded", "downstream stage failed in fail-fast mode").  Null when
   * the originator did not supply one.
   */
  readonly reason: string | null;

  constructor(reason?: string) {
    super(reason ?? "Cancelled");
    this.name   = "CancellationError";
    this.reason = reason ?? null;
  }
}

/**
 * Predicate: is this thrown value a `CancellationError`?  Useful in
 * orchestrator error boundaries that need to differentiate cancellation
 * from other throws without doing `instanceof` plumbing across module
 * boundaries (which can fail if two copies of this package end up
 * loaded in the same process).
 */
export function isCancellationError(value: unknown): value is CancellationError {
  return value instanceof CancellationError
    || (typeof value === "object"
        && value !== null
        && (value as { name?: unknown }).name === "CancellationError");
}
