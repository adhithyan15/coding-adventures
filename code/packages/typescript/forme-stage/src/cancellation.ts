/**
 * Cancellation — cooperative shutdown signal threaded through every
 * stage invocation.
 *
 * Per FM01 §4.4: stages SHOULD honour cancellation at safe points
 * inside long-running work by calling `ctx.cancellation.throwIfCancelled()`.
 * They MUST NOT catch and swallow `CancellationError`.  The orchestrator
 * cancels a pipeline when:
 *
 *   - The user hits Ctrl-C in the CLI.
 *   - A peer stage errors with non-recoverable failure (fail-fast).
 *   - The overall build deadline expires.
 *
 * === Token vs. source ===
 *
 * `CancellationToken` is the *read-only* view a stage receives.  The
 * matching `CancellationTokenSource` (held by the orchestrator) is the
 * write-side: calling `source.cancel(reason)` flips the token's
 * `cancelled` flag, populates `reason`, fires `onCancel` callbacks,
 * and aborts the underlying `AbortSignal`.
 *
 * Splitting read- and write-sides this way makes the contract clean:
 * the stage cannot accidentally cancel itself, and the orchestrator
 * cannot leak the cancel handle into stage code.
 *
 * === AbortSignal interop ===
 *
 * Every CancellationToken exposes `signal: AbortSignal` so stages can
 * pass it directly into `fetch`, `setTimeout(..., { signal })`, or any
 * other web-platform API that already speaks AbortSignal.  When the
 * orchestrator cancels, the abort propagates without any extra wiring
 * from the stage.
 */

import { CancellationError } from "@coding-adventures/forme-errors";

/** Cancellation observation interface — the read-only side of a token. */
export interface CancellationToken {
  readonly cancelled: boolean;
  readonly reason: string | null;
  /** Throws `CancellationError` if cancellation has been requested. */
  throwIfCancelled(): void;
  /** Register a cleanup callback fired when cancellation occurs. */
  onCancel(callback: () => void): void;
  /** Standard web-platform abort signal for fetch/timer interop. */
  readonly signal: AbortSignal;
}

/** Write-side handle that controls a CancellationToken. */
export interface CancellationTokenSource {
  readonly token: CancellationToken;
  /**
   * Trip the token.  Subsequent reads see `cancelled === true` and
   * `reason === <given>` (or null if not provided).  Idempotent — a
   * second cancel call is a silent no-op.
   */
  cancel(reason?: string): void;
}

class TokenImpl implements CancellationToken {
  private _cancelled = false;
  private _reason: string | null = null;
  private readonly callbacks: Array<() => void> = [];
  private readonly controller = new AbortController();

  get cancelled(): boolean { return this._cancelled; }
  get reason(): string | null { return this._reason; }
  get signal(): AbortSignal { return this.controller.signal; }

  throwIfCancelled(): void {
    if (this._cancelled) {
      throw new CancellationError(this._reason ?? undefined);
    }
  }

  onCancel(callback: () => void): void {
    if (this._cancelled) {
      // Fire immediately if already cancelled.  Catch errors so a bad
      // callback doesn't crash callers of `onCancel`.
      try { callback(); } catch { /* swallow */ }
      return;
    }
    this.callbacks.push(callback);
  }

  /** Internal — only the source calls this. */
  fire(reason: string | undefined): void {
    if (this._cancelled) return; // idempotent
    this._cancelled = true;
    this._reason = reason ?? null;
    this.controller.abort();
    for (const cb of this.callbacks) {
      try { cb(); } catch { /* swallow per onCancel contract */ }
    }
    this.callbacks.length = 0;
  }
}

/**
 * Build a fresh source.  The returned `token` is what stages consume;
 * the returned `cancel` (on the source) is what the orchestrator holds.
 */
export function createCancellationTokenSource(): CancellationTokenSource {
  const token = new TokenImpl();
  return {
    token,
    cancel(reason) { token.fire(reason); },
  };
}

/**
 * A token that is permanently un-cancelled.  Useful for tests and for
 * synchronous stages that can't observe cancellation anyway.
 */
export function neverCancelledToken(): CancellationToken {
  return NEVER;
}

const NEVER_CONTROLLER = new AbortController();
const NEVER: CancellationToken = {
  cancelled: false,
  reason: null,
  throwIfCancelled() { /* no-op */ },
  onCancel() { /* no-op — will never fire */ },
  signal: NEVER_CONTROLLER.signal,
};
