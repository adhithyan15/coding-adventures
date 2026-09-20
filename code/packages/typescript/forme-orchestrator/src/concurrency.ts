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
      // Cancellation can fire synchronously through a custom token between
      // the initial closed check and insertion. Recheck before returning.
      if (closedError !== null) {
        const index = queue.findIndex(waiter => waiter.resolve === resolve);
        if (index >= 0) queue.splice(index, 1);
        reject(closedError);
      }
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
    const permit: ConcurrencyPermit = {
      async yieldWhile<R>(wait: () => Promise<R>): Promise<R> {
        if (yielding) {
          throw new Error("concurrency permit allows only one yield in flight");
        }
        if (!held) {
          throw new Error("yieldWhile requires an active permit");
        }
        yielding = true;
        held = false;
        release();
        try {
          const value = await wait();
          await acquire();
          held = true;
          return value;
        } finally {
          yielding = false;
        }
      },
    };

    try {
      return await task(permit);
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
