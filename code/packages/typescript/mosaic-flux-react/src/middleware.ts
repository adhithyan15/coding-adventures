// middleware.ts — middleware contract for cross-cutting concerns.
//
// Middleware sees every dispatched (action, prevState, nextState)
// triple. Common uses:
//
//   - logger: log action + state diff for debugging
//   - persistence: serialize chosen slots to localStorage / disk
//   - analytics: report user-significant actions to telemetry
//   - validation: assert invariants on next state; throw in dev mode
//   - effects: when action X completes, trigger side effect Y via
//     extension points or external dispatch
//
// Middleware runs AFTER the reducer (after `action.apply(state)`
// produces the next state). This is the simplest place — middleware
// observes results, not requests, and never blocks dispatch.
//
// Async effects in middleware schedule additional dispatches; they
// do not extend the synchronous round trip. The action that fires
// the effect lands first; the effect's resulting action lands later.

import type { MosaicAction } from "./action.js";

export type Middleware<State> = (
  action: MosaicAction<State>,
  prevState: State,
  nextState: State,
) => void;

/**
 * Compose middleware into a single function. Each middleware runs
 * in registration order. If one throws, subsequent middleware still
 * run — the runtime catches and logs the error so a single bad
 * middleware can't take down the whole store.
 */
export function composeMiddleware<State>(
  middleware: ReadonlyArray<Middleware<State>>,
): Middleware<State> {
  if (middleware.length === 0) {
    return () => {
      /* no-op when nothing's registered */
    };
  }
  if (middleware.length === 1) {
    return middleware[0]!;
  }
  return (action, prevState, nextState) => {
    for (const m of middleware) {
      try {
        m(action, prevState, nextState);
      } catch (err) {
        // Per the architecture (UI33-rewrite §6.4): one bad middleware
        // does not break others. We log to console (no external
        // dependency) and continue.
        // eslint-disable-next-line no-console
        console.error("[mosaic-flux] middleware threw:", err);
      }
    }
  };
}

/**
 * A logger middleware for dev builds. Logs action class name,
 * timestamp, and a shallow state diff.
 *
 * Production code typically composes its own logger that ships to
 * a telemetry backend rather than console.
 */
export function loggerMiddleware<State>(): Middleware<State> {
  return (action, prevState, nextState) => {
    const name = action.constructor.name;
    const changedKeys = shallowChangedKeys(prevState, nextState);
    // eslint-disable-next-line no-console
    console.groupCollapsed(
      `[mosaic-flux] ${name} — changed: ${changedKeys.length > 0 ? changedKeys.join(", ") : "(none)"}`,
    );
    // eslint-disable-next-line no-console
    console.log("action  :", action);
    // eslint-disable-next-line no-console
    console.log("prev    :", prevState);
    // eslint-disable-next-line no-console
    console.log("next    :", nextState);
    // eslint-disable-next-line no-console
    console.groupEnd();
  };
}

function shallowChangedKeys<State>(
  prev: State,
  next: State,
): ReadonlyArray<string> {
  if (
    typeof prev !== "object" ||
    prev === null ||
    typeof next !== "object" ||
    next === null
  ) {
    // Non-object states change atomically.
    return prev !== next ? ["<root>"] : [];
  }
  const changed: string[] = [];
  const prevObj = prev as Record<string, unknown>;
  const nextObj = next as Record<string, unknown>;
  const allKeys = new Set([...Object.keys(prevObj), ...Object.keys(nextObj)]);
  for (const k of allKeys) {
    if (prevObj[k] !== nextObj[k]) changed.push(k);
  }
  return changed;
}
