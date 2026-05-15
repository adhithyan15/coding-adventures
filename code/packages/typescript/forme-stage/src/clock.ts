/**
 * Clock — wall-clock and monotonic time, parameterised so reproducible
 * builds can freeze it (FM01 §4.3).
 *
 * Three methods on the contract:
 *
 *   - `nowMs()`     — UTC milliseconds since the Unix epoch.
 *   - `nowIso()`    — ISO-8601 / RFC 3339 string at the same instant.
 *   - `monotonicMs()` — strictly non-decreasing reference for measuring
 *                       elapsed time inside a stage.  Distinct from
 *                       `nowMs` because system clocks can jump (NTP
 *                       sync, leap seconds, time-zone changes during
 *                       suspend/resume) and stage timings need a stable
 *                       reference.
 *
 * === Two implementations ===
 *
 * 1. **systemClock()** — the production clock.  Wraps `Date.now()` for
 *    wall time and `performance.now()` for monotonic time.  Falls back
 *    to `Date.now()` for monotonic when `performance` isn't available
 *    (it's universal in Node 16+, browsers, Deno, Bun, and Workers, so
 *    the fallback is mostly defensive).
 *
 * 2. **frozenClock(timestamp)** — the reproducible-build clock.  Both
 *    `nowMs` and `nowIso` return the supplied timestamp every time.
 *    `monotonicMs` is its own counter starting at 0, advanced by
 *    successive calls — tests that measure elapsed time still work,
 *    they just don't observe the system clock at all.
 */

/** Clock contract, identical to FM01 §4.3. */
export interface Clock {
  nowMs(): number;
  nowIso(): string;
  monotonicMs(): number;
}

// ─── System clock ─────────────────────────────────────────────────────────

/** Build a Clock backed by the host's real time. */
export function systemClock(): Clock {
  const monotonic = monotonicSource();
  return {
    nowMs:        () => Date.now(),
    nowIso:       () => new Date().toISOString(),
    monotonicMs:  monotonic,
  };
}

function monotonicSource(): () => number {
  const perf = (globalThis as { performance?: { now?: () => number } }).performance;
  if (perf && typeof perf.now === "function") {
    return () => perf.now!();
  }
  // Fallback: not strictly monotonic across NTP jumps but better than
  // nothing.  Date.now reliably exists everywhere.
  return () => Date.now();
}

// ─── Frozen clock ─────────────────────────────────────────────────────────

export interface FrozenClockOptions {
  /** Wall-clock millisecond timestamp to return from `nowMs`. */
  readonly timestamp: number;
  /**
   * Initial value of the monotonic counter.  Default: 0.  Each call
   * to `monotonicMs` advances by `monotonicTickMs` (default: 0).
   */
  readonly monotonicStart?: number;
  readonly monotonicTickMs?: number;
}

/**
 * Build a Clock whose wall time is fixed to `timestamp`.  Used by the
 * orchestrator's reproducible-build mode (FM03 §8) to make stage outputs
 * byte-stable across runs.
 *
 * The monotonic source is independent of the wall clock — it advances
 * deterministically, by `monotonicTickMs` per call (default 0 = static).
 * Set a non-zero tick if your stage measures elapsed work and needs the
 * value to change between samples.
 */
export function frozenClock(options: FrozenClockOptions): Clock {
  const ts = options.timestamp;
  const tick = options.monotonicTickMs ?? 0;
  let mono = options.monotonicStart ?? 0;
  // Cache the ISO string — stringifying the same instant repeatedly
  // is wasteful when the orchestrator scopes child loggers per input.
  const iso = new Date(ts).toISOString();
  return {
    nowMs: () => ts,
    nowIso: () => iso,
    monotonicMs: () => {
      const value = mono;
      mono += tick;
      return value;
    },
  };
}
