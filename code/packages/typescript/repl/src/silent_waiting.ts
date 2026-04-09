/**
 * SilentWaiting — a no-op Waiting implementation.
 *
 * The `Waiting` interface was designed for richly animated terminals that can
 * display spinners, progress bars, or elapsed-time counters. Not every host
 * environment supports or wants that:
 *
 * - Unit tests running in a headless Node.js process should not emit ANSI
 *   escape codes or print to stdout at all.
 * - CI pipelines typically capture raw stdout and display it verbatim;
 *   spinner characters look like garbage in log files.
 * - Embedded or piped usage (e.g., `echo "1+1" | repl`) has no concept of
 *   "waiting" because the input is pre-buffered.
 *
 * `SilentWaiting` satisfies the `Waiting` interface with completely inert
 * implementations — `start` returns an empty object, `tick` passes state
 * through unchanged, and `stop` does nothing. The interval still fires (the
 * loop doesn't know it's silent), but the callbacks are cheap no-ops.
 *
 * This is the **Null Object pattern**: instead of checking `if (waiting)`
 * everywhere in the loop, the loop always calls the interface, and a null
 * object silently swallows the calls.
 */

import type { Waiting } from "./types.js";

/**
 * A Waiting implementation that shows no animation and performs no I/O.
 *
 * Useful as a default for tests, pipes, and non-interactive environments.
 *
 * @example
 * const waiting = new SilentWaiting();
 * const state = waiting.start();    // {}
 * const next  = waiting.tick(state); // state unchanged
 * waiting.stop(next);               // no-op
 * waiting.tickMs();                 // 100
 */
export class SilentWaiting implements Waiting {
  /**
   * Returns an empty object as the initial animation state.
   *
   * The opaque `unknown` type in the interface means callers cannot inspect
   * this object — they just pass it through to `tick` and `stop`. Using `{}`
   * rather than `null` avoids accidental null-dereference if a caller ever
   * tries to read the state (it's a defensive choice, not a requirement).
   */
  start(): unknown {
    return {};
  }

  /**
   * Returns the state unchanged.
   *
   * There is no animation to advance, so the state machine simply stays where
   * it is. The loop will keep calling `tick` every `tickMs` milliseconds, but
   * each call is effectively free — just an object identity pass-through.
   */
  tick(state: unknown): unknown {
    return state;
  }

  /**
   * How often (in ms) the loop should call `tick`.
   *
   * 100 ms is a reasonable default: it's imperceptibly fast for a spinner but
   * infrequent enough to avoid meaningful CPU overhead during long evaluations.
   */
  tickMs(): number {
    return 100;
  }

  /**
   * Does nothing.
   *
   * A real Waiting implementation would use `stop` to erase the spinner from
   * the terminal (e.g., write `\r` followed by spaces). SilentWaiting has
   * nothing to erase, so `stop` is a genuine no-op.
   */
  stop(_state: unknown): void {
    // Intentionally empty — no animation to clean up.
  }
}
