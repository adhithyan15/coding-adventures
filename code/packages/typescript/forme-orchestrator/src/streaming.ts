/**
 * Bounded single-source multicast for Forme stream edges.
 *
 * One upstream iterator is shared by a statically known set of consumers.
 * Every attached branch sees the same ordered prefix while a fixed per-branch
 * window prevents a fast consumer from growing a slow consumer's queue
 * without bound.
 */

import { CancellationError } from "@coding-adventures/forme-errors";
import type { CancellationToken } from "@coding-adventures/forme-stage";

export const DEFAULT_STREAM_WINDOW = 64;

export interface BoundedFanOutStats {
  readonly activeBranches: number;
  readonly upstreamPulls: number;
  readonly retainedValues: number;
  readonly peakRetainedValues: number;
}

export interface BoundedFanOut<T> {
  readonly branches: readonly AsyncIterable<T>[];
  readonly stats: () => BoundedFanOutStats;
}

interface PendingRead<T> {
  readonly resolve: (result: IteratorResult<T>) => void;
  readonly reject: (error: unknown) => void;
}

interface BranchState<T> {
  readonly queue: T[];
  active: boolean;
  iteratorTaken: boolean;
  terminalDelivered: boolean;
  cancellationPending: CancellationError | null;
  pending: PendingRead<T> | null;
}

type Terminal =
  | { readonly kind: "done" }
  | { readonly kind: "error"; readonly error: unknown };

/**
 * Split one async iterable into a fixed set of ordered, bounded branches.
 *
 * Branches are single-use. Calling `return()` detaches that branch and removes
 * it from backpressure. The upstream is opened only when a branch first asks
 * for a value.
 */
export function createBoundedFanOut<T>(
  source: AsyncIterable<T>,
  branchCount: number,
  cancellation: CancellationToken,
  windowSize = DEFAULT_STREAM_WINDOW,
): BoundedFanOut<T> {
  assertBound(branchCount, "branch count", true);
  assertBound(windowSize, "window size", false);

  const states: BranchState<T>[] = Array.from({ length: branchCount }, () => ({
    queue: [],
    active: true,
    iteratorTaken: false,
    terminalDelivered: false,
    cancellationPending: null,
    pending: null,
  }));
  let upstream: AsyncIterator<T> | null = null;
  let upstreamClosed = false;
  let terminal: Terminal | null = null;
  let cancelled = false;
  let pumpRunning = false;
  let upstreamPulls = 0;
  let peakRetainedValues = 0;

  const retainedValues = (): number =>
    states.reduce((total, state) => total + state.queue.length, 0);

  const activeBranches = (): number =>
    states.reduce((total, state) => total + (state.active ? 1 : 0), 0);

  const getUpstream = (): AsyncIterator<T> => {
    upstream ??= source[Symbol.asyncIterator]();
    return upstream;
  };

  const closeUpstream = async (): Promise<void> => {
    if (upstreamClosed) return;
    upstreamClosed = true;
    try {
      if (upstream === null) return;
      const close = upstream.return;
      if (typeof close === "function") await close.call(upstream);
    } catch {
      // Cleanup cannot replace the source failure or cancellation that caused
      // the close. Normal iterator completion does not call this path.
    }
  };

  const settleTerminalWaiters = (): void => {
    if (terminal === null) return;
    for (const state of states) {
      if (!state.active || state.pending === null || state.queue.length > 0) continue;
      const pending = state.pending;
      state.pending = null;
      if (terminal.kind === "error" && !state.terminalDelivered) {
        state.terminalDelivered = true;
        pending.reject(terminal.error);
      } else {
        pending.resolve({ done: true, value: undefined });
      }
    }
  };

  const finish = (nextTerminal: Terminal): void => {
    if (terminal !== null) return;
    terminal = nextTerminal;
    settleTerminalWaiters();
  };

  const cancel = (): void => {
    if (cancelled) return;
    cancelled = true;
    let error = new CancellationError(cancellation.reason ?? undefined);
    try {
      cancellation.throwIfCancelled();
    } catch (caught) {
      if (caught instanceof CancellationError) error = caught;
    }
    terminal = { kind: "error", error };
    for (const state of states) {
      if (!state.active) continue;
      state.queue.length = 0;
      state.active = false;
      if (state.pending !== null) {
        const pending = state.pending;
        state.pending = null;
        state.terminalDelivered = true;
        pending.reject(error);
      } else {
        // The next read observes cancellation once even though the branch is
        // already detached from pressure and resource accounting.
        state.cancellationPending = error;
      }
    }
    void closeUpstream();
  };

  cancellation.onCancel(cancel);

  const hasDemand = (): boolean =>
    states.some(state => state.active && state.pending !== null);

  const hasCapacity = (): boolean =>
    states.every(state => !state.active || state.pending !== null || state.queue.length < windowSize);

  const broadcast = (value: T): void => {
    for (const state of states) {
      if (!state.active) continue;
      if (state.pending !== null) {
        const pending = state.pending;
        state.pending = null;
        pending.resolve({ done: false, value });
      } else {
        state.queue.push(value);
      }
    }
    peakRetainedValues = Math.max(peakRetainedValues, retainedValues());
  };

  const pump = async (): Promise<void> => {
    if (pumpRunning) return;
    pumpRunning = true;
    try {
      while (terminal === null && activeBranches() > 0 && hasDemand() && hasCapacity()) {
        let done: boolean;
        let value: T | undefined;
        try {
          cancellation.throwIfCancelled();
          upstreamPulls += 1;
          const result: unknown = await getUpstream().next();
          if (cancelled || terminal !== null || activeBranches() === 0) break;
          if (typeof result !== "object" || result === null) {
            throw new TypeError("upstream iterator next() must return an object");
          }
          const candidate = result as IteratorResult<T>;
          done = Boolean(candidate.done);
          value = done ? undefined : candidate.value;
        } catch (error) {
          if (terminal === null) finish({ kind: "error", error });
          await closeUpstream();
          break;
        }

        // Cancellation or last-branch detachment may happen while next() is
        // in flight. Such a late value must never repopulate cleared queues.
        if (terminal !== null || activeBranches() === 0) break;
        if (done) {
          finish({ kind: "done" });
          break;
        }
        broadcast(value as T);
      }
    } catch (error) {
      if (terminal === null) finish({ kind: "error", error });
      await closeUpstream();
    } finally {
      pumpRunning = false;
    }
  };

  const schedulePump = (): void => { void pump(); };

  const next = async (state: BranchState<T>): Promise<IteratorResult<T>> => {
    if (state.cancellationPending !== null) {
      const error = state.cancellationPending;
      state.cancellationPending = null;
      throw error;
    }
    if (!state.active) return { done: true, value: undefined };
    cancellation.throwIfCancelled();
    if (state.queue.length > 0) {
      const value = state.queue.shift()!;
      schedulePump();
      return { done: false, value };
    }
    if (terminal !== null) {
      if (terminal.kind === "error" && !state.terminalDelivered) {
        state.terminalDelivered = true;
        throw terminal.error;
      }
      return { done: true, value: undefined };
    }
    if (state.pending !== null) {
      throw new Error("bounded fan-out branch permits only one unresolved next() call");
    }
    return new Promise<IteratorResult<T>>((resolve, reject) => {
      state.pending = { resolve, reject };
      schedulePump();
    });
  };

  const detach = async (state: BranchState<T>): Promise<IteratorResult<T>> => {
    if (state.active) {
      state.active = false;
      state.queue.length = 0;
      if (state.pending !== null) {
        const pending = state.pending;
        state.pending = null;
        pending.resolve({ done: true, value: undefined });
      }
      if (activeBranches() === 0) await closeUpstream();
      else schedulePump();
    }
    return { done: true, value: undefined };
  };

  const branches = states.map((state): AsyncIterable<T> => ({
    [Symbol.asyncIterator](): AsyncIterator<T> {
      if (state.iteratorTaken) {
        throw new Error("bounded fan-out branches are single-use");
      }
      state.iteratorTaken = true;
      return {
        next: () => next(state),
        return: () => detach(state),
        throw: async (error?: unknown) => {
          await detach(state);
          throw error;
        },
      };
    },
  }));

  return {
    branches,
    stats: () => ({
      activeBranches: activeBranches(),
      upstreamPulls,
      retainedValues: retainedValues(),
      peakRetainedValues,
    }),
  };
}

function assertBound(value: number, label: string, allowZero: boolean): void {
  if (!Number.isSafeInteger(value) || value < (allowZero ? 0 : 1)) {
    throw new Error(`${label} must be ${allowZero ? "a non-negative" : "a positive"} safe integer`);
  }
}
