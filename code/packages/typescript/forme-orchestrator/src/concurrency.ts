/** Shared FIFO concurrency control for stage and per-item scheduler work. */

import { CancellationError } from "@coding-adventures/forme-errors";
import type { CancellationToken } from "@coding-adventures/forme-stage";

export interface ConcurrencyPoolStats {
  readonly active: number;
  readonly queued: number;
  readonly peakActive: number;
}

export interface ConcurrencyPermit {
  /**
   * Release this task's permit while awaiting an upstream dependency, then
   * reacquire at the FIFO tail before returning the dependency's value.
   */
  yieldWhile<T>(wait: () => Promise<T>): Promise<T>;
}

export interface ConcurrencyPool {
  run<T>(task: (permit: ConcurrencyPermit) => Promise<T> | T): Promise<T>;
  stats(): ConcurrencyPoolStats;
}

interface Waiter {
  readonly resolve: () => void;
  readonly reject: (error: unknown) => void;
}

/**
 * Create one cancellation-aware permit budget shared by a pipeline run.
 *
 * New tasks and yielded-task reacquisitions join the same FIFO queue. Active
 * work is cooperatively cancelled by the caller's token; queued work is
 * rejected immediately when that token fires.
 */
export function createConcurrencyPool(
  limit: number,
  cancellation: CancellationToken,
): ConcurrencyPool {
  if (!Number.isSafeInteger(limit) || limit < 1) {
    throw new Error("concurrency limit must be a positive safe integer");
  }

  const queue: Waiter[] = [];
  let active = 0;
  let peakActive = 0;
  let closedError: CancellationError | null = null;
  let resolveCancellation!: (error: CancellationError) => void;
  const cancellationSignal = new Promise<CancellationError>(resolve => {
    resolveCancellation = resolve;
  });

  const dispatch = (): void => {
    if (closedError !== null) return;
    while (active < limit && queue.length > 0) {
      const waiter = queue.shift()!;
      active += 1;
      peakActive = Math.max(peakActive, active);
      waiter.resolve();
    }
  };

  const acquire = async (): Promise<void> => {
    if (closedError !== null) throw closedError;
    if (active < limit && queue.length === 0) {
      active += 1;
      peakActive = Math.max(peakActive, active);
      return;
    }
    return new Promise<void>((resolve, reject) => {
      queue.push({ resolve, reject });
    });
  };

  const release = (): void => {
    if (active < 1) {
      throw new Error("concurrency pool internal error: release without an active permit");
    }
    active -= 1;
    dispatch();
  };

  const close = (): void => {
    if (closedError !== null) return;
    closedError = cancellationError(cancellation);
    resolveCancellation(closedError);
    const waiting = queue.splice(0, queue.length);
    for (const waiter of waiting) waiter.reject(closedError);
  };

  cancellation.onCancel(close);

  const run = async <T>(
    task: (permit: ConcurrencyPermit) => Promise<T> | T,
  ): Promise<T> => {
    await acquire();
    let held = true;
    let yielding = false;
    let inFlightYield: Promise<unknown> | null = null;
    let yieldFailed = false;
    let yieldError: unknown;
    let poisonedError: unknown = null;
    const reacquire = async (): Promise<void> => {
      try {
        await acquire();
        held = true;
      } catch (error) {
        poisonedError = error;
        throw error;
      }
    };
    const permit: ConcurrencyPermit = {
      yieldWhile<R>(wait: () => Promise<R>): Promise<R> {
        if (yielding) {
          return Promise.reject(new Error("concurrency permit allows only one yield in flight"));
        }
        if (!held) {
          return Promise.reject(new Error("yieldWhile requires an active permit"));
        }
        yielding = true;
        held = false;
        release();
        let operation!: Promise<R>;
        operation = (async (): Promise<R> => {
          try {
            const outcome = await Promise.race([
              Promise.resolve().then(wait).then(
                value => ({ kind: "value" as const, value }),
                error => ({ kind: "error" as const, error }),
              ),
              cancellationSignal.then(error => ({ kind: "cancelled" as const, error })),
            ]);
            if (outcome.kind === "cancelled") {
              poisonedError = outcome.error;
              throw outcome.error;
            }
            if (outcome.kind === "error") {
              // Restore the permit before exposing the terminal upstream
              // error, so even a local catch cannot run outside the budget.
              await reacquire();
              throw outcome.error;
            }
            await reacquire();
            return outcome.value;
          } finally {
            yielding = false;
            if (inFlightYield === operation) inFlightYield = null;
          }
        })();
        inFlightYield = operation;
        // Observe rejection immediately even when task code discards the
        // returned promise. Every yielded-wait failure is terminal for this
        // invocation until run() settles, so a local catch cannot turn it into
        // success or replace it with a later contextual error.
        void operation.catch(error => {
          yieldFailed = true;
          yieldError = error;
        });
        return operation;
      },
    };

    try {
      let value: T | undefined;
      let taskFailed = false;
      let taskError: unknown;
      try {
        value = await task(permit);
      } catch (error) {
        taskFailed = true;
        taskError = error;
      }

      const unfinishedYield = inFlightYield;
      if (unfinishedYield !== null) {
        try {
          await unfinishedYield;
          if (!taskFailed) {
            taskFailed = true;
            taskError = new Error("concurrency task must await yieldWhile before completing");
          }
        } catch (error) {
          if (!taskFailed) {
            taskFailed = true;
            taskError = error;
          }
        }
      }
      if (poisonedError !== null) throw poisonedError;
      if (yieldFailed) throw yieldError;
      if (taskFailed) throw taskError;
      return value as T;
    } finally {
      if (held) {
        held = false;
        release();
      }
    }
  };

  return {
    run,
    stats: () => ({ active, queued: queue.length, peakActive }),
  };
}

function cancellationError(cancellation: CancellationToken): CancellationError {
  try {
    cancellation.throwIfCancelled();
  } catch (error) {
    if (error instanceof CancellationError) return error;
  }
  return new CancellationError(cancellation.reason ?? undefined);
}
